//use failure::Error;
use std::thread;
use std::time::Duration;

#[derive(PartialEq)]
pub enum Role {
    Sender,
    Aggregator,
    Sink,
}
pub struct Config {
    pub role: Role,
    pub id: String,
}


impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, failure::Error> {
        args.next();

        let role = match args.next() {
            None            => return Err(failure::err_msg("Didn't get a role as arg1")),
            Some(argument)  => match argument.as_str() {
                "--sender"     => Role::Sender,
                "--aggregator" => Role::Aggregator,
                "--sink"       => Role::Sink,
                _              => return Err(failure::format_err!("could not understand role {}", argument))
            }
        };

        let id = match (&role, args.next()) {
                (Role::Sink, None)           => return Err(failure::err_msg("expecting sink ID string.")),
                (Role::Sink, Some(argument)) => argument,
                _                            => String::from("unused"),
        };

        Ok(Config { role, id })
    }
}

pub fn run_sender() -> Result<(), failure::Error> {
    println!("Sender");
    let context = zmq::Context::new();

    //socket to talk to clients
    let publisher = context.socket(zmq::PUB)?;
    publisher.set_sndhwm(1_100_000).expect("failed setting hwm");
    publisher.connect("tcp://0:5561")?;

    //now broadcast 1M updates followed by end
    println!("Broadcasting messages");
    for i in 0..1_000_000 {
        println!("{}", i);
        publisher.send("Rhubarb", 0).expect("failed broadcasting");
        thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}

pub fn run_aggregator() -> Result<(), failure::Error> {
    println!("Aggregator");

    let context = zmq::Context::new();
    //let mut sink_client_id = "".as_bytes();
    let mut sink_client_id = String::from("");

    // recv notifs on subscriber
    let incoming_notif = context.socket(zmq::SUB)?;
    incoming_notif.bind("tcp://0:5561")?;
    incoming_notif.set_subscribe(b"")?;

    // recv yield requests:
    let sink_yield = context.socket(zmq::REP)?;
    sink_yield.bind("tcp://0:5562")?;

    // publish notif to single id sink:
    let sink_notif = context.socket(zmq::PUB)?;
    sink_notif.bind("tcp://0:5563")?;


    loop {
        let mut items = [
            incoming_notif.as_poll_item(zmq::POLLIN),
            sink_yield.as_poll_item(zmq::POLLIN),
        ];
        zmq::poll(&mut items, -1)?;

        // example ref use this: if items[0].is_readable() && receiver.recv(&mut msg, 0).is_ok() {
        if items[0].is_readable() {
            let message = match incoming_notif.recv_string(0)? {
                Ok(m) =>  m,
                Err(_) => continue
            };

            sink_notif.send(&sink_client_id[..], zmq::SNDMORE)?;
            sink_notif.send(&message[..], 0)?;
        }

        if items[1].is_readable() {
            match sink_yield.recv_string(0)? {
                Ok(id) =>  {
                    println!("setting sink notif subscribe to: {}", id);
                    // sink_client_id = id.as_bytes().clone();
                    sink_client_id = id.clone();
                    sink_yield.send("ok man", 0)?;
                },
                Err(_) => continue
            };
        }
    }
}

pub fn run_sink(config: Config) -> Result<(), failure::Error> {
    println!("sink with id: {}", config.id);

    let context = zmq::Context::new();

    // register and ask for others to yield:
    let gimme = context.socket(zmq::REQ)?;
    gimme.connect("tcp://0:5562")?;
    gimme.send(&config.id[..], 0)?;

    let a_ok = gimme.recv_string(0)?;
    println!("got answer: {}", a_ok.unwrap());


    // publish notif to single id sink:
    let incoming_notif = context.socket(zmq::SUB)?;
    incoming_notif.connect("tcp://0:5563")?;
    incoming_notif.set_subscribe(config.id.as_bytes())?;

    // loop around incoming_notif.
    loop {
        let message = match incoming_notif.recv_string(0)? {
            Ok(m)  => m,
            Err(_) => continue
        };
        println!("END GAYME {}", message);
    }
}

pub fn run(config: Config) -> Result<(), failure::Error> {
    match config.role {
        Role::Sender     => run_sender(),
        Role::Aggregator => run_aggregator(),
        Role::Sink       => run_sink(config),
    }
}
