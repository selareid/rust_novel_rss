mod feed_handler;
mod novel_structs;

use rouille::router;
use crate::feed_handler::handle_feed_request;
use crate::novel_structs::Config;


fn main() {
    let config = Config::new("2345".to_string(), "data".to_string(), "readings.data".to_string(), "stories.conf".to_string());

    println!("Starting server at {}", config.web_addr());

    rouille::start_server(config.web_addr(), move |request| {
        println!("got a request {:?}", request);

        router!(request,
            (GET) (/feeds/{reading_id: String}) => {
                handle_feed_request(&config, reading_id)
            },

            _ => rouille::Response::text("Error 404").with_status_code(404)
        )
    });
}
