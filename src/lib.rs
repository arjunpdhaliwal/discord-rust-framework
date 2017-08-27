extern crate websocket;
extern crate futures;
extern crate tokio_core;
extern crate tokio_pool;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;


use std::collections::BTreeMap;

use tokio_core::reactor::Core;

use futures::future::Future;
use futures::Stream;

use websocket::message::OwnedMessage;

use std::collections::BinaryHeap;
use std::sync::Mutex;
use std::sync::Arc;


mod test;
mod gateway;

const DISCORD_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=6&encoding=json";

pub struct Client {
    core: tokio_core::reactor::Core,
    pool: Arc<tokio_pool::TokioPool>,
    join: Arc<tokio_pool::PoolJoin>,
    message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>,
}

impl Client {
    pub fn new() -> Client {
        let core = Core::new()
                       .expect("Could not instantiate Client.");
        let message_queue = Arc::new(Mutex::new(BinaryHeap::<gateway::ClientMessage>::new()));
        let (pool, join) = tokio_pool::TokioPool::new(2)
                                     .expect("Could not create thread pool.");

        Client {
            core,
            pool: Arc::new(pool),
            join: Arc::new(join),
            message_queue,
        }
    }

    pub fn authenticate(&mut self, token: String) {
        let queue_ref1 = &self.message_queue;
        let queue_ref2 = &self.message_queue;

        let socket = websocket::ClientBuilder::new(DISCORD_GATEWAY_URL)
            .expect("Could not construct client.")
            .add_protocol("rust-websocket")
            .async_connect(None, &(&mut self.core).handle())
            .and_then(|(duplex, _)| {
                    Client::gateway_autenticate(token.clone(), queue_ref1);
                    let (sink, stream) = duplex.split();
                    let a = stream.filter_map(|message| Client::handle_stream(message, queue_ref2));
                    a.forward(sink)
            });
        

        self.core.run(socket);
    }

    fn gateway_autenticate(token: String, message_queue: &Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>) {
        let mut queue = message_queue.lock().unwrap();

        let mut properties = BTreeMap::new();
        properties.insert(String::from("$os"), String::from("Linux"));

        let identity = gateway::identity::Identity {
            token,  
            properties,
            compress: None,
            large_threshold: None,
        };

        let identification_message_body = gateway::MessageBody {
            op: 2,
            d: identity,
            s: None,
            t: None,
        };

        let identification_message = gateway::ClientMessage {
            body: serde_json::to_string(&identification_message_body)
                                 .expect("Could not serialize response."),
            priority: 0,
        };

        queue.push(identification_message);
    }

    fn handle_stream(message: websocket::OwnedMessage, message_queue: &Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>) -> Option<websocket::OwnedMessage> {
        let mut queue = message_queue.lock().unwrap();

        if let OwnedMessage::Text(text) = message {
            let deserialized_dispatch: gateway::ServerMessage = serde_json::from_str(&text)
                                                                     .expect("Could not parse JSON.");
            let serialized_data = deserialized_dispatch.d;
            let title = deserialized_dispatch.t;

            match title {
                Some(t) => {
                    match t.trim() {
                        "READY" => {
                            let deserialized_data: gateway::ready::Ready = serde_json::from_value(serialized_data)
                                                                      .expect("Could not parse JSON.");
                            println!("\nServer: {:#?}\n", deserialized_data);
                            Some(OwnedMessage::Close(None))
                        }
                        _ => {
                            None
                        }
                    }
                    
                }
                None => {
                    let deserialized_data: gateway::hello::Hello = serde_json::from_value(serialized_data)
                                                                      .expect("Could not parse JSON.");
                    println!("\nServer: {:#?}\n", deserialized_data);

                    let message = queue.pop()
                                                    .expect("Could not read message from queue.");
                    println!("\nResponse: {:#?}\n", message.body);
                    Some(OwnedMessage::Text(message.body))
                }
            }
        } else {
            None
        }
    }
}
