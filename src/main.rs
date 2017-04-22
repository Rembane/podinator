#![recursion_limit = "1024"]

#[macro_use] extern crate chan;
extern crate chan_signal;
extern crate chrono;
#[macro_use] extern crate clap;
extern crate core;
#[macro_use] extern crate error_chain;
extern crate futures;
extern crate itertools;
extern crate hyper;
extern crate hyper_tls;
extern crate quick_xml;
extern crate rmp_serde as rmps;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate tokio_core;
extern crate toml;

mod database;
mod errors;
mod pod;

use chan_signal::Signal;
use std::fs::{File};
use std::io::{Read};
use std::path::{Path};
use std::sync::{Arc, RwLock};
use std::thread;

use database::Database;
use errors::*;

#[derive(Clone, Debug, Deserialize)]
struct Config {
    db_path: String,
    podcast_path: String,
}

/// Initialize the database and the config object and return them.
fn initialize<'a>() -> Result<(Database, Config, clap::ArgMatches<'a>)> {
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
        (@subcommand episodes =>
            (about: "Manages episodes.")
            (@subcommand clear =>
                (about: "Deletes all episodes.")
            )
        )
    ).get_matches();

    let mut config: Config = match File::open(matches.value_of("CONFIG").unwrap_or("podinator.toml")) {
        Ok(mut fh) => {
            let mut s = String::new();
            fh.read_to_string(&mut s)?;
            toml::from_str(&s)?
        }
        Err(_) => { // Default configuration.
            Config {
                db_path: String::from("podcasts.db"),
                podcast_path: String::from("podcasts/")
            }
        }
    };

    if let Some(p) = matches.value_of("DBPATH") {
         config.db_path = String::from(p);
    }
    if let Some(p) = matches.value_of("PODPATH") {
        config.podcast_path = String::from(p);
    }

    let db_path = Path::new(&config.db_path);
    // If the database file doesn't exist, we create a new database.
    let mut db = match Database::from_file(db_path) {
        Ok(d) => d,
        Err(_) => Database::new(),
    };
    Ok((db, config.clone(), matches))
}

/// Run the program while catching all signals.
fn run<'a>(_sdone: chan::Sender<()>, db: Arc<RwLock<Database>>, config: &Config, matches: clap::ArgMatches<'a>) -> Result<()> {
    // This is our database!
    let db_path = Path::new(&config.db_path);
    match matches.subcommand() {
        ("add", Some(matches)) => {
            let url = matches.value_of("URL").ok_or("Please supply a URL.")?;
            let mut db = db.write().unwrap();
            db.add(url);
            db.to_file(db_path).chain_err(|| "Writing to database failed.")?;
        }
        ("list", Some(_)) => {
            let db2 = db.read().unwrap();
            for p in db2 {
                println!("{}", p);
            }
        }
        ("download", Some(_)) => {
            let mut db = db.write().unwrap();
            db.download(Path::new(&config.podcast_path)).chain_err(|| "Tried to download podcast, world went boom.")?;
            db.to_file(db_path).chain_err(|| "Tried to save podcast database to file. Something failed.")?;
        }
        ("episodes", Some(submatch)) => {
            if let ("clear", Some(_)) = submatch.subcommand() {
                let mut db = db.write().unwrap();
                db.clear_episodes();
            }
        }
        _ => {},
    }
    Ok(())
}

/// Consistent error handling boilerplate.
fn error_handling<A>(res: Result<A>) -> A {
    match res {
        Err(ref e) => {
            println!("Error: {}", e);
            for e in e.iter().skip(1) {
                println!("Caused by: {}", e);
            }
            if let Some(backtrace) = e.backtrace() {
                println!("Backtrace: {:?}", backtrace);
            }
            std::process::exit(1);
        }
        Ok(v) => v
    }
}

fn main() {
    // Signal handling boilerplate.
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);
    let (sdone, rdone) = chan::sync(0);
    let (db, config, matches) = error_handling(initialize());
    let db_path = Path::new(&config.db_path);
    let thread_handle = thread::spawn(move || error_handling(run(sdone, Arc::new(RwLock::new(db)), &config, matches)));
    chan_select! {
        signal.recv() -> signal => {
            println!("Received signal: {:?}, exiting gracefully.", signal);
            db.to_file(&db_path);
        },
        rdone.recv() => {
            // We're done, just exit.
            std::process::exit(0);
        }
    }
}

use tokio_core::reactor::Core;
use futures::{Future, Stream};
use futures::future;

use hyper::{Url, Method, Error};
use hyper::client::{Client, Request};
use hyper::header::{Authorization, Accept, UserAgent, qitem};
use hyper::mime::Mime;
use hyper_tls::HttpsConnector;

fn main() {
    let url = Url::parse("https://api.github.com/user").unwrap();
    let mut req = Request::new(Method::Get, url);
    let mime: Mime = "application/vnd.github.v3+json".parse().unwrap();
    let token = String::from("token {Your_Token_Here}");
    req.headers_mut().set(UserAgent(String::from("github-rs")));
    req.headers_mut().set(Accept(vec![qitem(mime)]));
    req.headers_mut().set(Authorization(token));

    let mut event_loop = Core::new().unwrap();
    let handle = event_loop.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4,&handle))
        .build(&handle);
    let work = client.request(req)
        .and_then(|res| {
            println!("Response: {}", res.status());
            println!("Headers: \n{}", res.headers());

            res.body().fold(Vec::new(), |mut v, chunk| {
                v.extend(&chunk[..]);
                future::ok::<_, Error>(v)
            }).and_then(|chunks| {
                let s = String::from_utf8(chunks).unwrap();
                future::ok::<_, Error>(s)
            })
        });
    let user = event_loop.run(work).unwrap();
    println!("We've made it outside the request! \
              We got back the following from our \
              request:\n");
    println!("{}", user);
}
