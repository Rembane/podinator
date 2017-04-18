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

    let mut db = Database::from_file(Path::new("podcasts.db")).chain_err(|| "Something went terribly wrong when loading the database file.")?;

    match matches.subcommand() {
        ("add", Some(matches)) => {
            let url = matches.value_of("URL").ok_or("Please supply a URL.")?;
            db.add(url);
        }
        ("list", Some(_)) => {
            for p in db.into_iter() {
                println!("{:?}", p);
            }
        }
        ("download", Some(_)) => {
            for mut p in db.into_iter() {
                p.get_rss().chain_err(|| "Downloading RSS failed horribly.")?;
                p.download().chain_err(|| "Downloading podcast failed horribly.")?;
            }
        }
        (_, _) => {},
    }
    Ok(())
//    let mut p = pod::Podcast::new(" ", "http://www.newrustacean.com/feed.xml");
//    p.get_rss().unwrap();
//    p.download();
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
