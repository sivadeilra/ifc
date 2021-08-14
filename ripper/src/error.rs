use std::borrow::Cow;

#[derive(Debug)]
pub enum Error {
    Bad(Cow<'static, str>),
    BadString,
    MissingExpected,
    IO(std::io::Error),
}

impl Error {
    pub fn bad(s: &'static str) -> Self {
        Self::Bad(Cow::Borrowed(s))
    }

    pub fn bad_string(s: String) -> Self {
        Self::Bad(Cow::Owned(s))
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e)
    }
}

pub type Result<T> = core::result::Result<T, Error>;
