//use failure::Error;

pub enum Role {
    Sender,
    Aggregator,
    Sink,
}
pub struct Config {
    pub role: Role,
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

        Ok(Config { role })
    }
}

pub fn run_sender() -> Result<(), failure::Error> {
    println!("Sender");
    let context = zmq::Context::new();

    //socket to talk to clients
    let publisher = context.socket(zmq::PUB).unwrap();
    publisher.set_sndhwm(1_100_000).expect("failed setting hwm");
    publisher
        .bind("tcp://*:5561")
        .expect("failed binding publisher");

    //socket to receive signals
    let syncservice = context.socket(zmq::REP).unwrap();
    syncservice
        .bind("tcp://*:5562")
        .expect("failed binding syncservice");

    //get syncronization from subscribers
    println!("Waiting for subscribers");
    for _ in 0..10 {
        syncservice.recv_msg(0).expect("failed receiving sync");
        syncservice.send("", 0).expect("failed sending sync");
    }
    //now broadcast 1M updates followed by end
    println!("Broadcasting messages");
    for _ in 0..1_000_000 {
        publisher.send("Rhubarb", 0).expect("failed broadcasting");
    }
}

pub fn run_aggregator() -> Result<(), failure::Error> {
    println!("Aggregator");

    let context = zmq::Context::new();

    //first connect our subscriber
    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber
        .connect("tcp://localhost:5561")
        .expect("failed connecting subscriber");
    subscriber
        .set_subscribe(b"")
        .expect("failed setting subscription");
    thread::sleep(Duration::from_millis(1));

    //second sync with publisher
    let syncclient = context.socket(zmq::REQ).unwrap();
    syncclient
        .connect("tcp://localhost:5562")
        .expect("failed connect syncclient");
    syncclient.send("", 0).expect("failed sending sync request");
    syncclient.recv_msg(0).expect("failed receiving sync reply");

    //third get our updates and report how many we got
    let mut update_nbr = 0;
    loop {
        let message = subscriber
            .recv_string(0)
            .expect("failed receiving update")
            .unwrap();
        if message == "END" {
            break;
        }
        update_nbr += 1;
    }
    println!("Received {} updates", update_nbr);
}

pub fn run_sink() -> Result<(), failure::Error> {
    println!("sink");
    Ok(())
}

pub fn run(config: Config) -> Result<(), failure::Error> {
    match config.role {
        Role::Sender => run_sender(),
        Role::Aggregator => run_aggregator(),
        Role::Sink => run_sink(),
    }
}
