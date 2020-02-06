use std::env;
use std::process;
use std::error::Error;
use notifier::Config;

fn main() -> Result<(), failure::Error>{
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args)?;

    //println!("With text:\n{}", contents);
    if 1 == 2 {
        process::exit(5);
    }

    //Ok(());
    minigrep::run(config)
}

// fn main() -> Result<(), failure::Error> {
//     let config = Config::new(env::args()).unwrap_or_else(|err| {
//         eprintln!("Problem parsing arguments: {}", err);
//         process::exit(1);
//     });
// //     let config = Config::new(&args)?;
// //
//
//     if let Err(e) = minigrep::run(config) {
//         eprintln!("Application error: {}", e);
//         process::exit(1);
//     }
// }
