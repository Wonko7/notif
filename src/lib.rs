extern crate clap;
use clap::{Arg, AppSettings, App};

mod config;
use config::Config;
mod run;


fn get_v<'a>(opts: &'a clap::ArgMatches, name: &str) -> &'a str {
    // couldn't put a lifetime on a closure, this is just a helper to get values out of matches
    opts.value_of(name).unwrap()
}

pub fn run() -> Result<(), failure::Error> {
    let verbose  = false;
    let hostname = hostname::get().unwrap().into_string().unwrap();

    let matches = App::new("notif")
        .version("0.0.3") // hmm.
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("william@undefined.re")
        .about("routes remote notifications to you")
        .arg(Arg::with_name("config") // TODO: does not work, but should be fine: https://github.com/clap-rs/clap/issues/1570
            .short('c')
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
                .short('u')
                .default_value("normal")
                .possible_values(&["low", "normal", "critical"])
                .help("urgency")))
        .subcommand(App::new("notify")
            .about("show notifications")
            .arg(Arg::with_name("ID")
                .help("notifier ID")
                .required(true)
                .index(1)))
        .subcommand(App::new("route")
            .about("receive and forward notifications"))
        .get_matches();

    // TODO add --verbose args work again.
    let config_file = matches.value_of("config");
    let config = Config::new(config_file)?;

    match matches.subcommand() {
        ("send", Some(ms))   => run::send(
            config,
            hostname.as_str(),
            get_v(ms, "SUMMARY"),
            get_v(ms, "BODY"),
            get_v(ms, "urgency"),
        ),
        ("notify", Some(ms)) => run::notify(
            config,
            hostname.as_str(),
            get_v(ms, "ID").to_string() // because the thread takes ownership.
            ),
        ("route", _)         => run::serve(config),
        _                    => Err(failure::err_msg("unreachable"))
    }
}
