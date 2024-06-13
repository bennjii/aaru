pub use server::*;
pub use shard::*;
pub use codec::*;
pub use route::*;
pub use util::*;
pub use geo::*;

use crate::codec::error::CodecError;
use crate::route::error::RouteError;
use crate::shard::error::ShardError;
use crate::geo::error::GeoError;

use crate::err::err_macro::impl_err;

pub mod util;
pub mod tile;
pub mod shard;
pub mod codec;
pub mod route;
pub mod geo;
pub mod server;

#[derive(Debug)]
pub enum Error {
    Shard(ShardError),
    Codec(CodecError),
    Route(RouteError),
    Geo(GeoError)
}

type Result<T> = std::result::Result<T, Error>;

impl_err!(RouteError, Route);
impl_err!(CodecError, Codec);
impl_err!(ShardError, Shard);
impl_err!(GeoError, Geo);
