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
use futures::future::{Loop, LoopFn, loop_fn};
use futures::{Stream, Sink, stream};
use futures::sync::mpsc;
use futures_cpupool::CpuPool;

use websocket::message::OwnedMessage;

use std::collections::BinaryHeap;
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{thread, time};

mod test;
mod gateway;

const DISCORD_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=6&encoding=json";

pub struct Client {
    core: Core,
    pool: Arc<CpuPool>,
    message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>,
    seq: Arc<AtomicUsize>,
}

impl Client {
    pub fn new(token: String) -> Client {
        let core = Core::new().expect("Could not create event loop core.");
        let pool = CpuPool::new(7);
        let message_queue = Arc::new(Mutex::new(BinaryHeap::<gateway::ClientMessage>::new()));
        let seq = Arc::new(AtomicUsize::new(0));

        Client::gateway_autenticate(token.clone(), message_queue.clone());

        Client {
            core,
            pool: Arc::new(pool),
            message_queue,
            seq,
        }
    }

    pub fn authenticate(&mut self) {
        let queue_ref1 = self.message_queue.clone();
        let queue_ref2 = self.message_queue.clone();
        let queue_ref3 = self.message_queue.clone();
        let seq_ref1 = self.seq.clone();
        let seq_ref2 = self.seq.clone();
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
                let (heartbeat_trigger_sender, heartbeat_trigger_receiver) = mpsc::unbounded();
                let (heartbeat_control_sender, heartbeat_control_receiver) = std::sync::mpsc::channel::<String>();

                let heartbeat_ready_sender = ready_sender.clone();

                let rx_worker = pool.spawn_fn(move || {
                    println!("\nReceiving worker is live\n");
                    rx.for_each(move |message| {
                        println!("\nGot a message\n");
                        let queue_ref = &queue_ref1;
                        let seq_ref = &seq_ref1;
                        let inner_queue = queue_ref.clone();
                        let inner_seq = seq_ref.clone();
                        let worker_channel = ready_sender.clone();

                        let message_handler = inner_pool.spawn_fn(move || {
                            println!("\nHandling a message in a new thread\n");
                            Client::handle_stream(message, inner_queue, inner_seq);
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
                        
                        message_handler
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
                        let mut queue = queue_ref.lock().expect("Could not get lock on queue.");
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
                        println!("\nSending a message\n");
                        OwnedMessage::Text(message)
                    })
                    .map_err(|_| { websocket::WebSocketError::NoDataAvailable })
                    .forward(tx)
                });

                let heartbeat_worker = pool.spawn_fn(move || {
                    loop_fn((heartbeat_trigger_sender, heartbeat_control_receiver), move |(trigger_channel, control)| {
                        if control.try_recv().is_err() {
                            let trigger_channel_next = trigger_channel.clone();

                            println!("\nSending heartbeat...\n");
                            trigger_channel.send("trigger").wait();

                            thread::sleep(time::Duration::from_secs(5));
                            Ok(Loop::Continue((trigger_channel_next, control)))
                        } else {
                            Ok(Loop::Break(()))
                        }
                    })
                });

                let heartbeat_sender_worker = pool.spawn_fn(move || { //required as LoopFn does not unlock the mutex
                    let queue_ref = queue_ref3;
                    let seq_ref = seq_ref2;
                    let worker_channel = heartbeat_ready_sender;
                    heartbeat_trigger_receiver.for_each(move |message| {
                        let inner_queue_ref = queue_ref.clone();
                        let inner_seq_ref = seq_ref.clone();
                        let mut queue = inner_queue_ref.lock().expect("Could not get lock on queue.");

                        let s = inner_seq_ref.load(Ordering::Relaxed) as i16;
                        if s == 0 {
                            let heartbeat_message_body = gateway::HeartbeatMessageBody::<String> {
                                op: 1,
                                d: String::from("null"),
                            };
                                let heartbeat_message = gateway::ClientMessage {
                                    body: serde_json::to_string(&heartbeat_message_body)
                                                         .expect("Could not serialize response."),
                                    priority: 9,
                                };

                                queue.push(heartbeat_message);
                        } else {
                            let heartbeat_message_body = gateway::HeartbeatMessageBody::<i16> {
                                op: 1,
                                d: s,
                            };
                                let heartbeat_message = gateway::ClientMessage {
                                    body: serde_json::to_string(&heartbeat_message_body)
                                                         .expect("Could not serialize response."),
                                    priority: 9,
                                };

                                queue.push(heartbeat_message);
                        }

                        

                        worker_channel.clone().send("ready").map_err(|e| {
                            ()
                        }).map(|result| {
                            ()
                        })
                    }).map_err(|e| {
                        websocket::WebSocketError::NoDataAvailable //FIXME: hack
                    }).map(|result| {
                        ()
                    })
                });

                rx_worker.join4(tx_stream_worker, tx_worker, heartbeat_worker.join(heartbeat_sender_worker))
            });
        
        self.core.run(socket);
    }

    fn gateway_autenticate(token: String, message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>) {
        let mut queue = message_queue.lock().expect("Could not get lock on queue.");

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
            priority: 10,
        };

        queue.push(identification_message);
    }

    fn handle_stream(message: websocket::OwnedMessage, message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>, seq_num: Arc<AtomicUsize>) {
        let mut queue = message_queue.lock().expect("Could not get lock on queue.");

        if let OwnedMessage::Text(text) = message {
            let deserialized_dispatch: gateway::ServerMessage = serde_json::from_str(&text)
                                                                     .expect("Could not parse JSON.");
            let serialized_data = deserialized_dispatch.d;
            let title = deserialized_dispatch.t;
            let opcode = deserialized_dispatch.op;
            if let Some(s) = deserialized_dispatch.s {
                seq_num.store(s as usize, Ordering::Relaxed);
            }
        
            match title {
                Some(t) => {
                    match t.trim() {
                        "READY" => {
                            let deserialized_data: gateway::ready::Ready = serde_json::from_value(serialized_data)
                                                                      .expect("Could not parse JSON.");
                            println!("\nServer: READY\n {:?} \n", deserialized_data);
                        }
                        _ => {
                            println!("\nUnknown server message\n {:?} \n", serialized_data);
                        }
                    }
                    
                }
                None => {
                    match opcode {
                        10 => {
                            let deserialized_data: gateway::hello::Hello = serde_json::from_value(serialized_data)
                                                                              .expect("Could not parse JSON.");
                            println!("\nServer: HELLO:\n {:?} \n", deserialized_data);
                        }
                        11 => {
                            println!("\nServer: Heartbeat acknowledged\n");
                        }
                        _ => {
                            println!("\nUnknown opcode\n");
                        }
                    }
                }
            }
        } else {
            println!("\nUnknown server message format\n");
        }
    }
}
