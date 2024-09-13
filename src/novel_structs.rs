use std::fs;
use chrono::Utc;
use rss::{Item, ItemBuilder};

fn get_days_since_epoch() -> u64 {
    const SECONDS_PER_DAY: u64 = 86400;
    let now = Utc::now();
    now.timestamp() as u64 / SECONDS_PER_DAY
}

pub(crate) struct Config {
    port: String,
    directory: String,
    filename_readings: String,
    filename_stories: String
}

impl Config {
    pub(crate) fn new(port: String, directory: String, filename_readings: String, filename_stories: String) -> Self {
        Config { port, directory, filename_readings, filename_stories }
    }

    pub(crate) fn web_addr(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }

    pub(crate) fn get_path_to_readings(&self) -> String {
        format!("{}/{}", &self.directory, &self.filename_readings)
    }

    pub(crate) fn get_path_to_stories(&self) -> String {
        format!("{}/{}", &self.directory, &self.filename_stories)
    }
}

#[derive(Debug)]
pub(crate) struct Story {
    id: String,
    pub(crate) title: String,
    url: String, // e.g. example.com/orv/chap_0000%s.xhtml where %s is num
    leading_zeros: usize,
    chapter_zero_url: Option<String>, // url of intro/prologue chapter "0"
}

impl Story {
    pub(crate) fn next_chapter_exists(&self, current_chapter: u64) -> bool {
        self.is_chapter_exist(current_chapter+1)
    }

    fn is_chapter_exist(&self, chapter: u64) -> bool {
        let url = self.get_chapter_url(chapter);
        let res = reqwest::blocking::get(url).unwrap();
        let status = res.status();

        status.is_success()
    }

    fn get_chapter_url(&self, chapter: u64) -> String {
        self.url.replace("%s", &format!("{:0width$}", chapter, width=self.leading_zeros))
    }

    pub(crate) fn get_rss_items(&self, max_chapter: u64, start_chapter: u64) -> Vec<rss::Item> {
        let mut items: Vec<rss::Item> = Vec::new();

        for chapter_i in start_chapter..max_chapter+1 {
            let item: Item;

            if chapter_i == 0 {
                match &self.chapter_zero_url {
                    None => {panic!("No zero chapter url found, but zero chapter was requested!")}
                    Some(url) => {
                        item = ItemBuilder::default()
                            .title(format!("{} Chapter {}", self.title, chapter_i))
                            .link(url.to_string())
                            .build();
                    }
                }
            }
            else {
                item = ItemBuilder::default()
                    .title(format!("{} Chapter {}", self.title, chapter_i))
                    .link(self.get_chapter_url(chapter_i))
                    .build();
            }
            items.push(item);
        }

        items.reverse(); // put latest chapter at start of feed
        items
    }

    pub(crate) fn _new(id: String, title: String, url: String, leading_zeros: usize, chapter_zero_url: Option<String>) -> Self {
        Story { id: id, title, url, leading_zeros, chapter_zero_url }
    }

    pub(crate) fn get_story(stories_path: &String, desired_story_id: &String) -> Result<Self, Box<dyn std::error::Error>> {
        for line in fs::read_to_string(stories_path)?.lines() {
            let split_line = shell_words::split(line)?;
            let mut sections = split_line.iter();

            if let Some(current_story_id) = sections.next() {
                if current_story_id == desired_story_id {
                    let title = sections.next().unwrap();
                    let url = sections.next().unwrap();
                    let leading_zeros = sections.next().unwrap();
                    let chapter_zero_url = sections.next();

                    assert!(matches!(sections.next(), None));

                    return Ok(Story {
                        id: desired_story_id.to_string(),
                        title: title.to_string(),
                        url: url.to_string(),
                        leading_zeros: leading_zeros.parse()?,
                        chapter_zero_url: chapter_zero_url.filter(|s| s.len() >= 5).map(|s| s.to_string()), // minimal string length for url to be accepted
                    })
                }
            } else { panic!("Issue with stories file. Could not get story id."); }
        }

        Err(Box::from(format!("Failed to find story with id {}", desired_story_id)))
    }
}


#[derive(Debug)]
pub(crate) struct Reading {
    pub(crate) id: ReadingId,
    pub(crate) story_id: String,
    pub(crate) frequency: u64,
    pub(crate) start_chapter: u64,
    pub(crate) current_chapter: u64,
    last_update: u64,
    pub(crate) chapters_per_update: u64,
}

impl Reading {
    pub(crate) fn needs_update(&self) -> bool {
        get_days_since_epoch() >= self.last_update + self.frequency
    }

    pub(crate) fn increment_current_chapter(&mut self, path_to_readings: &String) {
        self.current_chapter += self.chapters_per_update;
        self.last_update = get_days_since_epoch();
        self.update_reading_on_disk(&path_to_readings);
    }

    pub(crate) fn get_reading(readings_path: &String, desired_reading_id: ReadingId) -> Result<Self, rouille::Response> {
        let file = match fs::read_to_string(readings_path) {
            Ok(data) => data,
            Err(e) => return Err(rouille::Response::text(e.to_string()).with_status_code(500))
        };

        for line in file.lines() {
            let mut sections = line.split_whitespace();

            if let Some(current_reading_id) = sections.next() {
                if desired_reading_id == current_reading_id {
                    let story_id = sections.next().unwrap();
                    let frequency = sections.next().unwrap();
                    let chapters_per_update = sections.next().unwrap();
                    let current_chapter = sections.next().unwrap();
                    let last_update = sections.next().unwrap();
                    let start_chapter = sections.next().unwrap_or("0");

                    assert!(matches!(sections.next(), None));

                    return Ok(Reading {
                        id: current_reading_id.to_string(),
                        story_id: story_id.to_string(),
                        frequency: frequency.parse().unwrap(),
                        chapters_per_update: chapters_per_update.parse().unwrap(),
                        current_chapter: current_chapter.parse().unwrap(),
                        last_update: last_update.parse().unwrap(),
                        start_chapter: start_chapter.parse().unwrap(),
                    })
                }
            }
            else { panic!("Issue with readings file. Could not get reading id"); }
        }

        Err(rouille::Response::text("Error 404").with_status_code(404))
    }

    fn update_reading_on_disk(&self, path: &String) {
        let mut readings_file_string = String::new();
        let mut firstline = true;

        for line in fs::read_to_string(&path).unwrap().lines() {
            let mut sections = line.split_whitespace();

            if let Some(current_reading_id) = sections.next() {
                if self.id == current_reading_id { // we have the current story
                    if !firstline { readings_file_string.push_str("\n"); }
                    readings_file_string.push_str(&format!("{} {} {} {} {} {} {}", self.id, self.story_id, self.frequency, self.chapters_per_update, self.current_chapter, self.last_update, self.start_chapter));

                    continue;
                }
            }

            if !firstline { readings_file_string.push_str("\n"); }
            readings_file_string.push_str(line);

            firstline = false;
        }

        fs::write(&path, readings_file_string).unwrap();
    }
}

pub type ReadingId = String;