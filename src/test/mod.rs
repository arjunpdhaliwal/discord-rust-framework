#[cfg(test)]
mod tests {
    extern crate env_logger;

    #[test]
    fn initialization_test() {
        ::Client::new(String::from("MzQ5OTEyMTE1OTYxNzkwNDY0.DH8YkQ.sUB4M6AlSJAZapL3f8lQyyE7SOg"));
    }

    #[test]
    fn authentication_test() {
        env_logger::init().unwrap();
        info!("\nRunning test...\n");
        let mut client = ::Client::new(String::from("MzQ5OTEyMTE1OTYxNzkwNDY0.DH8YkQ.sUB4M6AlSJAZapL3f8lQyyE7SOg")); //dummy discord account for testing
        client.authenticate();
    }
}
