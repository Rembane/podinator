#![recursion_limit = "1024"]

extern crate chrono;
#[macro_use] extern crate clap;
#[macro_use] extern crate error_chain;
extern crate itertools;
extern crate quick_xml;
extern crate reqwest;
extern crate rmp_serde as rmps;
extern crate serde;
#[macro_use] extern crate serde_derive;

mod database;
mod errors;
mod pod;

use database::Database;
use errors::*;
use std::path::{Path};

fn run() -> Result<()> {
    let matches = clap_app!(app =>
        (version: crate_version!())
        (author: crate_authors!(", "))
        (about: crate_description!())
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

    let db_path = Path::new("podcasts.db");
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
            db.download().chain_err(|| "Tried to download podcast, world went boom.")?;
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
