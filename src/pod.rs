use chrono;
use reqwest;
use std::collections::HashMap;
use xml::reader::{ParserConfig, XmlEvent};

#[derive(Clone, Copy)]
enum States {
    ParsingItem,
    ParsingTitle,
    ParsingPubDate,
    Other
}

#[derive(Debug)]
pub struct Podcast {
    title:      String,
    url:        String,
    pub_date:   chrono::DateTime<chrono::FixedOffset>,
    downloaded: Option<chrono::DateTime<chrono::UTC>>,
}

fn str_to_date(s: &str) -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc2822(s).expect(&format!("That was not the correct type of date: {:?}", s))
}

/// Get the RSS file associated with an URL and turn it into a Vec of media files.
pub fn get_rss(url: &str) -> Result<Vec<Podcast>, String> {
    let client   = reqwest::Client::new().unwrap();
    let response = client.get(url).send().unwrap();
    if response.status().is_success() {
        let mut current_state = States::Other;
        let mut part_hash     = HashMap::new();
        let mut result        = Vec::new();
        let reader = ParserConfig::new()
            .trim_whitespace(true)
            .whitespace_to_characters(true)
            .ignore_comments(true)
            .create_reader(response);
        for e in reader.into_iter() {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                    match name.local_name.to_lowercase().as_ref() {
                        "item"      => current_state = States::ParsingItem,
                        "enclosure" => {
                            part_hash.insert("url", attributes
                                .iter()
                                .find(|a| a.name.local_name == "url")
                                .map(|a| a.value.to_owned())
                                .expect("I couldn't find an URL in this enclosure."));
                        }
                        "title"     => {
                            // We don't want the title of the podcast, only the title
                            // of each episode.
                            match current_state {
                                States::ParsingItem => current_state = States::ParsingTitle,
                                _                   => ()
                            }
                        }
                        "pubdate"   => current_state = States::ParsingPubDate,
                        _ => ()
                    }
                }
                Ok(XmlEvent::EndElement { name }) => {
                    match (name.local_name.to_lowercase().as_ref(), current_state) {
                        ("item", States::ParsingItem)       => {
                            println!("This is the current part_hash: {:?}", part_hash);
                            { // Scope cheat to let me use the closure getter.
                                let getter = |key| part_hash.get(key).unwrap();
                                result.push(Podcast {
                                        title:      getter("title").to_string(),
                                        url:        getter("url").to_string(),
                                        pub_date:   str_to_date(getter("pub_date")),
                                        downloaded: None,
                                });
                            }
                            part_hash.clear();
                            current_state = States::Other;
                        }
                        ("title", States::ParsingTitle)     => current_state = States::ParsingItem,
                        ("pubdate", States::ParsingPubDate) => current_state = States::ParsingItem,
                        _ => ()
                    }
                }
                Ok(XmlEvent::Characters(s)) => {
                    match current_state {
                        States::ParsingTitle   => { part_hash.insert("title",    s); },
                        States::ParsingPubDate => { part_hash.insert("pub_date", s); },
                        _                      => (),
                    }
                }
                Err(e) => println!("Error: {}", e),
                _      => ()
            }
        }
        Ok(result)
    } else {
        Err(format!("We got a strange status: {:?} when fetching URL: {:?}", response.status(), url))
    }
}
