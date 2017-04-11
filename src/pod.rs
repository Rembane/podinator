use chrono::{ DateTime, TimeZone, UTC };
use reqwest;
use std::collections::HashMap;
use std::fs::{ File, create_dir_all };
use std::io::{ Read, Write };
use std::path::Path;
use xml::reader::{ ParserConfig, XmlEvent };

#[derive(Clone, Copy)]
enum States {
    ParsingPodcastTitle,
    ParsingItem,
    ParsingTitle,
    ParsingPubDate,
    Other
}

#[derive(Debug)]
pub struct Episode {
    title:      String,
    url:        String,
    pub_date:   DateTime<UTC>,
    downloaded: Option<DateTime<UTC>>,
    listened:   Option<DateTime<UTC>>,
}

#[derive(Debug)]
pub struct Podcast {
    title:        String,
    url:          String,
    episodes:     Vec<Episode>,
    last_checked: DateTime<UTC>,
}

impl Podcast {
    pub fn new(title: &str, url: &str) -> Podcast {
        Podcast {
            title:    title.to_string(),
            url:      url.to_string(),
            episodes: Vec::new(),
            // Epoch
            last_checked: UTC.timestamp(0, 0),
        }
    }

    /// Get the RSS file associated with an URL and update this podcast accordingly.
    pub fn get_rss(&mut self) -> Result<(), String> {
        let client   = reqwest::Client::new().unwrap();
        let response = client.get(&self.url).send().unwrap();
        if response.status().is_success() {
            let mut current_state = States::Other;
            let mut part_hash     = HashMap::new();
            let mut pc_title      = String::new();
            let reader = ParserConfig::new()
                .trim_whitespace(true)
                .whitespace_to_characters(true)
                .ignore_comments(true)
                .create_reader(response);
            for e in reader.into_iter() {
                match e {
                    Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                        match (name.local_name.to_lowercase().as_ref(), current_state) {
                            ("item",      States::Other)       => current_state = States::ParsingItem,
                            ("enclosure", States::ParsingItem) => {
                                part_hash.insert("url", attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "url")
                                    .map(|a| a.value.to_owned())
                                    .expect("I couldn't find an URL in this enclosure."));
                            }
                            ("title", States::ParsingItem) =>
                                current_state = States::ParsingTitle,

                            ("title", States::Other) =>
                                current_state = States::ParsingPodcastTitle,

                            ("pubdate", States::ParsingItem) =>
                                current_state = States::ParsingPubDate,
                            _ => ()
                        }
                    }
                    Ok(XmlEvent::EndElement { name }) => {
                        match (name.local_name.to_lowercase().as_ref(), current_state) {
                            ("item", States::ParsingItem)       => {
                                println!("This is the current part_hash: {:?}", part_hash);
                                { // Scope cheat to let me use the closure getter.
                                    let getter = |key| part_hash.get(key).unwrap();
                                    self.episodes.push(new_episode(
                                            getter("title"),
                                            getter("url"),
                                            str_to_date(getter("pub_date")),
                                    ));
                                }
                                part_hash.clear();
                                current_state = States::Other;
                            }
                            ("title", States::ParsingTitle)        => current_state = States::ParsingItem,
                            ("title", States::ParsingPodcastTitle) => current_state = States::Other,
                            ("pubdate", States::ParsingPubDate)    => current_state = States::ParsingItem,
                            _ => ()
                        }
                    }
                    Ok(XmlEvent::Characters(s)) => {
                        match current_state {
                            States::ParsingTitle   => {
                                part_hash.insert("title",    s);
                            },
                            States::ParsingPubDate => {
                                part_hash.insert("pub_date", s);
                            },
                            States::ParsingPodcastTitle => {
                                pc_title = s;
                            }
                            _                      => (),
                        }
                    }
                    Err(e) => println!("Error: {}", e),
                    _      => ()
                }
            }
            self.title        = pc_title.to_string();
            self.last_checked = UTC::now();
            Ok(())
        } else {
            Err(format!("We got a strange status: {:?} when fetching URL: {:?}", response.status(), &self.url))
        }
    }

    /// Download all episodes not already downloaded.
    pub fn download(&mut self) {
        println!("Downloading podcast: {:?}", self.title);
        let p = Path::new(&self.title);
        for e in self.episodes.iter_mut() {
            e.download(p);
        }
    }
}

impl Episode {
    /// Download this episode if it hasn't already been downloaded.
    pub fn download(&mut self, base: &Path) {
        if self.downloaded.is_none() {
            let file_name = self.url
                .rsplit("/")
                .next()
                .expect(&format!("Your URL doesn't contain any slashes, strange, eh? {:?}", self.url));
            println!("Downloading: {:?} from {:?}", self.title, self.url);
            let mut web = reqwest::get(&self.url)
                .expect(&format!("Couldn't find url: {:?}", self.url));
            create_dir_all(base).unwrap();
            let p        = base.join(Path::new(file_name));
            let mut f    = File::create(&p).expect(&format!("Unable to create file with path: {:?}", &p));
            let mut buf = Vec::new();
            web.read_to_end(&mut buf).unwrap();
            f.write_all(&buf).unwrap();
            self.downloaded = Some(UTC::now());
        } else {
            println!("This episode has already been downloaded. {:?}", self.title);
        }
    }
}

fn new_episode(title: &str, url: &str, pub_date: DateTime<UTC>) -> Episode {
    Episode {
        title: title.to_string(),
        url: url.to_string(),
        pub_date: pub_date,
        downloaded: None,
        listened: None,
    }
}

fn str_to_date(s: &str) -> DateTime<UTC> {
    DateTime::parse_from_rfc2822(s)
        .expect(&format!("That was not the correct type of date: {:?}", s))
        .with_timezone(&UTC)
}

