use chrono;
use reqwest;
use std::collections::HashMap;
use xml::reader::{ParserConfig, XmlEvent};

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
    pub_date:   chrono::DateTime<chrono::UTC>,
    downloaded: Option<chrono::DateTime<chrono::UTC>>,
    listened:   Option<chrono::DateTime<chrono::UTC>>,
}

#[derive(Debug)]
pub struct Podcast {
    title:        String,
    url:          String,
    episodes:     Vec<Episode>,
    last_checked: chrono::DateTime<chrono::UTC>,
}

fn new_episode(title: &str, url: &str, pub_date: chrono::DateTime<chrono::UTC>) -> Episode {
    Episode {
        title: title.to_string(),
        url: url.to_string(),
        pub_date: pub_date,
        downloaded: None,
        listened: None,
    }
}

fn str_to_date(s: &str) -> chrono::DateTime<chrono::UTC> {
    chrono::DateTime::parse_from_rfc2822(s)
        .expect(&format!("That was not the correct type of date: {:?}", s))
        .with_timezone(&chrono::UTC)
}

/// Get the RSS file associated with an URL and turn it into a Podcast.
pub fn get_rss(url: &str) -> Result<Podcast, String> {
    let client   = reqwest::Client::new().unwrap();
    let response = client.get(url).send().unwrap();
    if response.status().is_success() {
        let mut current_state = States::Other;
        let mut part_hash     = HashMap::new();
        let mut episodes      = Vec::new();
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
                                episodes.push(new_episode(
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
        Ok(Podcast {
            title:        pc_title.to_string(),
            url:          url.to_string(),
            episodes:     episodes,
            last_checked: chrono::UTC::now()
        })
    } else {
        Err(format!("We got a strange status: {:?} when fetching URL: {:?}", response.status(), url))
    }
}
