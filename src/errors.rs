use std::fmt::Display;

error_chain! {
    foreign_links {
        Encoding(::rmps::encode::Error);
        Decoding(::rmps::decode::Error);
        IO(::std::io::Error);
    }
}
