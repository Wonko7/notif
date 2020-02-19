use crate::config::Config;
use crate::notif::Notification;

use serde::{Serialize, Deserialize};
use libzmq::{prelude::*, poll::*, *};
use signal_hook::{iterator::Signals, SIGHUP};


pub fn send(config: Config, notif: Notification) -> Result<(), failure::Error> {
    let _          = config.auth.build()?;
    let send_notif = config.sender.build()?; // better config FIXME

    send_notif.send(bincode::serialize(&notif).unwrap())?;
    send_notif.recv_msg()?; // wait for ack.

    Ok(())
}

pub fn route(config: Config) -> Result<(), failure::Error> {
    let mut current_notifier_id = None;
    let _                       = config.auth.build()?;
    let incoming_notif          = config.router_in.build()?; // better config FIXME
    let outgoing_notif          = config.router_out.build()?;
    let mut poller              = Poller::new();
    let mut events              = Events::new();

    poller.add(&outgoing_notif, PollId(0), READABLE)?;
    poller.add(&incoming_notif, PollId(1), READABLE)?;

    loop {
        println!("waiting");
        poller.poll(&mut events, Period::Infinite)?;

        for event in &events {
            match event.id() {
                PollId(0) => { // SEIZE from notifier:
                    let seize_req        = outgoing_notif.recv_msg()?; // we need prelude::*, why?
                    current_notifier_id  = Some(seize_req.routing_id().unwrap());
                    outgoing_notif.route("ACK", current_notifier_id.unwrap())?;
                    // if verbose:
                    println!("routing id {:?} seized by: {}", current_notifier_id, seize_req.to_str()?);
                }
                PollId(1) => { // Forward to notifier:
                    let notif_fwd = incoming_notif.recv_msg()?;
                    let sender_id = notif_fwd.routing_id().unwrap();

                    if let Some(current_notifier_id) = current_notifier_id {
                        outgoing_notif.route(notif_fwd, current_notifier_id)
                            .unwrap_or_else(|_e| incoming_notif.route("DROP", sender_id).unwrap()); // current_notifier might have fucked off.
                        incoming_notif.route("ACK", sender_id)?;
                    } else {
                        incoming_notif.route("DROP", sender_id)?;
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

pub fn notify(config: Config, hostname: &str) -> Result<(), failure::Error> {
    let _              = config.auth.build()?;
    let incoming_notif = config.notifier.build()?; // connect.
    incoming_notif.send("seize!")?;

    // catch SIGHUP to seize notifier. (use on unlock xscreensaver, etc):
    let (tx, from_int_rx) = std::sync::mpsc::channel();
    let signals           = Signals::new(&[SIGHUP])?;
    std::thread::spawn(move || {
        for _signal in signals.forever() {
            tx.send(true).unwrap();
        }
    });

    let mut poller     = Poller::new();
    let mut events     = Events::new();
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
        } else if let Ok(true) = from_int_rx.recv() {
            // if verbose:
            println!("seizing from poll!");
            incoming_notif.send("seize!")?;
        }
    }
}
