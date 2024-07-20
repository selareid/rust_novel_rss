mod feed_handler;
mod novel_structs;

use crate::feed_handler::handle_feed_request;
use crate::novel_structs::Config;


fn main() {
    let config = Config::new("data".to_string(), "readings.data".to_string(), "stories.conf".to_string());

    rouille::start_server("0.0.0.0:2345", move |request| {
        println!("got a request {:?}", request);

        let response = handle_feed_request(&config, "reading_orv".to_string());

        response
    });
}
