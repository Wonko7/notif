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

        let id = if Role::Sink == role   {
            match args.next() {
                None            => return Err(failure::err_msg("expecting sink ID string.")),
                Some(argument)  => argument
            }
        } else {
            String::from("unused")
        };


        Ok(Config { role, id })
    }
}

pub fn run_sender() -> Result<(), failure::Error> {
    println!("Sender");
    let context = zmq::Context::new();

    //socket to talk to clients
    let publisher = context.socket(zmq::PUB).unwrap();
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

    //first connect our subscriber
    let subscriber = context.socket(zmq::SUB)?;
    println!("1");
    subscriber.bind("tcp://0:5561")?;
    println!("2");
    subscriber.set_subscribe(b"")?;
    println!("3");

    //third get our updates and report how many we got
    let mut update_nbr = 0;
    loop {
        let message = match subscriber.recv_string(0)? {
            Ok(m) => m,
            Err(_) => {
                println!("ignoring non utf8");
                continue;
            }
        };

        println!("{}", message);
        if message == "END" {
            break;
        }
        update_nbr += 1;
    }
    println!("Received {} updates", update_nbr);
    Ok(())
}

pub fn run_sink(config: Config) -> Result<(), failure::Error> {
    println!("sink with id: {}", config.id);
    Ok(())
}

pub fn run(config: Config) -> Result<(), failure::Error> {
    match config.role {
        Role::Sender     => run_sender(),
        Role::Aggregator => run_aggregator(),
        Role::Sink       => run_sink(config),
    }
}
