use chrono::{DateTime, TimeZone, UTC};
use reqwest;
use itertools::Itertools;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::{File, create_dir_all};
use std::io::{BufReader, Read, Write};
use std::path::Path;

#[derive(Clone, Copy)]
enum States {
    ParsingPodcastTitle,
    ParsingItem,
    ParsingTitle,
    ParsingPubDate,
    Other,
}

#[derive(Debug)]
pub struct Episode {
    title: String,
    url: String,
    pub_date: DateTime<UTC>,
    downloaded: Option<DateTime<UTC>>,
    listened: Option<DateTime<UTC>>,
    local_file_name: Option<String>,
}

#[derive(Debug)]
pub struct Podcast {
    title: String,
    url: String,
    episodes: Vec<Episode>,
    last_checked: DateTime<UTC>,
}

impl Podcast {
    pub fn new(title: &str, url: &str) -> Podcast {
        Podcast {
            title: title.to_string(),
            url: url.to_string(),
            episodes: Vec::new(),
            // Epoch
            last_checked: UTC.timestamp(0, 0),
        }
    }

    /// Get the RSS file associated with an URL and update this podcast accordingly.
    pub fn get_rss(&mut self) -> Result<(), String> {
        let client = reqwest::Client::new().unwrap();
        let response = client.get(&self.url).send().unwrap();
        if response.status().is_success() {
            let mut current_state = States::Other;
            let mut part_hash = HashMap::new();
            let mut pc_title = String::new();
            let mut reader = Reader::from_reader(BufReader::new(response));
            reader.trim_text(true);
            let mut buf = Vec::new();
            loop {
                match reader.read_event(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        match (reader.decode(e.name()).to_lowercase().as_str(), current_state) {
                            ("item", States::Other) => current_state = States::ParsingItem,
                            ("title", States::ParsingItem) => current_state = States::ParsingTitle,
                            ("title", States::Other) => current_state = States::ParsingPodcastTitle,
                            ("pubdate", States::ParsingItem) => current_state = States::ParsingPubDate,
                            _ => (),
                        }
                    }
                    Ok(Event::Empty(ref e)) => {
                        match (reader.decode(e.name()).to_lowercase().as_str(), current_state) {
                            ("enclosure", States::ParsingItem) => {
                                part_hash.insert("url", e.attributes()
                                                 .map(|a| a.unwrap())
                                                 .find(|a| a.key == b"url")
                                                 .map(|a| reader.decode(a.unescaped_value().unwrap().borrow()).into_owned())
                                                 .expect("I couldn't find an URL in this enclosure."));
                            }
                            _ => (),
                        }
                    }
                    Ok(Event::Text(e)) => {
                        let txt = e.unescape_and_decode(&reader).unwrap();
                        match current_state {
                            States::ParsingTitle => { part_hash.insert("title", txt); },
                            States::ParsingPubDate => { part_hash.insert("pub_date", txt); },
                            States::ParsingPodcastTitle => { pc_title = txt; },
                            _ => (),
                        }
                    }
                    Ok(Event::End(e)) => {
                        match (reader.decode(e.name()).to_lowercase().as_str(), current_state) {
                            ("item", States::ParsingItem) => {
                                println!("This is the current part_hash: {:?}", part_hash);
                                {
                                    // Scope cheat to let me use the closure getter.
                                    let getter = |key| part_hash.get(key).unwrap();
                                    self.episodes
                                        .push(Episode::new(getter("title"),
                                                          getter("url"),
                                                          str_to_date(getter("pub_date"))));
                                }
                                part_hash.clear();
                                current_state = States::Other;
                            }
                            ("title", States::ParsingTitle) => current_state = States::ParsingItem,
                            ("title", States::ParsingPodcastTitle) => current_state = States::Other,
                            ("pubdate", States::ParsingPubDate) => current_state = States::ParsingItem,
                            _ => (),
                        }
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => println!("Error at position: {} {:?}", reader.buffer_position(), e),
                    _ => (),
                }
                buf.clear();
            }
            self.title = pc_title.to_string();
            self.last_checked = UTC::now();
            Ok(())
        } else {
            Err(format!("We got a strange status: {:?} when fetching URL: {:?}",
                        response.status(),
                        &self.url))
        }
    }

    /// Download all episodes not already downloaded.
    pub fn download(&mut self) {
        println!("Downloading podcast: {:?}", self.title);
        let p = Path::new(&self.title);
        for e in self.episodes.iter_mut() {
            e.download(p, &self.title);
        }
    }
}

impl Episode {
    fn new(title: &str, url: &str, pub_date: DateTime<UTC>) -> Episode {
        Episode {
            title: title.to_string(),
            url: url.to_string(),
            pub_date: pub_date,
            downloaded: None,
            listened: None,
            local_file_name: None,
        }
    }

    /// Download this episode if it hasn't already been downloaded.
    pub fn download(&mut self, base: &Path, podcast_title: &str) {
        if self.downloaded.is_none() {
            let file_name = [
                podcast_title,
                &self.pub_date.format("%FT%R").to_string(),
                self.url
                    .rsplit("/")
                    .next()
                    .expect(&format!("Your URL doesn't contain any slashes, strange, eh? {:?}",
                                     self.url))
            ].into_iter().join("_");

            println!("Downloading: {:?} from {:?}", self.title, self.url);
            let mut web = reqwest::get(&self.url).expect(&format!("Couldn't find url: {:?}",
                                                                  self.url));
            create_dir_all(base).unwrap();
            let p = base.join(Path::new(&file_name));
            let mut f = File::create(&p).expect(&format!("Unable to create file with path: {:?}",
                                                         &p));
            let mut buf = Vec::with_capacity(1024 * 1024); // One MB chunks!
            loop {
                match web.read(&mut buf) {
                    Ok(0) => break,
                    Ok(_) => {
                        f.write(&buf).unwrap();
                    }
                    Err(_) => break,
                }
            }
            self.downloaded = Some(UTC::now());
            self.local_file_name = Some(file_name);
        } else {
            println!("This episode has already been downloaded. {:?}", self.title);
        }
    }
}

fn str_to_date(s: &str) -> DateTime<UTC> {
    DateTime::parse_from_rfc2822(s)
        .expect(&format!("That was not the correct type of date: {:?}", s))
        .with_timezone(&UTC)
}
