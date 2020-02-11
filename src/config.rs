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
    pub fn new(args: &mut std::env::Args) -> Result<Config, failure::Error> {
        let unused_id = String::from("unused");

        args.next();
        let (role, id) = match args.next() {
            None           => return Err(failure::err_msg("Didn't get a role as arg1")),
            Some(argument) => match argument.as_str() {
                "--send"     => (Role::Sender, unused_id),
                "--server"   => (Role::Server, unused_id),
                "--notifier" => // sigh.
                    if let Some(id) = args.next() {
                        (Role::Notifier, id)
                    } else {
                        return Err(failure::err_msg("expecting --notifier ID"))
                    }
                _            => return Err(failure::format_err!("could not understand role {}", argument))
            }
        };

        Ok(Config { role, id })
    }
}
