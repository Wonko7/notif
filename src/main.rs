extern crate clap;
use clap::{Arg, AppSettings, App, crate_version};

mod config;
mod notif;
mod run;

use config::Config;
use notif::Notification;
use libzmq::{TcpAddr, prelude::TryInto};

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
        // add -v --verbose for verbose println
        .arg(Arg::with_name("config") // TODO: does not work, but should be fine: https://github.com/clap-rs/clap/issues/1570
            //.short('c')
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .takes_value(true))
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
        .subcommand(App::new("generate")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about("helpers for config files")
            .subcommand(App::new("keys")
                .about("generate key pair"))
            .subcommand(App::new("topo")
                .about("generate config files for one server and multiple clients")
                .arg(Arg::with_name("INCOMING_ADDR")
                    .help("server incoming notification tcp address")
                    .required(true)
                    .index(1))
                .arg(Arg::with_name("OUTGOING_ADDR")
                    .help("server outgoing notification tcp address")
                    .required(true)
                    .index(2))
                .arg(Arg::with_name("NB_CLIENTS")
                    .help("number of client configs to generate")
                    .required(true)
                    .index(3))))
        .get_matches();

    let config_file = matches.value_of("config");
    let config      = Config::new(config_file);

    match matches.subcommand() {
        ("send", Some(ms)) => run::send(
            config?,
            Notification {
                hostname: hostname.as_str(),
                summary:  get_v(ms, "SUMMARY"),
                body:     get_v(ms, "BODY"),
                urgency:  get_v(ms, "urgency"),
            }
        ),
        ("notify", _) => run::notify(
            config?,
            hostname.as_str()
        ),
        ("route", _) => run::route(config?),
        // config:
        ("generate", Some(ms)) => match ms.subcommand() {
            ("keys", _) => Ok(config::generate_keys()),
            ("topo", Some(ms)) => {
                let incoming   = get_v(ms, "INCOMING_ADDR");
                let outgoing   = get_v(ms, "OUTGOING_ADDR");
                let nb_clients = get_v(ms, "NB_CLIENTS");
                config::generate_topo(&incoming.try_into()?, &outgoing.try_into()?, nb_clients.parse().unwrap())
            }
            _ => unreachable!() // FIXME check this.
        },
        _ => unreachable!()
    }
}
