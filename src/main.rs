extern crate clap;
use clap::{Arg, AppSettings, App, crate_version};

mod config;
mod notif;
mod run;

use config::Config;
use notif::Notification;

fn get_v<'a>(opts: &'a clap::ArgMatches, name: &str) -> &'a str {
    // couldn't put a lifetime on a closure, this is just a helper to get values out of matches
    opts.value_of(name).unwrap()
}

fn main() -> Result<(), failure::Error> {
    let verbose  = false;
    let hostname = hostname::get().unwrap().into_string().unwrap();

    let matches = App::new("notif")
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("william@undefined.re")
        .about("routes remote notifications to you")
        .arg(Arg::with_name("config") // TODO: does not work, but should be fine: https://github.com/clap-rs/clap/issues/1570
            //.short('c')
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
        // TODO add "generate" command for keys/config.
        .subcommand(App::new("send")
            .about("send a notification")
            .arg(Arg::with_name("SUMMARY")
                .help("notification summary")
                .required(true)
                .index(1))
            .arg(Arg::with_name("BODY")
                .help("notification body")
                .required(true)
                .index(2))
            .arg(Arg::with_name("urgency")
                //.short('u')
                .short("u")
                .default_value("normal")
                .possible_values(&["low", "normal", "critical"])
                .help("urgency")))
        .subcommand(App::new("notify")
            .about("show notifications"))
        .subcommand(App::new("route")
            .about("receive and forward notifications"))
        .get_matches();

    // TODO add --verbose args work again.
    let config_file = matches.value_of("config");
    let config      = Config::new(config_file)?;

    match matches.subcommand() {
        ("send", Some(ms))   => run::send(
            config,
            Notification {
                hostname: hostname.as_str(),
                summary:  get_v(ms, "SUMMARY"),
                body:     get_v(ms, "BODY"),
                urgency:  get_v(ms, "urgency"),
            }
        ),
        ("notify", _) => run::notify(
            config,
            hostname.as_str()
        ),
        ("route", _)         => run::route(config),
        _                    => Err(failure::err_msg("unreachable"))
    }
}
