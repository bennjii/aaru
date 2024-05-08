use std::fmt::{Debug, format, Formatter};
use std::io;

pub enum ShardError {
    IOError(io::Error),
    OSMError(osmpbf::Error),
}

impl Debug for ShardError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ShardError::IOError(e) => format!("IOError: {}", e),
            ShardError::OSMError(e) => format!("OSMError: {}", e),
        })
    }
}

impl From<io::Error> for ShardError {
    fn from(value: io::Error) -> ShardError {
        ShardError::IOError(value)
    }
}

impl From<osmpbf::Error> for ShardError {
    fn from(value: osmpbf::Error) -> Self {
        ShardError::OSMError(value)
    }
}