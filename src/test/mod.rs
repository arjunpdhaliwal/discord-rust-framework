#[cfg(test)]
mod tests {
    #[test]
    fn initialization_test() {
        ::Client::new();
    }

    #[test]
    fn authentication_test() {
        let mut client = ::Client::new();
        client.authenticate(String::from("MzQ5OTEyMTE1OTYxNzkwNDY0.DH8YkQ.sUB4M6AlSJAZapL3f8lQyyE7SOg")); //dummy discord account for testing
    }
}
