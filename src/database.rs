//// This module keeps track of all podcasts and their episodes.

use rmps::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::fs::{File};
use std::path::Path;

use errors::*;
use pod::{Podcast};

#[derive(Debug, Deserialize, Serialize)]
pub struct Database(Vec<Podcast>);

impl Database {
    /// Create a new, empty database.
    pub fn new() -> Database {
        Database(Vec::new())
    }

    /// Save database to file.
    /// TODO: If something crashes while we do this the old file gets deleted.
    ///       Can we create a new temp file and do a `mv` when done instead?
    pub fn to_file(&self, path: &Path) -> Result<()> {
        let mut fh = File::create(path).chain_err(|| "Couldn't create file.")?;
        let mut s = Serializer::new(&mut fh);
        self.serialize(&mut s).chain_err(|| "Couldn't serialize the database.")
    }

    /// Load database from file.
    pub fn from_file(path: &Path) -> Result<Database> {
        let fh = File::open(path).chain_err(|| "The file doesn't exist!")?;
        let mut d = Deserializer::from_read(fh);
        Deserialize::deserialize(&mut d).chain_err(|| "Couldn't deserialize the database.")
    }

    /// Create a new podcast in the database, by supplying this function with an URL.
    pub fn add(&mut self, url: &str) {
        self.0.push(Podcast::new(" ", url));
    }

    /// Download all the podcasts in the database.
    pub fn download(&mut self, pod_path: &Path) -> Result<()> {
        for mut p in &mut self.0 {
            p.get_rss().chain_err(|| "Downloading RSS failed horribly.")?;
            println!("Downloading podcast: {:?}", p.title);
            let podcast_title = p.title.clone();
            let path = pod_path.join(Path::new(&podcast_title));
            for mut e in p {
                e.download(&path, &podcast_title).chain_err(|| "Podcast download failed.")?;
            }
        }
        Ok(())
    }

    /// Delete all episodes.
    pub fn clear_episodes(&mut self) {
        for mut p in &mut self.0 {
            p.clear_episodes();
        }
    }
}

impl IntoIterator for Database {
    type Item = Podcast;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
