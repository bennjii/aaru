#![doc = include_str!("../README.md")]
#![allow(dead_code)]

#[cfg(all(feature = "osm", feature = "overture"))]
compile_error!("cannot enable multiple data sources at the same time, pick one of 'osm, overture'");

extern crate alloc;

pub use routers_transition::*;

pub mod transition {
    pub use routers_transition::*;
}

pub mod codec {
    pub use routers_codec::*;
}

pub mod network {
    pub use routers_network::*;
}

pub mod shard {
    pub use routers_shard::*;
}

#[cfg(test)]
pub mod test;
