extern crate chrono;
extern crate reqwest;
extern crate xml;

mod pod;

fn main() {
    println!("Result: {:?}", pod::get_rss("http://www.newrustacean.com/feed.xml"));
}

