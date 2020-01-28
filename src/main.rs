use libzmq::{prelude::*, *};
use std::convert::TryInto;

fn main() -> Result<(), failure::Error> {
    // Use a system assigned port.
    let addr: TcpAddr = "127.0.0.1:*".try_into()?;

    let server = ServerBuilder::new()
        .bind(addr)
        .build()?;

    // Retrieve the addr that was assigned.
    let bound = server.last_endpoint()?;

    let client = ClientBuilder::new()
        .connect(bound)
        .build()?;

    // Send a string request.
    client.send("tell me something")?;

    // Receive the client request.
    let msg = server.recv_msg()?;
    let id = msg.routing_id().unwrap();

    // Reply to the client.
    server.route("it takes 224 bits to store a i32 in java", id)?;

    // We can reply as much as we want.
    server.route("also don't talk to me", id)?;

    // Retreive the first reply.
    let mut msg = client.recv_msg()?;
    // And the second.
    client.recv(&mut msg)?;


    println!("Hello, world!");
    Ok(())
}
