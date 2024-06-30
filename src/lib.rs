#![doc = include_str!("../readme.md")]
#![allow(dead_code)]

use crate::geo::error::GeoError;
#[cfg(feature = "codec")]
use crate::codec::error::CodecError;
#[cfg(feature = "route")]
use crate::route::error::RouteError;
#[cfg(feature = "tile")]
use crate::tile::error::TileError;

#[doc(hidden)]
pub mod util;
#[cfg(feature = "grpc_server")]
#[doc(hidden)]
pub mod server;
#[cfg(feature = "tile")]
pub mod tile;
#[cfg(feature = "route")]
pub mod route;
#[cfg(feature = "codec")]
pub mod codec;
pub mod geo;

#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "codec")]
    Codec(CodecError),
    #[cfg(feature = "route")]
    Route(RouteError),
    #[cfg(feature = "tile")]
    Tile(TileError),
    Geo(GeoError),
}

type Result<T> = std::result::Result<T, Error>;

impl_err!(GeoError, Geo);
#[cfg(feature = "codec")]
impl_err!(CodecError, Codec);
#[cfg(feature = "route")]
impl_err!(RouteError, Route);
#[cfg(feature = "tile")]
impl_err!(TileError, Tile);

