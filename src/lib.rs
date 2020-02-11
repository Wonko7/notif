mod notif;
mod config;
use notif::Notification;
use config::Config;

pub fn run_sender(args: std::env::Args) -> Result<(), failure::Error> {
    // I don't understand why args doesn't need to be mut when we use it as mut in from_argv.
    let context = zmq::Context::new();
    let notif   = Notification::from_argv(args)?;
    let c = Config::new();
    println!("{:?}", c);

    println!("sending: {:?}", notif);
    let send_notif = context.socket(zmq::REQ)?;
    send_notif.connect("tcp://0:5561")?;

    let msg: [&[u8]; 4] = [
        &notif.hostname.as_str().as_bytes(),
        &notif.priority.as_str().as_bytes(),
        &notif.title.as_str().as_bytes(),
        &notif.body.as_str().as_bytes(),
    ];

    send_notif.send_multipart(&msg, 0)?;
    let ack = send_notif.recv_string(0)?;
    println!("sent and got ack: {}", ack.unwrap()); // if ack isn't utf8 well panic.

    Ok(())
}

pub fn run_server() -> Result<(), failure::Error> {
    println!("Server");

    let context = zmq::Context::new();
    //let mut notifier_id = "".as_bytes();
    let mut notifier_id = String::from("kekette"); // default for now...

    // recv notifs on subscriber
    let incoming_notif = context.socket(zmq::REP)?;
    incoming_notif.bind("tcp://0:5561")?;

    // recv yield requests:
    let notifier_yield = context.socket(zmq::REP)?;
    notifier_yield.bind("tcp://0:5562")?;

    // publish notif to single id notifier:
    let outgoing_notif = context.socket(zmq::PUB)?;
    outgoing_notif.bind("tcp://0:5563")?;

    loop {
        let mut items = [
            incoming_notif.as_poll_item(zmq::POLLIN),
            notifier_yield.as_poll_item(zmq::POLLIN),
        ];
        zmq::poll(&mut items, -1)?;

        if items[0].is_readable() {
            // forward notif to currently elected notifier:
            let messages = incoming_notif.recv_multipart(0)?;
            if messages.len() != 4 {
                println!("Dropping message with {} parts", messages.len());
                continue;
            }
            // could also use messages.insert(notifier_id.clone(), 0), seemed uglier.
            let msg: [&[u8]; 5] = [
                &notifier_id.as_str().as_bytes(), // PUB id env
                &messages[0],
                &messages[1],
                &messages[2],
                &messages[3],
            ];
            outgoing_notif.send_multipart(&msg, 0)?;
            incoming_notif.send("ack", 0)?;
        }

        if items[1].is_readable() {
            if let Ok(id) = notifier_yield.recv_string(0)? {
                println!("setting notifier subscribe to: {}", id);
                // server will make a unique ID per client. better yet, use zmq for that.
                // => dealer/router has what we want.
                // yield to the new notifier:
                notifier_id = id.clone();
                notifier_yield.send("ok man", 0)?;
            };
        }
    }
}

pub fn run_notifier(mut args: std::env::Args) -> Result<(), failure::Error> {
    let id = args.next().expect("--notifier <ID>: missing ID.");

    println!("notifier with id: {}", id);

    let context = zmq::Context::new();
    let gimme = context.socket(zmq::REQ)?; // signal USR1 to takeover again
    gimme.connect("tcp://0:5562")?;
    gimme.send(id.as_str(), 0)?;

    let a_ok = gimme.recv_string(0)?;
    println!("got answer: {}", a_ok.unwrap());


    // publish notif to single id notifier:
    let incoming_notif = context.socket(zmq::SUB)?;
    incoming_notif.connect("tcp://0:5563")?;
    incoming_notif.set_subscribe(id.as_bytes())?;

    // loop around incoming_notif.
    loop {
        let messages = incoming_notif.recv_multipart(0)?;
        if messages.len() != 5 {
            println!("Dropping message with {} parts", messages.len());
            continue;
        }

        // println!("1{} 2{} 3{} 4{} 5{}", conv(0)?, conv(1)?, conv(2)?, conv(3)?, conv(4)?);
        let conv            = |i: usize| String::from_utf8(messages[i].clone()); // couldn't get around the clone.
        let notif_hostname  = conv(1)?;
        let hostname        = hostname::get().unwrap();
        let title           = if notif_hostname.as_str() != hostname.to_str().unwrap() {
            notif_hostname + ": " + conv(3)?.as_str()
        } else {
            conv(3)?
        };

        std::process::Command::new("/usr/bin/notify-send")
            .arg("--")
            .arg(title)
            .arg(conv(4)?)
            .spawn()?;
    };
}

pub fn run() -> Result<(), failure::Error> {
    let mut args = std::env::args();
    args.next();

    match args.next() {
        None           => return Err(failure::err_msg("Didn't get a role as arg1")),
        Some(argument) => match argument.as_str() {
            "--send"     => run_sender(args),
            "--notifier" => run_notifier(args),
            "--server"   => run_server(),
            _            => Err(failure::format_err!("could not understand role {}", argument))
        }
    }
}
