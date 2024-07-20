use rss::{Channel, ChannelBuilder};
use rouille::Response;
use crate::novel_structs::{Config, Story, Reading, ReadingId};

// todo - make more resilient to errors (using a lot of unwrap atm)
pub fn handle_feed_request(config: &Config, reading_id: ReadingId) -> Response {
    let mut reading: Reading = Reading::get_reading(&config.get_path_to_readings(), reading_id).unwrap();
    let story: Story = Story::get_story(&config.get_path_to_stories(), &reading.story_id).unwrap();

    println!("{:?}", reading);

    if reading.needs_update() && story.next_chapter_exists(reading.current_chapter) {
        reading.increment_current_chapter(&config.get_path_to_readings());
    }

    // Generate RSS feed
    let channel: Channel = ChannelBuilder::default()
        .title(&story.title)
        .description(format!("frequency: {}, id: {}", reading.frequency, reading.id))
        .items(story.get_rss_items(reading.current_chapter))
        .build();

    // return response
    Response::text(channel.to_string())
}