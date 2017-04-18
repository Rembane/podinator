use std::fmt::Display;

error_chain! {
    foreign_links {
        Encoding(::rmps::encode::Error);
        Decoding(::rmps::decode::Error);
        IO(::std::io::Error);
    }
}

/*
impl ::serde::de::Error for Error {
    fn custom<T>(msg: T) -> Error
        where T: Display
    {
        format!("{}", msg).into()
    }
}
*/
