extern crate chrono;
extern crate itertools;
extern crate quick_xml;
extern crate reqwest;

mod pod;

fn main() {
    let mut p = pod::Podcast::new(" ", "http://www.newrustacean.com/feed.xml");
    p.get_rss().unwrap();
    p.download();
}
