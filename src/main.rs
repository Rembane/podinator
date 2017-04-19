#![recursion_limit = "1024"]

extern crate chrono;
#[macro_use] extern crate clap;
extern crate core;
#[macro_use] extern crate error_chain;
extern crate itertools;
extern crate quick_xml;
extern crate reqwest;
extern crate rmp_serde as rmps;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate toml;

mod database;
mod errors;
mod pod;

use std::fs::{File};
use std::io::{Read};
use std::path::{Path};

use database::Database;
use errors::*;

#[derive(Debug, Deserialize)]
struct Config {
    db_path: String,
    podcast_path: String,
}

fn run() -> Result<()> {
    let matches = clap_app!(app =>
        (version: crate_version!())
        (author: crate_authors!(", "))
        (about: crate_description!())
        (@arg CONFIG: -c --config +takes_value "Set the path to config file, otherwise podinator.toml will be used.")
        (@arg DBPATH: -d --dbpath +takes_value "Set the path to the database file.")
        (@arg PODPATH: -p --podpath +takes_value "Set the path to the directory where the podcasts are stored.")
        (@subcommand add =>
            (about: "Add a podcast.")
            (@arg URL: +required "Add a podcast by supplying a URL.")
        )
        (@subcommand list =>
            (about: "List all podcasts.")
        )
        (@subcommand download =>
            (about: "Download all podcasts that haven't been downloaded yet.")
        )
    ).get_matches();

    let mut config: Config = match File::open(matches.value_of("CONFIG").unwrap_or("podinator.toml")) {
        Ok(mut fh) => {
            let mut s = String::new();
            fh.read_to_string(&mut s)?;
            toml::from_str(&mut s)?
        }
        Err(_) => { // Default configuration.
            Config {
                db_path: String::from("podcasts.db"),
                podcast_path: String::from("podcasts/")
            }
        }
    };

    match matches.value_of("DBPATH") {
        Some(p) => { config.db_path = String::from(p); }
        _       => ()
    }
    match matches.value_of("PODPATH") {
        Some(p) => { config.podcast_path = String::from(p); }
        _       => ()
    }

    let db_path = Path::new(&config.db_path);
    // If the database file doesn't exist, we create a new database.
    let mut db = match Database::from_file(db_path) {
        Ok(d) => d,
        Err(_) => Database::new(),
    };

    match matches.subcommand() {
        ("add", Some(matches)) => {
            let url = matches.value_of("URL").ok_or("Please supply a URL.")?;
            db.add(url);
            db.to_file(db_path).chain_err(|| "Writing to database failed.")?;
        }
        ("list", Some(_)) => {
            for p in db.into_iter() {
                println!("{:?}", p);
            }
        }
        ("download", Some(_)) => {
            db.download(Path::new(&config.podcast_path)).chain_err(|| "Tried to download podcast, world went boom.")?;
            db.to_file(db_path).chain_err(|| "Tried to save podcast database to file. Something failed.")?;
        }
        _ => {},
    }
    Ok(())
}

fn main() {
    // Error handling boilerplate.
    if let Err(ref e) = run() {
        println!("Error: {}", e);
        for e in e.iter().skip(1) {
            println!("Caused by: {}", e);
        }
        if let Some(backtrace) = e.backtrace() {
            println!("Backtrace: {:?}", backtrace);
        }
        std::process::exit(1);
    }
}
