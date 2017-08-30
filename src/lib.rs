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
use futures::sync::mpsc;
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
        let pool = CpuPool::new(5);
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

        let handle = self.core.handle();
        let socket = websocket::ClientBuilder::new(DISCORD_GATEWAY_URL)
            .expect("Could not construct client.")
            .add_protocol("rust-websocket")
            .async_connect(None, &handle)
            .and_then(move |(duplex, _)| {
                let (tx, rx) = duplex.split();
                println!("\nConnected to websocket\n");
                let inner_pool = pool.clone();
                let mut core = Core::new().expect("Could not create event loop core.");
                let (ready_sender, ready_receiver) = mpsc::unbounded::<&str>();
                let (message_sender, message_receiver) = mpsc::unbounded::<String>();

                let rx_worker = pool.spawn_fn(move || {
                    println!("\nReceiving worker is live\n");
                    rx.for_each(move |message| {
                        println!("\nGot a message\n");
                        let queue_ref = &queue_ref1;
                        let inner_queue = queue_ref.clone();
                        let worker_channel = ready_sender.clone();

                        //let mut rx_handling_core = Core::new().expect("Could not create event loop core.");
                        let message_handler = inner_pool.spawn_fn(move || {
                            println!("\nHandling a message in a new thread\n");
                            Client::handle_stream(message, inner_queue);
                            println!("\nSending ready\n");
                            worker_channel.send("ready").map_err(|err| {
                                ()
                            }).map(|result| {
                                ()
                            })
                        }).map_err(|e| {
                            websocket::WebSocketError::NoDataAvailable //FIXME: hack
                        }).map(|result| {
                            ()
                        });
                        //rx_handling_core.run(message_handler);

                        
                        message_handler
                        //let all_ok: Result<(), websocket::WebSocketError> = Ok(());
                        //all_ok
                    }).map_err(|e| {
                        websocket::WebSocketError::NoDataAvailable //FIXME: hack
                    }).map(|result| {
                        ()
                    })
                });

                let tx_stream_worker = pool.spawn_fn(move || {
                    println!("\nFiltering worker is live\n");
                    let inner_message_sender = message_sender;
                    ready_receiver.for_each(move |message| {
                        println!("\nReceiving ready\n");

                        let queue_ref = &queue_ref2;
                        let mut queue = queue_ref.lock().unwrap();
                        let mut messages = Vec::<Result<String, mpsc::SendError<String>>>::new();

                        while let Some(message) = queue.pop() {
                            println!("\nPopping a message off the queue\n");
                            &messages.push(Ok(message.body));
                        }

                        let message_stream = messages.into_iter();
                        stream::iter(message_stream)
                            .forward(inner_message_sender.clone())
                            .map_err(|e| {
                                () //FIXME: hack
                            }).map(|result| {
                                ()
                            })
                    }).map_err(|e| {
                        websocket::WebSocketError::NoDataAvailable //FIXME: hack
                    }).map(|result| {
                        ()
                    })
                });

                let tx_worker = pool.spawn_fn(move || {
                    println!("\nTransmitting worker is live\n");
                    message_receiver.map(|message| { 
                        OwnedMessage::Text(message)
                    })
                    .map_err(|_| { websocket::WebSocketError::NoDataAvailable })
                    .forward(tx)
                });

                //rx_worker
                rx_worker.join(tx_stream_worker.join(tx_worker))
            });
        
        self.core.run(socket);
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
                            println!("\nServer: READY\n");
                            Ok(())
                        }
                        _ => {
                            println!("\nUnknown server message\n");
                            Ok(())
                        }
                    }
                    
                }
                None => {
                    let deserialized_data: gateway::hello::Hello = serde_json::from_value(serialized_data)
                                                                      .expect("Could not parse JSON.");
                    println!("\nServer: IDENTIFY\n");
                    Ok(())
                }
            }
        } else {
            println!("\nUnknown server message format\n");
            Ok(())
        }
    }
}
