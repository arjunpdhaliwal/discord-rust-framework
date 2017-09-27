#[cfg(test)]
mod tests {
    extern crate env_logger;

    #[test]
    fn initialization_test() {
        env_logger::init().unwrap();
        ::Client::new(String::from("INSERT DISCORD BOT TOKEN HERE"), |message| {
            match message {
                "oh" => Some(String::from("ok")),
                _ => None
            }
        });
    }
}
