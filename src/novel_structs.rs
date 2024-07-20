use std::fs;
use chrono::Utc;
use rss::{Item, ItemBuilder};

fn get_days_since_epoch() -> u64 {
    const SECONDS_PER_DAY: u64 = 86400;
    let now = Utc::now();
    now.timestamp() as u64 / SECONDS_PER_DAY
}

pub(crate) struct Config {
    directory: String,
    filename_readings: String,
    filename_stories: String
}

impl Config {
    pub(crate) fn new(directory: String, filename_readings: String, filename_stories: String) -> Self {
        Config { directory, filename_readings, filename_stories }
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
    leading_zeros: usize
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

    pub(crate) fn get_rss_items(&self, max_chapter: u64) -> Vec<rss::Item> {
        let mut items: Vec<rss::Item> = Vec::new();

        // todo - remove hard coded orv stuff
        // this one adds intro to the feed
        if self.id == "orv" {
            let intro_item: Item = ItemBuilder::default()
                .title("Omniscient Reader Viewpoint Intro".to_string())
                .link("http://read.selareid.moe/stories/omniscient_readers_viewpoint/intro.xhtml".to_string())
                .build();
            items.push(intro_item);
        }

        for chapter_i in 1..max_chapter+1 {
            let item: Item = ItemBuilder::default()
                .title(format!("{} Chapter {}", self.title, chapter_i))
                .link(self.get_chapter_url(chapter_i))
                .build();

            items.push(item);
        }

        items
    }

    pub(crate) fn _new(id: String, title: String, url: String, leading_zeros: usize) -> Self {
        Story {id, title, url, leading_zeros }
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

                    assert!(matches!(sections.next(), None));

                    return Ok(Story {
                        id: desired_story_id.to_string(),
                        title: title.to_string(),
                        url: url.to_string(),
                        leading_zeros: leading_zeros.parse()?
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
    pub(crate) current_chapter: u64,
    last_update: u64
}

impl Reading {
    pub(crate) fn needs_update(&self) -> bool {
        get_days_since_epoch() >= self.last_update + self.frequency
    }

    pub(crate) fn increment_current_chapter(&mut self, path_to_readings: &String) {
        self.current_chapter += 1;
        self.last_update = get_days_since_epoch();
        self.update_reading_on_disk(&path_to_readings);
    }

    pub(crate) fn get_reading(readings_path: &String, desired_reading_id: ReadingId) -> Result<Self, Box<dyn std::error::Error>> {
        for line in fs::read_to_string(readings_path)?.lines() {
            let mut sections = line.split_whitespace();

            if let Some(current_reading_id) = sections.next() {
                if desired_reading_id == current_reading_id {
                    let story_id = sections.next().unwrap();
                    let frequency = sections.next().unwrap();
                    let current_chapter = sections.next().unwrap();
                    let last_update = sections.next().unwrap();

                    assert!(matches!(sections.next(), None));

                    return Ok(Reading {
                        id: current_reading_id.to_string(),
                        story_id: story_id.to_string(),
                        frequency: frequency.parse()?,
                        current_chapter: current_chapter.parse()?,
                        last_update: last_update.parse()?,
                    })
                }
            }
            else { panic!("Issue with readings file. Could not get reading id"); }
        }

        Err(Box::from(format!("Failed to find reading with id {}", desired_reading_id)))
    }

    fn update_reading_on_disk(&self, path: &String) {
        let mut readings_file_string = String::new();
        let mut firstline = true;

        for line in fs::read_to_string(&path).unwrap().lines() {
            let mut sections = line.split_whitespace();

            if let Some(current_reading_id) = sections.next() {
                if self.id == current_reading_id { // we have the current story
                    if !firstline { readings_file_string.push_str("\n"); }
                    readings_file_string.push_str(&format!("{} {} {} {} {}", self.id, self.story_id, self.frequency, self.current_chapter, self.last_update));

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