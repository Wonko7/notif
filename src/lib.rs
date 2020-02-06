use failure::Error;

pub enum Role {
    Sender,
    Aggregator,
    Sink,
}
pub struct Config {
    pub role: Role,
}


impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, &'static str> {
        args.next();

        let role = match args.next() {
            None            => return Err("Didn't get a role as arg1"),
            Some(argument)  => match argument.as_str() {
                "--sender"     => Role::Sender,
                "--aggregator" => Role::Aggregator,
                "--sink"       => Role::Sink,
                _              => return Err(format!("could not understand role {}", argument).as_str())
            }
        };

        Ok(Config { role })
    }
}

pub fn run_sender() -> Result<(), failure::Error> {
    if 1 == 2 {
        return Err("lol");
    }
    println!("sender");
    Ok(())
}

pub fn run_aggregator() -> Result<(), failure::Error> {
    println!("sender");
    Ok(())
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
