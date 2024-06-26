#![allow(dead_code)]

use crate::codec::error::CodecError;
use crate::route::error::RouteError;
use crate::geo::error::GeoError;
use crate::tile::error::TileError;

pub mod util;
pub mod tile;
pub mod codec;
pub mod route;
pub mod geo;
pub mod server;

#[derive(Debug)]
pub enum Error {
    Codec(CodecError),
    Route(RouteError),
    Tile(TileError),
    Geo(GeoError),
}

type Result<T> = std::result::Result<T, Error>;

impl_err!(RouteError, Route);
impl_err!(CodecError, Codec);
impl_err!(TileError, Tile);
impl_err!(GeoError, Geo);
