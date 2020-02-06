use std::env;
use std::process;
use std::error::Error;
use notifier::Config;

fn main() -> Result<(), failure::Error>{
    notifier::run(Config::new(env::args())?)
}
