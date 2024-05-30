pub use shard::*;
pub use codec::*;
pub use route::*;
pub use geo::*;
pub use server::*;

use crate::codec::error::CodecError;
use crate::route::error::RouteError;
use crate::shard::error::ShardError;

pub mod shard;
pub mod codec;
pub mod route;
pub mod geo;
pub mod server;

#[derive(Debug)]
pub enum Error {
    Shard(ShardError),
    Codec(CodecError),
    Route(RouteError)
}

type Result<T> = std::result::Result<T, Error>;

impl From<RouteError> for Error {
    fn from(value: RouteError) -> Self {
        Error::Route(value)
    }
}

impl From<CodecError> for Error {
    fn from(value: CodecError) -> Self {
        Error::Codec(value)
    }
}

impl From<ShardError> for Error {
    fn from(value: ShardError) -> Self {
        Error::Shard(value)
    }
}