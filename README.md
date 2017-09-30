# discord-rust-framework
Framework for communication with the Discord Gateway over websockets using Rust.

### Usage:
```rs
extern crate discord_rust_framework;

fn main() {
    discord_rust_framework::Client::new(String::from(INSERT_TOKEN_HERE), |message| {
        match message {
            "ping" => Some(String::from("pong")),
            _ => None
        }
    }).connect();
}
```

Run tests with `cargo test`.
