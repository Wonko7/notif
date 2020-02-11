#[derive(PartialEq)]
pub enum Role {
    Sender,      // this is used to create a notification and send it to the server.
    Server,      // receives notifs from multiple Senders forwards them to ONE unique Notifier
    Notifier,
}
pub struct Config {
    pub role: Role,
    pub id: String,
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, failure::Error> {
        args.next();

        let role = match args.next() {
            None           => return Err(failure::err_msg("Didn't get a role as arg1")),
            Some(argument) => match argument.as_str() {
                "--send"     => Role::Sender,
                "--server"   => Role::Server,
                "--notifier" => Role::Notifier, // WIP not sure about names.
                _            => return Err(failure::format_err!("could not understand role {}", argument))
            }
        };

        let id = match (&role, args.next()) {
                (Role::Notifier, None)           => return Err(failure::err_msg("expecting notifier ID string.")),
                (Role::Notifier, Some(argument)) => argument,
                _                                => String::from("unused"),
        };

        Ok(Config { role, id })
    }
}
