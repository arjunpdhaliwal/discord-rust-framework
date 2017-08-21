extern crate websocket;
extern crate futures;
extern crate tokio_core;

use tokio_core::reactor::Core;

use futures::future::Future;
use futures::Stream;
use futures::Sink;

mod test;

const DISCORD_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=6";

pub struct Client {
    tx: websocket::sync::Client<std::boxed::Box<websocket::stream::sync::NetworkStream + std::marker::Send>>,
}

impl Client {
    pub fn new() -> Result<i32, ()> {
        let mut core = Core::new().unwrap();

        let socket = websocket::ClientBuilder::new(DISCORD_GATEWAY_URL)
            .expect("Could not construct client.")
            .add_protocol("rust-websocket")
            .async_connect(None, &core.handle())
            .and_then(|(duplex, _)| {
                let (sink, stream) = duplex.split();
                stream.filter_map(|message| {
                    println!("Received Message: {:?}", message);
                    Some(message)
                })
                  .forward(sink)
            });


        core.run(socket).unwrap();
        Ok(1)
    }

    fn send_heartbeat(&self) {
    }

    pub fn authenticate() -> Result<i32, i32> {
        Ok(1)
    }
}






