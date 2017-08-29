extern crate websocket;
extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;


#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

#[macro_use] 
extern crate log;

use std::collections::BTreeMap;

use tokio_core::reactor::Core;

use futures::future::Future;
use futures::Stream;
use futures::Sink;
use futures::stream;
use futures_cpupool::CpuPool;

use websocket::message::OwnedMessage;

use std::collections::BinaryHeap;
use std::sync::Mutex;
use std::sync::Arc;


mod test;
mod gateway;

const DISCORD_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=6&encoding=json";

pub struct Client {
    core: Core,
    pool: Arc<CpuPool>,
    message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>,
}

impl Client {
    pub fn new(token: String) -> Client {
        let core = Core::new().expect("Could not create event loop core.");
        let pool = CpuPool::new(4);
        let message_queue = Arc::new(Mutex::new(BinaryHeap::<gateway::ClientMessage>::new()));

        Client::gateway_autenticate(token.clone(), message_queue.clone());

        Client {
            core,
            pool: Arc::new(pool),
            message_queue,
        }
    }

    pub fn authenticate(&mut self) {
        let queue_ref1 = self.message_queue.clone();
        let queue_ref2 = self.message_queue.clone();
        let pool = self.pool.clone();

        info!("\nAuthenticating\n");
        let event_loop = self.pool.spawn_fn(move || {
            info!("\nSpawned core loop\n");

            let mut core = Core::new().expect("Could not create event loop core.");
            let handle = core.handle();
            let socket = websocket::ClientBuilder::new(DISCORD_GATEWAY_URL)
                    .expect("Could not construct client.")
                    .add_protocol("rust-websocket")
                    .async_connect(None, &handle)
                    .and_then(move |(duplex, _)| {
                        let (tx, rx) = duplex.split();
                        info!("\nConnected to websocket\n");
                        let inner_pool = pool.clone();

                        let mut rx_core = Core::new().expect("Could not create event loop core.");
                        let rx_worker = pool.spawn_fn(move || {
                            info!("\nReceiving worker is live\n");
                            rx.for_each(move |message| {
                                info!("\nGot a message\n");
                                let queue_ref = &queue_ref1;
                                let inner_queue = queue_ref.clone();

                                let mut rx_handling_core = Core::new().expect("Could not create event loop core.");
                                let message_handling = inner_pool.spawn_fn(move || {
                                    info!("\nHandling a message in a new thread\n");
                                    Client::handle_stream(message, inner_queue).map_err(|_| ())
                                });
                                rx_handling_core.run(message_handling);

                                let g: Result<(), websocket::WebSocketError> = Ok(());
                                g
                            })
                        });
                        rx_core.run(rx_worker);

                        let mut tx_core = Core::new().expect("Could not create event loop core.");
                        let tx_worker = pool.spawn_fn(move || {
                            info!("\nTransmitting worker is live\n");
                            let queue_ref = &queue_ref2;
                            let mut queue = queue_ref.lock().unwrap();
                            let mut messages = Vec::<Result<OwnedMessage, websocket::WebSocketError>>::new();

                            while let Some(message) = queue.pop() {
                                &messages.push(Ok(OwnedMessage::Text(message.body)));
                            }

                            let messages_wrapped = messages.into_iter();
                            stream::iter(messages_wrapped).forward(tx)
                        });
                        tx_core.run(tx_worker);

                        let g: Result<(), websocket::WebSocketError> = Ok(());
                        g
                    });
            
            core.run(socket);

            let g: Result<(), websocket::WebSocketError> = Ok(());
            g
        });
        self.core.run(event_loop);
    }

    fn gateway_autenticate(token: String, message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>) {
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

    fn handle_stream(message: websocket::OwnedMessage, message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>) -> Result<(), websocket::WebSocketError> {
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
                            info!("\nServer: READY\n");
                            Ok(())
                        }
                        _ => {
                            Ok(())
                        }
                    }
                    
                }
                None => {
                    let deserialized_data: gateway::hello::Hello = serde_json::from_value(serialized_data)
                                                                      .expect("Could not parse JSON.");
                    info!("\nServer: IDENTIFY\n");
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}
