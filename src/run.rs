use std::sync::Arc;

use crate::config::Config;
use crate::notif::Notification;

use serde::{Serialize, Deserialize};
use libzmq::{prelude::*, poll::*, *};
use signal_hook::{iterator::Signals, SIGHUP};


pub fn send(config: Config, hostname: &str, summary: &str, body: &str, urgency: &str) -> Result<(), failure::Error> {
    let _          = config.auth.build()?;
    let send_notif = config.sender.build()?; // better config FIXME

    let msg = Notification {
        hostname,
        urgency,
        summary,
        body,
    };

    send_notif.send(bincode::serialize(&msg).unwrap())?;
    send_notif.recv_msg()?; // wait for ack.


    // TODO: add --verbose for this;
    // let ack = send_notif.recv_string(0)?;
    // println!("sent and got ack: {}", ack.unwrap()); // if ack isn't utf8 well panic.

    Ok(())
}

pub fn route(config: Config) -> Result<(), failure::Error> {
    println!("route");

    let mut current_notifier_id = None;
    let _              = config.auth.build()?;
    let incoming_notif = config.router_in.build()?; // better config FIXME
    let outgoing_notif = config.router_out.build()?;
    let mut poller     = Poller::new();
    let mut events     = Events::new();

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
                    println!("routing id {:?} seized by: {}", current_notifier_id, seize_req.to_str()?);
                    outgoing_notif.route("ACK", current_notifier_id.unwrap())?;
                }
                PollId(1) => { // Forward to notifier:
                    let notif_fwd = incoming_notif.recv_msg()?;
                    let sender_id = notif_fwd.routing_id().unwrap();
                    if let Some(current_notifier_id) = current_notifier_id {
                        println!("to {:?} forwarding: {}", current_notifier_id, notif_fwd.to_str()?);
                        outgoing_notif.route(notif_fwd, current_notifier_id)?;
                        incoming_notif.route("ACK", sender_id)?;
                    } else {
                        incoming_notif.route("DROP", sender_id)?;
                    }
                }
                _         => unreachable!(),
            }
        }
    }
}

pub fn notify(config: Config, hostname: &str, id: String) -> Result<(), failure::Error> {
    println!("notify");
    let _                = config.auth.build()?;
    let incoming_notif   = config.notifier.build()?; // connect.
    let interrupted      = Arc::new(false);
    let mut interrupted2 = Arc::clone(&interrupted);

    incoming_notif.send("seize!")?;

    // catch SIGHUP to seize notifier! (use on unlock xscreensaver, etc)
    let signals        = Signals::new(&[SIGHUP])?;
    std::thread::spawn(move || {
        for _signal in signals.forever() {
            println!("seizing!");
            *Arc::get_mut(&mut interrupted2).unwrap() = true;
            // set global to one, check it in else
        }
    });

    let mut poller     = Poller::new();
    let mut events     = Events::new();

    poller.add(&incoming_notif, PollId(0), READABLE)?;

    loop {
        println!("waiting");
        if let Ok(_) = poller.poll(&mut events, Period::Infinite) {
            for event in &events {
                // event.is_readable()), .id = PollId(0), unwrap a result?
                if event.is_readable() {
                    let msg = incoming_notif.recv_msg()?;
                    println!("len: {}", msg.len());

                    if msg.len() == 3 && msg.to_str()? == "ACK" {
                        println!("ignore ack");
                        continue;
                    }
                    let notif: Notification = bincode::deserialize(&msg.as_bytes())?;
                    println!("so easy {:?}", notif);

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
                } else if *interrupted {
                    println!("else... int");
                    // check interrupt global here.
                }
            }
        } else {
            println!("int");
        }
    }
}
