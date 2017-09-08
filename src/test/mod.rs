#[cfg(test)]
mod tests {
    extern crate env_logger;

    #[test]
    fn initialization_test() {
        env_logger::init().unwrap();
        ::Client::new(String::from("MzQ5OTEyMTE1OTYxNzkwNDY0.DH8YkQ.sUB4M6AlSJAZapL3f8lQyyE7SOg"), |message| {
            match message {
                "oh" => Some(String::from("ok")),
                _ => None
            }
        });
    }
}
