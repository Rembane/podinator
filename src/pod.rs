use chrono::{DateTime, TimeZone, UTC};
use core::slice;
use reqwest;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::fs::{File, create_dir_all, remove_file};
use std::io::{BufReader, Read, Write};
use std::path::Path;

use errors::*;

#[derive(Clone, Copy)]
enum States {
    ParsingPodcastTitle,
    ParsingItem,
    ParsingTitle,
    ParsingPubDate,
    Other,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Episode {
    title: String,
    url: String,
    pub_date: DateTime<UTC>,
    downloaded: Option<DateTime<UTC>>,
    listened: Option<DateTime<UTC>>,
    local_file_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Podcast {
    pub title: String,
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
    pub fn get_rss(&mut self) -> Result<()> {
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
                        if let ("enclosure", States::ParsingItem) = (reader.decode(e.name()).to_lowercase().as_str(), current_state) {
                            part_hash.insert("url", e.attributes()
                                             .map(|a| a.unwrap())
                                             .find(|a| a.key == b"url")
                                             .map(|a| reader.decode(a.unescaped_value().unwrap().borrow()).into_owned())
                                             .expect("I couldn't find an URL in this enclosure."));
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
                                self.episodes
                                    .push(Episode::new(&part_hash["title"],
                                                      &part_hash["url"],
                                                      str_to_date(&part_hash["pub_date"])));
                                part_hash.clear();
                                current_state = States::Other;
                            }
                            ("title", States::ParsingTitle)
                                | ("pubdate", States::ParsingPubDate) => current_state = States::ParsingItem,
                            ("title", States::ParsingPodcastTitle) => current_state = States::Other,
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
            bail!(format!("We got a strange status: {:?} when fetching URL: {:?}",
                        response.status(),
                        &self.url))
        }
    }

    pub fn clear_episodes(&mut self) {
        println!("Clearing episodes.");
        for e in &self.episodes {
            if let Some(ref f) = e.local_file_name {
                println!("Deleting file: {:?}", f);
                remove_file(Path::new(&f)).unwrap();
            }
        }
        self.episodes.clear();
    }
}

impl Display for Podcast {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
        write!(f, "{}\n{} {}\n", self.title, self.url, self.last_checked)?;
        for e in &self.episodes {
            e.fmt(f)?;
        }
        Ok(())
    }
}

impl IntoIterator for Podcast {
    type Item = Episode;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.episodes.into_iter()
    }
}

impl<'a> IntoIterator for &'a mut Podcast {
    type Item = &'a mut Episode;
    type IntoIter = slice::IterMut<'a, Episode>;

    fn into_iter(mut self) -> Self::IntoIter {
        self.episodes.iter_mut()
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
    pub fn download(&mut self, base: &Path, podcast_title: &str) -> Result<()> {
        if self.downloaded.is_none() {
            let file_name = match self.url.rsplit('/').next() {
                Some(s) => format!("{}_{}_{}", podcast_title, self.pub_date.format("%FT%R").to_string(), s),
                None => bail!(format!("Your URL doesn't contain any slashes, strange, eh? {:?}", self.url)),
            };
            println!("Downloading: {:?} from {:?}", self.title, self.url);
            let mut web = reqwest::get(&self.url).chain_err(|| format!("Got error when trying to download url: {:?}", self.url))?;
            create_dir_all(base)?;
            let mut f = File::create(&base.join(Path::new(&file_name)))?;
            let mut buf = Vec::new();
            web.read_to_end(&mut buf)?;
            f.write_all(&buf)?;
            self.downloaded = Some(UTC::now());
            self.local_file_name = Some(file_name);
        } else {
            println!("This episode has already been downloaded: {:?}", self.title);
        }
        Ok(())
    }
}

fn option_date_fmt(x: Option<DateTime<UTC>>, f: &mut fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
     match x {
        None => write!(f, ""),
        Some(d) => write!(f, "{}", d.format("%FT%R").to_string())
    }
}

impl Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::core::result::Result<(), ::core::fmt::Error> {
        write!(f, "    {}\n", self.title)?;
        option_date_fmt(Some(self.pub_date), f)?;
        write!(f, "\t")?;
        option_date_fmt(self.downloaded, f)?;
        write!(f, "\t")?;
        option_date_fmt(self.listened, f)?;
        write!(f, "\n")
    }
}

fn str_to_date(s: &str) -> DateTime<UTC> {
    DateTime::parse_from_rfc2822(s)
        .expect(&format!("That was not the correct type of date: {:?}", s))
        .with_timezone(&UTC)
}
