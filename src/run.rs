use failure::{bail, Error, err_msg, format_err};
use libzmq::{ErrorKind, prelude::*, poll::*, auth::{CurveServerCreds, CurveClientCreds}, Heartbeat, Period, ServerBuilder, ClientBuilder};
use signal_hook::{iterator::Signals, SIGHUP, SIGUSR1, SIGUSR2};
use std::{collections::VecDeque, time::Duration};

use crate::config::Config;
use crate::notif::{Notification, RequestType::*};

// FIXME: needs empirical data
static TIMEOUT: Duration = Duration::from_secs(5);
static QUEUE_SIZE: usize = 1000;

/// Sender endpoint: send a Notification message, wait a bit for an ACK.
pub fn send(config: Config, notif: Notification) -> Result<(), Error> {
    let verbose        = config.verbose.unwrap_or(false);
    let client_creds   = CurveClientCreds::new(config.as_client.server.public)
        .add_cert(config.as_client.cert);
    let outgoing_notif = ClientBuilder::new()
        .connect(&config.as_client.server.incoming)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .mechanism(client_creds)
        .build()?;

    outgoing_notif.send(bincode::serialize(&notif).unwrap())?;
    if verbose {
        println!("sent notification to: {}", &config.as_client.server.incoming);
    }

    if let Ok(status) = outgoing_notif.recv_msg() {
        if verbose {
            println!("received: {}", status.to_str()?);
        }
        Ok(())
    } else {
        spawn_local_notif(&notif)?;
        Err(format_err!("timeout, no response from {}", &config.as_client.server.incoming))
    }
}

/// Notifier endpoint: receives Notification messages to be displayed.  Has the lifetime of an X
/// session, can be made to yield or seize current notifier status with SIGUSR1 & SIGUSR2. Yield
/// when X is locked, sieze when unlocked.
pub fn notify(config: Config, hostname: &str) -> Result<(), Error> {
    let verbose        = config.verbose.unwrap_or(false);
    let client_creds   = CurveClientCreds::new(config.as_client.server.public)
        .add_cert(config.as_client.cert);
    let incoming_notif = ClientBuilder::new()
        .connect(config.as_client.server.outgoing)
        .recv_timeout(TIMEOUT)
        .send_timeout(TIMEOUT)
        .mechanism(client_creds)
        .build()?;

    incoming_notif.send("SEIZE")?;
    if verbose {
        println!("sent SEIZE: {}", &config.as_client.server.incoming);
    }

    // catch signals to seize/yield current notifier. (use on unlock xscreensaver, etc):
    let (tx, interrupt_rx) = std::sync::mpsc::channel();
    let signals            = Signals::new(&[SIGHUP, SIGUSR1, SIGUSR2])?;
    std::thread::spawn(move || {
        for signal in signals.forever() {
            match signal {
                SIGUSR1          => tx.send(Yield).unwrap(),
                SIGHUP | SIGUSR2 => tx.send(Seize).unwrap(),
                _                => unreachable!() // since other signals aren't registered.
            }
        }
    });

    // loop state:
    let mut wait_for_ack   = true;
    let mut replay_request = Seize;

    let mut poller = Poller::new();
    let mut events = Events::new();
    poller.add(&incoming_notif, PollId(0), READABLE)?;

    loop {
        match poller.poll(&mut events, if wait_for_ack { Period::Finite(TIMEOUT) } else { Period::Infinite }) {
            Ok(_) => {
                for _event in &events {
                    let msg = incoming_notif.recv_msg()?;
                    // router ACK'd our yield/seize request:
                    if msg.len() == 3 && msg.to_str()? == "ACK" {
                        wait_for_ack = false;
                        if verbose {
                            println!("got ACK");
                        }
                        continue;
                    }

                    let keep_in_scope: String;
                    let mut notif: Notification = bincode::deserialize(&msg.as_bytes())?;
                    if notif.hostname != hostname {
                        keep_in_scope = format!("@{}: {}", notif.hostname, notif.summary);
                        notif.summary = keep_in_scope.as_str()
                    };
                    spawn_local_notif(&notif)?;

                    if verbose {
                        println!("notifying: {} {}", notif.summary, notif.body);
                    }
                }
            },
            Err(err) => {
                if err.kind() == ErrorKind::Interrupted {
                    if let Ok(interrupt_message) = interrupt_rx.recv() {
                        wait_for_ack   = true;
                        replay_request = interrupt_message;
                    }
                }
                // spam request forever. TODO: make this configurable, could exit instead.
                let interrupt_message = if replay_request == Seize { "SEIZE" } else { "YIELD" };
                incoming_notif.send(interrupt_message)?;
                if verbose {
                    println!("sent {}: {}", interrupt_message, &config.as_client.server.incoming);
                }
            }
        }
    }
}

/// Route messages from senders to current notifier.  Only one notifier is active at the same time,
/// the latest one to have sent a SEIZE message. If no notifiers are active queue queue_size
/// messages before dropping the oldest notifications.
pub fn route(config: Config) -> Result<(), Error> {
    if let None = config.as_server {
        return Err(err_msg("missing as_server section in config"));
    }
    let verbose        = config.verbose.unwrap_or(false);
    let srv_config     = config.as_server.unwrap();
    let _auth_registry = &srv_config.auth.build()?;
    let server_creds   = CurveServerCreds::new(&srv_config.secret);
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

    if verbose {
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
                PollId(0) => // control message from notifier: SEIZE or YIELD
                    if let Ok(id) = notifier_change_request(&outgoing_notif, current_notifier_id, &mut queue, verbose) {
                        current_notifier_id = id;
                    },
                PollId(1) => // notification message to forward to notifier:
                    if let Ok(id) = fwd_notification(&incoming_notif, &outgoing_notif, current_notifier_id, &mut queue, queue_size, verbose) {
                        current_notifier_id = id;
                    },
                _ => unreachable!(),
            }
        }
    }
}

/// process SEIZE/YIELD messages from notifiers, returns current notifier (None on yield).
/// Used by router.
fn notifier_change_request(
    outgoing_notif:      &libzmq::Server,
    current_notifier_id: Option<libzmq::RoutingId>,
    queue:               &mut VecDeque<libzmq::Msg>,
    verbose:             bool,
) -> Result<Option<libzmq::RoutingId>, Error> {
    let notifier_req        = outgoing_notif.recv_msg()?;
    let notifier_req_str    = notifier_req.to_str()?;
    let id                  = notifier_req.routing_id().unwrap();
    let mut new_notifier_id = current_notifier_id;

    if notifier_req_str == "SEIZE" {
        new_notifier_id = Some(id);
        for msg in queue.iter() { // FIXME diff between queue & queue.iter()? // queue.iter().map(|msg| outgoing_notif.route(msg, id)).collect();
            outgoing_notif.route(msg, id)?;
        }
        queue.clear();
    } else if Some(id) == current_notifier_id { // YIELD: queue messages in the meantime, and only YIELD if notifier was the current notifier.
        new_notifier_id = None;
    }

    outgoing_notif.route("ACK", id)?;

    if verbose {
        println!("routing id {} request {}", id.0, notifier_req_str);
    }

    Ok(new_notifier_id)
}

/// Receive an incoming notification, send it to the current notifier.
/// If None are active queue the notification.
/// Used by router.
fn fwd_notification( // FIXME: arguments could be cleaner, looks like what I'd do in C.
    incoming_notif:      &libzmq::Server,
    outgoing_notif:      &libzmq::Server,
    current_notifier_id: Option<libzmq::RoutingId>,
    queue:               &mut VecDeque<libzmq::Msg>,
    queue_size:          usize,
    verbose:             bool,
) -> Result<Option<libzmq::RoutingId>, Error> {
    let notif_fwd = incoming_notif.recv_msg()?;
    let sender_id = notif_fwd.routing_id().unwrap();

    if let Some(notifier_id) = current_notifier_id {
        if verbose {
            println!("Forwarding message to routing id {:?}", notifier_id);
        }
        if let Ok(_) = outgoing_notif.route(&notif_fwd, notifier_id) {
            incoming_notif.route("ACK", sender_id)?;
            Ok(current_notifier_id)
        } else { // current_notifier has fucked off.
            queue_notification(notif_fwd, queue, queue_size, verbose);
            incoming_notif.route("QUEUED", sender_id)?;
            Ok(None)
        }
    } else { // queue!
        queue_notification(notif_fwd, queue, queue_size, verbose);
        incoming_notif.route("QUEUED", sender_id)?;
        Ok(current_notifier_id)
    }
}

fn queue_notification(
    notif_msg:  libzmq::Msg,
    queue:      &mut VecDeque<libzmq::Msg>,
    queue_size: usize,
    verbose:    bool,
) {
    if verbose {
        println!("no notifier: queueing");
    }
    if queue.len() == queue_size {
        queue.pop_front();
        if verbose {
            println!("dropping oldest");
        }
    }
    queue.push_back(notif_msg);
}

fn spawn_local_notif(notif: &Notification) -> Result<(), Error> {
    std::process::Command::new("notify-send")
        .args(&["-u", notif.urgency, "--", notif.summary, notif.body])
        .spawn()?
        .wait()?;
    Ok(())
}
