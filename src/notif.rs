#[derive(Debug)]
pub struct Notification {
    pub hostname: String,
    pub title:    String,
    pub body:     String,
    pub priority: String, // might end up u32
}

impl Notification { // rewrite all of this
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
