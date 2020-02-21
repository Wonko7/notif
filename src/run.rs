use failure::{Error, err_msg, format_err};
use libzmq::{prelude::*, poll::*, auth::{CurveServerCreds, CurveClientCreds}, Heartbeat, Period, ServerBuilder, ClientBuilder};
use signal_hook::{iterator::Signals, SIGHUP};
use std::time::Duration;

use crate::config::Config;
use crate::notif::Notification;

static TIMEOUT: Duration = Duration::from_secs(60);

pub fn send(config: Config, notif: Notification) -> Result<(), Error> {
    let client_creds = CurveClientCreds::new(config.as_client.server.public)
        .add_cert(config.as_client.cert);
    let send_notif   = ClientBuilder::new()
        .connect(&config.as_client.server.incoming)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .mechanism(client_creds)
        .build()?;

    if let Some(true) = config.verbose {
        println!("connect: {}", &config.as_client.server.incoming);
    }

    send_notif.send(bincode::serialize(&notif).unwrap())?;
    let status = send_notif.recv_msg()?; // wait for ack.
    let msg    = status.to_str()?;
    if msg == "ACK" {
        Ok(())
    } else {
        Err(format_err!("received: {}", msg))
    }
}

pub fn route(config: Config) -> Result<(), Error> {
    if let None = config.as_server {
        return Err(err_msg("missing as_server section in config"));
    }
    let mut current_notifier_id = None;

    let srv_config     = config.as_server.unwrap();
    let _auth_registry = srv_config.auth.build()?;
    let server_creds   = CurveServerCreds::new(srv_config.secret);
    let heartbeat      = Heartbeat::new(TIMEOUT)
        .add_timeout(2 * TIMEOUT);
    let incoming_notif = ServerBuilder::new()
        .bind(srv_config.incoming)
        .mechanism(&server_creds)
        .heartbeat(&heartbeat)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .build()?;
    let outgoing_notif = ServerBuilder::new()
        .bind(srv_config.outgoing)
        .mechanism(&server_creds)
        .heartbeat(&heartbeat)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .build()?;

    let mut poller = Poller::new();
    let mut events = Events::new();
    poller.add(&outgoing_notif, PollId(0), READABLE)?;
    poller.add(&incoming_notif, PollId(1), READABLE)?;

    loop {
        poller.poll(&mut events, Period::Infinite)?;
        for event in &events {
            match event.id() {
                PollId(0) => { // SEIZE from notifier:
                    let seize_req       = outgoing_notif.recv_msg()?; // we need prelude::*, why?
                    current_notifier_id = Some(seize_req.routing_id().unwrap());
                    outgoing_notif.route("ACK", current_notifier_id.unwrap())?;
                    if let Some(true) = config.verbose {
                        println!("routing id {} seized with msg: {}", current_notifier_id.unwrap().0, seize_req.to_str()?);
                    }
                },
                PollId(1) => { // Forward to notifier:
                    let notif_fwd = incoming_notif.recv_msg()?;
                    let sender_id = notif_fwd.routing_id().unwrap();

                    if let Some(current_notifier_id) = current_notifier_id {
                        outgoing_notif.route(notif_fwd, current_notifier_id)
                            .unwrap_or_else(|_e| incoming_notif.route("DROP", sender_id).unwrap()); // current_notifier might have fucked off.
                        incoming_notif.route("ACK", sender_id)?;
                        if let Some(true) = config.verbose {
                            println!("Forward message to routing id {:?}", current_notifier_id);
                        }
                    } else {
                        incoming_notif.route("DROP", sender_id)?;
                        if let Some(true) = config.verbose {
                            println!("dropping, no notifier");
                        }
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

    // catch SIGHUP to seize notifier. (use on unlock xscreensaver, etc):
    let (tx, interrupt_rx) = std::sync::mpsc::channel();
    let signals            = Signals::new(&[SIGHUP])?;
    std::thread::spawn(move || {
        for _signal in signals.forever() {
            tx.send(true).unwrap();
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

                std::process::Command::new("/usr/bin/notify-send")
                    .args(&["-u", notif.urgency, "--", summary, notif.body])
                    .spawn()?
                    .wait()?;
                }
        } else if let Ok(true) = interrupt_rx.recv() {
            incoming_notif.send("SEIZE")?;
            if let Some(true) = config.verbose {
                println!("seizing!");
            }
        }
    }
}
