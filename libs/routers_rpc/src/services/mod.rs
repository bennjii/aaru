use alloc::sync::Arc;
use routers_codec::osm::OsmNetwork;
use std::path::PathBuf;

pub struct RPCAdapter<T> {
    pub(crate) inner: Arc<T>,
}

impl<T> RPCAdapter<T> {
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

pub mod matcher;
pub mod optimise;
pub mod proximity;
pub mod timezone;

pub struct OsmService;
impl OsmService {
    pub fn from_file(file: PathBuf) -> Result<OsmNetwork, Box<dyn core::error::Error>> {
        OsmNetwork::from_pbf(&file)
    }
}

#[cfg(feature = "overture")]
pub struct OvertureService;

#[cfg(feature = "overture")]
impl OvertureService {
    pub fn from_file(
        file: PathBuf,
    ) -> Result<routers_codec::overture::OvertureNetwork, Box<dyn core::error::Error>> {
        routers_codec::overture::OvertureNetwork::from_geoparquet(&file).map_err(Into::into)
    }
}
