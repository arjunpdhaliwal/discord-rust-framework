#[cfg(test)]
mod tests {
    extern crate env_logger;

    #[test]
    fn bot_test() {
        env_logger::init().unwrap();
        ::Client::new(String::from("INSERT DISCORD BOT TOKEN HERE"), |message| {
            match message {
                "ping" => Some(String::from("pong")),
                _ => None
            }
        });
    }
}
