#[derive(Debug)]
pub struct Notification {
    pub hostname: String,
    pub title:    String,
    pub body:     String,
    pub priority: String, // might end up u32
}

impl Notification {
    pub fn from_argv(mut args: std::env::Args) -> Result<Notification, failure::Error> {
        // lol, this can't be canonical rust.
        if let (Some(priority), Some(title), Some(body), Ok(h)) = (args.next(), args.next(), args.next(), hostname::get()) {
            if let Ok(hostname) = h.into_string() {
                return Ok(Notification { priority, title, body, hostname });
            }
        }
        return Err(failure::err_msg("expecting --send priority title body"));
    }
}

#[derive(Debug)]
pub struct Notif<'a> {
    pub hostname: &'a [u8],
    pub title:    &'a [u8],
    pub body:     &'a [u8],
    pub priority: &'a [u8], // might end up u32
}
