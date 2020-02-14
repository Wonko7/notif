extern crate signal_hook;
use signal_hook::{iterator::Signals, SIGHUP};
extern crate clap;
use clap::{Arg, AppSettings, App};

mod config;
use config::Config;


pub fn run_sender(config: Config, hostname: &str, summary: &str, body: &str, priority: &str) -> Result<(), failure::Error> {
    // I don't understand why args^ doesn't need to be mut when we use it as mut in from_argv.
    let context    = zmq::Context::new();
    let send_notif = context.socket(zmq::REQ)?;
    send_notif.connect(format!("tcp://{}:{}", config.server_ip, config.incoming_notif_port).as_str())?;
//    let notif      = Notification::from_argv(args)?;

    let msg: [&[u8]; 4] = [
        hostname.as_bytes(),
        priority.as_bytes(),
        summary.as_bytes(),
        body.as_bytes(),
    ];

    send_notif.send_multipart(&msg, 0)?;
    send_notif.recv_string(0)?.unwrap(); // wait for ack.
    // TODO: see if --debug or something:
    // let ack = send_notif.recv_string(0)?;
    // println!("sent and got ack: {}", ack.unwrap()); // if ack isn't utf8 well panic.

    Ok(())
}

pub fn run_server(config: Config) -> Result<(), failure::Error> {
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

pub fn run_notifier(config: Config, hostname: &str, id: String) -> Result<(), failure::Error> {
    let context        = zmq::Context::new();
    let seize_notifier = context.socket(zmq::REQ)?; // TODO: signal USR1 to takeover again
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

            let notif_get      = |i: usize| String::from_utf8(notif[i].clone()); // couldn't get around the clone.
            let notif_hostname = notif_get(1)?;
            let body           = notif_get(4)?;
            let title          = if notif_hostname != hostname {
                format!("@{}: {}", notif_hostname, notif_get(3)?)
            } else {
                notif_get(3)?
            };
            // println!("env 0{}, host 1{}, prio 2{}, title 3{}, body 4{}", notif_get(0)?, notif_get(1)?, notif_get(2)?, notif_get(3)?, notif_get(4)?);

            std::process::Command::new("/usr/bin/notify-send")
                .arg("--")
                .arg(title)
                .arg(body)
                .spawn()?
                .wait()?;
        }
    }
}

fn get_v<'a>(opts: &'a clap::ArgMatches, name: &str) -> &'a str {
    // couldn't put a lifetime on a closure, this is just a helper to get values out of matches
    opts.value_of(name).unwrap()
}

pub fn run() -> Result<(), failure::Error> {
    let verbose  = false;
    let hostname = hostname::get().unwrap().into_string().unwrap();

    let matches = App::new("notif")
        .version("0.0.3") // hmm.
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("william@undefined.re")
        .about("routes remote notifications to you")
        .arg(Arg::with_name("config") // TODO: does not work, but should be fine: https://github.com/clap-rs/clap/issues/1570
            .short('c')
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        .subcommand(App::new("send")
            .about("send a notification")
            .arg(Arg::with_name("SUMMARY")
                .help("notification summary")
                .required(true)
                .index(1))
            .arg(Arg::with_name("BODY")
                .help("notification body")
                .required(true)
                .index(2))
            .arg(Arg::with_name("urgency")
                .short('u')
                .default_value("normal")
                .possible_values(&["low", "normal", "urgent"])
                .help("urgency")))
        .subcommand(App::new("notify")
            .about("show notifications")
            .arg(Arg::with_name("ID")
                .help("notifier ID")
                .required(true)
                .index(1)))
        .subcommand(App::new("route")
            .about("receive and forward notifications"))
        .get_matches();

    // TODO add --verbose args work again.
    let config_file = matches.value_of("config");
    let config = Config::new(config_file)?;

    match matches.subcommand() {
        ("send", Some(ms))   => run_sender(
            config,
            hostname.as_str(),
            get_v(ms, "SUMMARY"),
            get_v(ms, "BODY"),
            get_v(ms, "urgency"),
        ),
        ("notify", Some(ms)) => run_notifier(
            config,
            hostname.as_str(),
            get_v(ms, "ID").to_string() // because the thread takes ownership.
            ),
        ("route", _)         => run_server(config),
        _                    => Err(failure::err_msg("unreachable"))
    }
}
