# The Podinator

This is a project for managing the subscriptions of podcasts I'm listening to, and also for learning Rust.

## TODO

1. Logging, this might be a good start: https://doc.rust-lang.org/log/log/index.html
  1. Clean up the `println!`-messages. We have way too many right now.
1. Support for etags.
1. Make the podinator work with mixed feeds, like for instance: https://annien.wordpress.com/
1. Make it possible to mark episodes as listened to.
1. Web interface.
1. Create a "parse-all-feed-files-and-see-if-something-breaks"-command for testing new feeds.
1. `./podinator list`
  1. Make it human readable
  1. Let us look at single podcasts and their episodes.
1. Limit the used space. To, for instance 1 Gb or something.
