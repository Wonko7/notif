use crate::config;

extern crate signal_hook;
use signal_hook::{iterator::Signals, SIGHUP};

use config::Config;

pub fn send(config: Config, hostname: &str, summary: &str, body: &str, priority: &str) -> Result<(), failure::Error> {
    let context    = zmq::Context::new();
    let send_notif = context.socket(zmq::REQ)?;
    send_notif.connect(format!("tcp://{}:{}", config.server_ip, config.incoming_notif_port).as_str())?;

    let msg: [&[u8]; 4] = [
        hostname.as_bytes(),
        priority.as_bytes(),
        summary.as_bytes(),
        body.as_bytes(),
    ];

    send_notif.send_multipart(&msg, 0)?;
    send_notif.recv_string(0)?.unwrap(); // wait for ack.
    // TODO: add --verbose for this;
    // let ack = send_notif.recv_string(0)?;
    // println!("sent and got ack: {}", ack.unwrap()); // if ack isn't utf8 well panic.

    Ok(())
}

pub fn serve(config: Config) -> Result<(), failure::Error> {
    let context         = zmq::Context::new();
    let mut notifier_id = String::from("kekette"); // default for now...
    let bind            = |s: &zmq::Socket, port| s.bind(format!("tcp://{}:{}", config.server_ip, port).as_str());

    let incoming_notif = context.socket(zmq::REP)?;
    let notifier_seize = context.socket(zmq::REP)?;
    let outgoing_notif = context.socket(zmq::PUB)?;

    bind(&incoming_notif, &config.incoming_notif_port)?;
    bind(&notifier_seize, &config.notifier_seize_port)?;
    bind(&outgoing_notif, &config.outgoing_notif_port)?;

    println!("Notification server listening");
    loop {
        let mut items = [
            incoming_notif.as_poll_item(zmq::POLLIN),
            notifier_seize.as_poll_item(zmq::POLLIN),
        ];
        zmq::poll(&mut items, -1)?;

        if items[0].is_readable() {
            let notif_parts = incoming_notif.recv_multipart(0)?;
            if notif_parts.len() != 4 {
                println!("Dropping message with {} parts", notif_parts.len());
                continue;
            }
            // could also use notif_parts.insert(notifier_id.clone(), 0), seemed uglier.
            let routable_notif: [&[u8]; 5] = [
                &notifier_id.as_str().as_bytes(), // PUB id env
                &notif_parts[0],
                &notif_parts[1],
                &notif_parts[2],
                &notif_parts[3],
            ];
            outgoing_notif.send_multipart(&routable_notif, 0)?;
            incoming_notif.send("ack", 0)?;
        }

        if items[1].is_readable() {
            if let Ok(id) = notifier_seize.recv_string(0)? {
                println!("setting notifier subscribe to: {}", id);
                // TODO: server will make a unique ID per client. better yet, use zmq for that.
                // => dealer/router has what we want.
                // yield to the new notifier:
                notifier_id = id.clone();
                notifier_seize.send("ack", 0)?;
            };
        }
    }
}

pub fn notify(config: Config, hostname: &str, id: String) -> Result<(), failure::Error> {
    let context        = zmq::Context::new();
    let seize_notifier = context.socket(zmq::REQ)?;
    let incoming_notif = context.socket(zmq::SUB)?;

    seize_notifier.connect(format!("tcp://{}:{}", config.server_ip, config.notifier_seize_port).as_str())?;
    seize_notifier.send(id.as_str(), 0)?;
    incoming_notif.connect(format!("tcp://{}:{}", config.server_ip, config.outgoing_notif_port).as_str())?;
    incoming_notif.set_subscribe(id.as_bytes())?;
    seize_notifier.recv_string(0)?.unwrap(); // wait for ack.

    // catch SIGHUP to seize notifier! (use on unlock xscreensaver, etc)
    let signals        = Signals::new(&[SIGHUP])?;
    std::thread::spawn(move || {
        for _signal in signals.forever() {
            seize_notifier.send(id.as_str(), 0).expect("could not send seize_notifier");
            seize_notifier.recv_string(0).expect("could not recv seize_notifier").unwrap();
            println!("Seized notifier as {}", id.as_str());
        }
    });

    loop {
        if let Ok(notif) = incoming_notif.recv_multipart(0) { // ignore Err, interrupts for example.
            if notif.len() != 5 {
                println!("Dropping message with {} parts", notif.len());
                continue;
            }

            // put this in a fn and ignore Err, we shouldn't return and exit because of bad utf8.
            let keep_in_scope: String;
            let notif_get      = |i: usize| std::str::from_utf8(&notif[i]);
            let notif_hostname = notif_get(1)?;
            let priority       = notif_get(2)?;
            let body           = notif_get(4)?;
            let title          = if notif_hostname != hostname {
                keep_in_scope = format!("@{}: {}", notif_hostname, notif_get(3)?);
                keep_in_scope.as_str()
            } else {
                notif_get(3)?
            };

            std::process::Command::new("/usr/bin/notify-send")
                .args(&["-u", priority, "--", title, body])
                .spawn()?
                .wait()?;
        }
    }
}
