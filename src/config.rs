#[derive(PartialEq)]
pub enum Role {
    Sender,      // this is used to create a notification and send it to the server.
    Server,      // receives notifs from multiple Senders forwards them to ONE unique Notifier
    Notifier,
}
