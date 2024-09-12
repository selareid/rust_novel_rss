use rss::{Channel, ChannelBuilder};
use crate::novel_structs::{Config, Story, Reading, ReadingId};

// todo - make more resilient to errors (using a lot of unwrap atm)
pub fn handle_feed_request(config: &Config, reading_id: ReadingId) -> rouille::Response {

    let reading = Reading::get_reading(&config.get_path_to_readings(), reading_id);
    let mut reading: Reading = match reading {
        Ok(r) => r,
        Err(e) => return e,
    };

    let story: Story = Story::get_story(&config.get_path_to_stories(), &reading.story_id).unwrap();

    println!("{:?}", reading);

    if reading.needs_update() && story.next_chapter_exists(reading.current_chapter) {
        reading.increment_current_chapter(&config.get_path_to_readings());
    }

    // Generate RSS feed
    let channel: Channel = ChannelBuilder::default()
        .title(&story.title)
        .description(format!("frequency: {}, {} chapters per update, id: {}", reading.frequency, reading.chapters_per_update, reading.id)) //todo get desc from function reading.get_desc()
        .items(story.get_rss_items(reading.current_chapter, reading.start_chapter))
        .build();

    // return response
    rouille::Response::text(channel.to_string())
}