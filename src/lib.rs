extern crate websocket;
extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate reqwest;
extern crate hyper;


#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

#[macro_use] 
extern crate log;

use std::collections::BTreeMap;

use tokio_core::reactor::Core;

use futures::future::Future;
use futures::future::{Loop, loop_fn};
use futures::{Stream, Sink, stream};
use futures::sync::mpsc;
use futures_cpupool::CpuPool;

use websocket::message::OwnedMessage;

use hyper::header::{Headers, Authorization};

use std::collections::BinaryHeap;
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{thread, time};

mod test;
mod gateway;
mod api;

const DISCORD_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=6&encoding=json";

pub struct Client<F> {
    core: Core,
    http: Arc<Mutex<reqwest::Client>>,
    pool: Arc<CpuPool>,
    message_queue: Arc<Mutex<BinaryHeap<gateway::ClientMessage>>>,
    seq: Arc<AtomicUsize>,
    message_handler: Arc<Mutex<F>>,
    token: String,
    user_id: String,
}

impl<F> Client<F> where F: Fn(&str) -> Option<String> + Send + Sync + 'static {
    pub fn new(token: String, message_handler: F) -> Client<F> {
        let core = Core::new().expect("Could not create event loop core.");
        let pool = CpuPool::new(7);
        let message_queue = Arc::new(Mutex::new(BinaryHeap::<gateway::ClientMessage>::new()));
        let seq = Arc::new(AtomicUsize::new(0));
        let http = reqwest::Client::new().expect("Could not create HTTP client.");

        let me_api_url = "https://discordapp.com/api/users/@me";

        let mut auth_header = String::from("Bot ");
        auth_header.push_str(&*token);
        let mut req_headers = Headers::new();
        req_headers.set(Authorization(auth_header));

        let mut res = http.get(me_api_url)
                      .expect("Could not start GET request.")
                      .headers(req_headers)
                      .send()
                      .expect("Could not send HTTP request.");

        let deserialized_response: api::User = res.json().expect("Could not parse response JSON.");
        let user_id = deserialized_response.id;

        Client::<F>::gateway_autenticate(token.clone(), message_queue.clone());

        Client {
            core,
            http: Arc::new(Mutex::new(http)),
            pool: Arc::new(pool),
            message_queue,
            seq,
            message_handler: Arc::new(Mutex::new(message_handler)),
            token,
            user_id,
        }
    }

    #[allow(unused_variables)]
    #[allow(unused_must_use)]
    #[allow(deprecated)]
    pub fn connect(&mut self) {
        let queue_ref1 = self.message_queue.clone();
        let queue_ref2 = self.message_queue.clone();
        let seq_ref1 = self.seq.clone();
        let seq_ref2 = self.seq.clone();
        let handler_ref = self.message_handler.clone();
        let http_ref = self.http.clone();
        let token = self.token.clone();
        let user_id = self.user_id.clone();
        let pool = self.pool.clone();

        let handle = self.core.handle();
        let socket = websocket::ClientBuilder::new(DISCORD_GATEWAY_URL)
            .expect("Could not construct client.")
            .add_protocol("rust-websocket")
            .async_connect(None, &handle)
            .and_then(move |(duplex, _)| {
                let (tx, rx) = duplex.split();
                info!("\nConnected to websocket\n");
                let inner_pool = pool.clone();
                let (ready_sender, ready_receiver) = mpsc::unbounded::<&str>();
                let (message_sender, message_receiver) = mpsc::unbounded::<String>();
                let (heartbeat_trigger_sender, heartbeat_trigger_receiver) = mpsc::unbounded();
                let (heartbeat_control_sender, heartbeat_control_receiver) = std::sync::mpsc::channel::<String>();

                let heartbeat_ready_sender = ready_sender.clone();

                let rx_worker = pool.spawn_fn(move || {
                    info!("\nReceiving worker is live\n");
                    rx.for_each(move |message| {
                        info!("\nGot a message\n");
                        let seq_ref = &seq_ref1;
                        let inner_seq = seq_ref.clone();
                        let worker_channel = ready_sender.clone();
                        let inner_handler_ref = handler_ref.clone();
                        let inner_http_ref = http_ref.clone();
                        let inner_token = token.clone();
                        let inner_user_id = user_id.clone();

                        let message_handler = inner_pool.spawn_fn(move || {
                            info!("\nHandling a message in a new thread\n");
                            Client::<F>::handle_stream(message, inner_seq, inner_handler_ref.clone(), inner_http_ref.clone(), inner_token.clone(), inner_user_id.clone());
                            info!("\nSending ready\n");
                            worker_channel.send("ready").map_err(|err| {
                                ()
                            }).map(|result| {
                                ()
                            })
                        }).map_err(|e| {
                            websocket::WebSocketError::NoDataAvailable
                        }).map(|result| {
                            ()
                        });
                        
                        message_handler
                    }).map_err(|e| {
                        websocket::WebSocketError::NoDataAvailable
                    }).map(|result| {
                        ()
                    })
                });

                let tx_stream_worker = pool.spawn_fn(move || {
                    info!("\nFiltering worker is live\n");
                    let inner_message_sender = message_sender;
                    ready_receiver.for_each(move |message| {
                        info!("\nReceiving ready\n");

                        let queue_ref = &queue_ref1;
                        let mut queue = queue_ref.lock().expect("Could not get lock on queue.");
                        let mut messages = Vec::<Result<String, mpsc::SendError<String>>>::new();

                        while let Some(message) = queue.pop() {
                            info!("\nPopping a message off the queue\n");
                            &messages.push(Ok(message.body));
                         }

                        let message_stream = messages.into_iter();
                        stream::iter(message_stream)
                            .forward(inner_message_sender.clone())
                            .map_err(|e| {
                                () 
                            }).map(|result| {
                                ()
                            })
                    }).map_err(|e| {
                        websocket::WebSocketError::NoDataAvailable
                    }).map(|result| {
                        ()
                    })
                });

                let tx_worker = pool.spawn_fn(move || {
                    info!("\nTransmitting worker is live\n");
                    message_receiver.map(|message| {              
                        info!("\nSending a message\n");
                        OwnedMessage::Text(message)
                    })
                    .map_err(|_| { websocket::WebSocketError::NoDataAvailable })
                    .forward(tx)
                });

                let heartbeat_worker = pool.spawn_fn(move || {
                    loop_fn((heartbeat_trigger_sender, heartbeat_control_receiver), move |(trigger_channel, control)| {
                        if control.try_recv().is_err() {
                            let trigger_channel_next = trigger_channel.clone();

                            info!("\nSending heartbeat...\n");
                            trigger_channel.send("trigger").wait();

                            thread::sleep(time::Duration::from_secs(5));
                            Ok(Loop::Continue((trigger_channel_next, control)))
                        } else {
                            Ok(Loop::Break(()))
                        }
                    })
                });

                let heartbeat_sender_worker = pool.spawn_fn(move || { //required as LoopFn does not unlock the mutex
                    let queue_ref = queue_ref2;
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
                        websocket::WebSocketError::NoDataAvailable
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

    fn handle_stream(message: websocket::OwnedMessage, seq_num: Arc<AtomicUsize>, handle_message: Arc<Mutex<F>>, http: Arc<Mutex<reqwest::Client>>, token: String, user_id: String) {
        let handler = handle_message.lock().expect("Could not get lock on message handling closure.");
        let http = http.lock().expect("Could not get lock on HTTP client.");

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
                            info!("\nServer: READY\n {:?} \n", deserialized_data);
                        }
                        "MESSAGE_CREATE" => {
                            let deserialized_data: gateway::discord_message::DiscordMessage = serde_json::from_value(serialized_data)
                                                                                                        .expect("Could not parse JSON.");
                            info!("\nServer: MESSAGE_CREATE\n {:?} \n", deserialized_data);

                            if deserialized_data.author.id != user_id {
                                let channel_id = deserialized_data.channel_id;
                                let handled_response = handler(&*deserialized_data.content);

                                if let Some(response) = handled_response {
                                    let mut channels_api_url = String::from("https://discordapp.com/api/channels/");
                                    channels_api_url.push_str(&channel_id);
                                    channels_api_url.push_str("/messages");
                                    let channels_api_url_str = &*channels_api_url;

                                    let mut json_data = BTreeMap::new();
                                    json_data.insert("content", response);

                                    let mut auth_header = String::from("Bot ");
                                    auth_header.push_str(&*token);
                                    let mut req_headers = Headers::new();
                                    req_headers.set(Authorization(auth_header));

                                    let res = http.post(channels_api_url_str)
                                                    .expect("Could not start POST request.")
                                                    .headers(req_headers)
                                                    .json(&json_data)
                                                    .expect("Could not parse JSON request.")
                                                    .send();
                                    info!("Discord API Response: {:?}", res);
                                }
                            }
                        }
                        _ => {
                            info!("\nUnknown server message\n {} \n", t);
                            info!("\nUnknown server message\n {:?} \n", serialized_data);
                        }
                    }
                    
                }
                None => {
                    match opcode {
                        10 => {
                            let deserialized_data: gateway::hello::Hello = serde_json::from_value(serialized_data)
                                                                              .expect("Could not parse JSON.");
                            info!("\nServer: HELLO:\n {:?} \n", deserialized_data);
                        }
                        11 => {
                            info!("\nServer: Heartbeat acknowledged\n");
                        }
                        _ => {
                            info!("\nUnknown opcode\n");
                        }
                    }
                }
            }
        } else {
            info!("\nUnknown server message format\n");
        }
    }
}
