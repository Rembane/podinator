//// This module keeps track of all podcasts and their episodes.

use rmps::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::fs::{File};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::vec::IntoIter;

use errors::*;
use pod::{Episode, Podcast};

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
}

impl IntoIterator for Database {
    type Item = Podcast;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
