mod notif;
mod config;
use notif::Notification;
use config::Config;

pub fn run_sender(config: Config, args: std::env::Args) -> Result<(), failure::Error> {
    // I don't understand why args^ doesn't need to be mut when we use it as mut in from_argv.
    let context    = zmq::Context::new();
    let send_notif = context.socket(zmq::REQ)?;
    send_notif.connect(format!("tcp://{}:{}", config.server_ip, config.incoming_notif_port).as_str())?;
    let notif      = Notification::from_argv(args)?;

    let msg: [&[u8]; 4] = [
        &notif.hostname.as_str().as_bytes(),
        &notif.priority.as_str().as_bytes(),
        &notif.title.as_str().as_bytes(),
        &notif.body.as_str().as_bytes(),
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

pub fn run_notifier(config: Config, mut args: std::env::Args) -> Result<(), failure::Error> {
    let id             = args.next().expect("--notifier <ID>: missing ID.");
    let context        = zmq::Context::new();
    let seize_notifier = context.socket(zmq::REQ)?; // TODO: signal USR1 to takeover again
    let incoming_notif = context.socket(zmq::SUB)?;
    let hostname       = hostname::get().unwrap().into_string().unwrap();

    seize_notifier.connect(format!("tcp://{}:{}", config.server_ip, config.notifier_seize_port).as_str())?;
    seize_notifier.send(id.as_str(), 0)?;
    incoming_notif.connect(format!("tcp://{}:{}", config.server_ip, config.outgoing_notif_port).as_str())?;
    incoming_notif.set_subscribe(id.as_bytes())?;
    seize_notifier.recv_string(0)?.unwrap(); // wait for ack.

    loop {
        let notif = incoming_notif.recv_multipart(0)?;
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
            .spawn()?;
    };
}

pub fn run() -> Result<(), failure::Error> {
    let mut args = std::env::args();
    let config   = Config::new()?;

    println!("running with: {:?}", config);

    args.next();
    match args.next() {
        Some(argument) => match argument.as_str() {
            "--send"     => run_sender(config, args),
            "--notifier" => run_notifier(config, args),
            "--server"   => run_server(config),
            _            => Err(failure::format_err!("could not understand role {}", argument))
        },
        None           => Err(failure::err_msg("Didn't get a role as arg1")),
    }
}
