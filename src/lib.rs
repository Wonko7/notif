use std::thread;
use std::time::Duration;

mod notif;
mod config;
use notif::Notification;
use config::{ Config, Role };


pub fn run_sender() -> Result<(), failure::Error> {
    // for testing only.
    // this will just send argv to the server & exit.
    println!("Sender");
    let context = zmq::Context::new();

    let notif = Notification::from_argv(std::env::args())?;
    println!("got: {:?}", notif);


    //socket to talk to clients
    let publisher = context.socket(zmq::PUB)?;
    publisher.set_sndhwm(1_100_000).expect("failed setting hwm");
    publisher.connect("tcp://0:5561")?;
    thread::sleep(Duration::from_millis(500));

    let msg: [&[u8]; 4] = [
        &notif.hostname.as_str().as_bytes(),
        &notif.priority.as_str().as_bytes(),
        &notif.title.as_str().as_bytes(),
        &notif.body.as_str().as_bytes(),
    ];

    publisher.send_multipart(&msg, 0)?;

    thread::sleep(Duration::from_millis(500));

    Ok(())
}

pub fn run_server() -> Result<(), failure::Error> {
    println!("Server");

    let context = zmq::Context::new();
    //let mut notifier_id = "".as_bytes();
    let mut notifier_id = String::from("kekette"); // default for now...

    // recv notifs on subscriber
    let incoming_notif = context.socket(zmq::SUB)?;
    incoming_notif.bind("tcp://0:5561")?;
    incoming_notif.set_subscribe(b"")?;

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

            println!("got {} msg!", messages.len());
            if messages.len() != 4 {
                println!("Dropping message with {} parts", messages.len());
                continue;
            }

            let msg: [&[u8]; 5] = [
                &notifier_id.as_str().as_bytes(), // PUB id env
                &messages[0],
                &messages[1],
                &messages[2],
                &messages[3],
            ];
            println!("made {} msg!", msg.len());

            outgoing_notif.send_multipart(&msg, 0)?;
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

pub fn run_notifier(config: Config) -> Result<(), failure::Error> {
    println!("notifier with id: {}", config.id);

    let context = zmq::Context::new();

    // register and ask for others to yield:
    let gimme = context.socket(zmq::REQ)?;
    gimme.connect("tcp://0:5562")?;
    gimme.send(config.id.as_str(), 0)?;

    let a_ok = gimme.recv_string(0)?;
    println!("got answer: {}", a_ok.unwrap());


    // publish notif to single id notifier:
    let incoming_notif = context.socket(zmq::SUB)?;
    incoming_notif.connect("tcp://0:5563")?;
    incoming_notif.set_subscribe(config.id.as_bytes())?;

    // loop around incoming_notif.
    loop {
        let mut messages = incoming_notif.recv_multipart(0)?; // FIXME assert length.

        let fugly_conv   = |i| String::from_utf8(messages.iter().nth(i).unwrap().clone()).unwrap();
        let env          = fugly_conv(0);
        let priority     = fugly_conv(1);
        let title        = fugly_conv(2);
        let body         = fugly_conv(3);
        let hostname     = fugly_conv(4);

        let notif        = Notification { priority, title, body, hostname };

        println!("ENDGAYME {:?}", notif);
    };
}

pub fn run() -> Result<(), failure::Error> {
    let config = Config::new(std::env::args())?;

    match config.role {
        Role::Sender   => run_sender(),
        Role::Server   => run_server(),
        Role::Notifier => run_notifier(config),
    }
}
