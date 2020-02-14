// #[derive(Debug)]
// pub struct Notification {
//     pub hostname: String,
//     pub title:    String,
//     pub body:     String,
//     pub priority: String, // u8, has 3 values.
// }
//
// impl Notification {
//     pub fn from_argv(mut args: std::env::Args) -> Result<Notification, failure::Error> {
//
//         let priority = args.next().expect("missing priority");
//         let title    = args.next().expect("missing title");
//         let body     = args.next().expect("missing body");
//         let hostname = hostname::get().unwrap().into_string().unwrap();
//
//         Ok(Notification { priority, title, body, hostname })
//     }
// }
