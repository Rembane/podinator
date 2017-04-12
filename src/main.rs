extern crate chrono;
extern crate reqwest;
extern crate xml;

mod pod;

fn main() {
    let mut p = pod::Podcast::new(" ", "http://www.newrustacean.com/feed.xml");
    p.get_rss().unwrap();
    p.download();
}
