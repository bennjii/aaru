pub use shard::*;
pub use codec::*;
pub use route::*;
pub use geo::*;
pub use server::*;

use crate::codec::error::CodecError;
use crate::shard::error::ShardError;


pub mod shard;
pub mod codec;
pub mod route;
pub mod geo;
pub mod server;

pub enum Error {
    Shard(ShardError),
    Codec(CodecError)
}

type Result<T> = std::result::Result<T, Error>;