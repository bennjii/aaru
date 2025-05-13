use std::io;
use std::io::Error;

#[derive(Debug)]
pub enum CodecError {
    IOError(std::io::Error),
}

impl From<io::Error> for CodecError {
    fn from(value: Error) -> Self {
        Self::IOError(value)
    }
}
