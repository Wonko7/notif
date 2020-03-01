use failure::{Error, err_msg, format_err};
use libzmq::{prelude::*, poll::*, auth::{CurveServerCreds, CurveClientCreds}, Heartbeat, Period, ServerBuilder, ClientBuilder};
use signal_hook::{iterator::Signals, SIGHUP, SIGUSR1, SIGUSR2};
use std::{collections::VecDeque, time::Duration};

use crate::config::Config;
use crate::notif::Notification;

// FIXME: needs empirical data
static TIMEOUT: Duration = Duration::from_secs(5);
static QUEUE_SIZE: usize = 3;

pub fn send(config: Config, notif: Notification) -> Result<(), Error> {
    let client_creds   = CurveClientCreds::new(config.as_client.server.public)
        .add_cert(config.as_client.cert);
    let outgoing_notif = ClientBuilder::new()
        .connect(&config.as_client.server.incoming)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .mechanism(client_creds)
        .build()?;

    if let Some(true) = config.verbose {
        println!("sending to: {}", &config.as_client.server.incoming);
    }

    outgoing_notif.send(bincode::serialize(&notif).unwrap())?;
    let status = outgoing_notif.recv_msg()?; // wait for ack.
    let msg    = status.to_str()?;

    if let Some(true) = config.verbose {
        println!("received: {}", msg);
    }
    Ok(())
}

pub fn route(config: Config) -> Result<(), Error> {
    if let None = config.as_server {
        return Err(err_msg("missing as_server section in config"));
    }

    let srv_config     = config.as_server.unwrap();
    let _auth_registry = srv_config.auth.build()?;
    let server_creds   = CurveServerCreds::new(srv_config.secret);
    let heartbeat      = Heartbeat::new(TIMEOUT)
        .add_timeout(2 * TIMEOUT);
    let incoming_notif = ServerBuilder::new()
        .bind(&srv_config.incoming)
        .mechanism(&server_creds)
        .heartbeat(&heartbeat)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .build()?;
    let outgoing_notif = ServerBuilder::new()
        .bind(&srv_config.outgoing)
        .mechanism(&server_creds)
        .heartbeat(&heartbeat)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .build()?;

    let mut current_notifier_id = None;
    let mut queue               = VecDeque::new();
    let queue_size              = srv_config.queue_size.unwrap_or(QUEUE_SIZE);

    if let Some(true) = config.verbose {
        println!("listening for incoming notifications on: {}", &srv_config.incoming);
        println!("forwarding to notifiers on: {}", &srv_config.outgoing);
    }

    let mut poller = Poller::new();
    let mut events = Events::new();
    poller.add(&outgoing_notif, PollId(0), READABLE)?;
    poller.add(&incoming_notif, PollId(1), READABLE)?;

    loop {
        poller.poll(&mut events, Period::Infinite)?;
        for event in &events {
            match event.id() {
                PollId(0) => { // control message from notifier: SEIZE or YIELD
                    let notifier_req     = outgoing_notif.recv_msg()?;
                    let notifier_req_str = notifier_req.to_str()?;
                    let id               = notifier_req.routing_id().unwrap();

                    if notifier_req_str == "SEIZE" {
                        current_notifier_id = Some(id);
                        for msg in &queue { // queue.iter().map(|msg| outgoing_notif.route(msg, id)).collect();
                            outgoing_notif.route(msg, id)?;
                        }
                        queue.clear();
                    } else { // YIELD: queue messages in the meantime.
                        current_notifier_id = None;
                    }
                    outgoing_notif.route("ACK", id)?;

                    if let Some(true) = config.verbose {
                        println!("routing id {} request: {}", id.0, notifier_req_str);
                    }
                },
                PollId(1) => { // notification message to forward to notifier:
                    let notif_fwd = incoming_notif.recv_msg()?;
                    let sender_id = notif_fwd.routing_id().unwrap();

                    if let Some(notifier_id) = current_notifier_id {
                        if let Ok(_) = outgoing_notif.route(notif_fwd, notifier_id) {
                            incoming_notif.route("ACK", sender_id)?;
                        } else { // current_notifier might have fucked off.
                            // FIXME also queue this.
                            current_notifier_id = None;
                            incoming_notif.route("DROP", sender_id)?;
                        }
                        if let Some(true) = config.verbose {
                            println!("Forward message to routing id {:?}", notifier_id);
                        }
                    } else { // queue!
                        if let Some(true) = config.verbose {
                            println!("no notifier: queueing");
                        }
                        if queue.len() == queue_size {
                            queue.pop_front();
                            if let Some(true) = config.verbose {
                                println!("dropping oldest");
                            }
                        }
                        queue.push_back(notif_fwd);
                        incoming_notif.route("QUEUED", sender_id)?;
                    }
                },
                _ => unreachable!(),
            }
        }
    }
}

pub fn notify(config: Config, hostname: &str) -> Result<(), Error> {
    let client_creds   = CurveClientCreds::new(config.as_client.server.public)
        .add_cert(config.as_client.cert);
    let incoming_notif = ClientBuilder::new()
        .connect(config.as_client.server.outgoing)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .mechanism(client_creds)
        .build()?;

    incoming_notif.send("SEIZE")?;
    if let Some(true) = config.verbose {
        println!("sent SEIZE: {}", &config.as_client.server.incoming);
    }

    // catch SIGHUP to seize notifier. (use on unlock xscreensaver, etc):
    let (tx, interrupt_rx) = std::sync::mpsc::channel();
    let signals            = Signals::new(&[SIGHUP, SIGUSR1, SIGUSR2])?;
    std::thread::spawn(move || {
        for signal in signals.forever() {
	    match signal {
		SIGUSR1          => tx.send("YIELD").unwrap(),
		SIGHUP | SIGUSR2 => tx.send("SEIZE").unwrap(),
		_                => unreachable!() // since other signals aren't registered.
	    }
        }
    });

    let mut poller = Poller::new();
    let mut events = Events::new();
    poller.add(&incoming_notif, PollId(0), READABLE)?;

    loop {
        if let Ok(_) = poller.poll(&mut events, Period::Infinite) {
            for _event in &events {
                let msg = incoming_notif.recv_msg()?;
                if msg.len() == 3 && msg.to_str()? == "ACK" {
                    continue;
                }
                let notif: Notification = bincode::deserialize(&msg.as_bytes())?;

                let keep_in_scope: String;
                let summary = if notif.hostname != hostname {
                    keep_in_scope = format!("@{}: {}", notif.hostname, notif.summary);
                    keep_in_scope.as_str()
                } else {
                    notif.summary
                };

                if let Some(true) = config.verbose {
                    println!("notifying: {} {}", summary, notif.body);
                }

                std::process::Command::new("/usr/bin/notify-send")
                    .args(&["-u", notif.urgency, "--", summary, notif.body])
                    .spawn()?
                    .wait()?;
            }
        } else if let Ok(interrupt_message) = interrupt_rx.recv() {
            incoming_notif.send(interrupt_message)?;
            if let Some(true) = config.verbose {
                println!("sent {}: {}", interrupt_message, &config.as_client.server.incoming);
            }
        }
    }
}
