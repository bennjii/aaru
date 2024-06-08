pub mod decode;
pub mod element;
pub(crate) mod blob;
pub(crate) mod block;
pub mod test;
pub mod error;
pub mod parallel;
pub mod consts;

pub mod osm {
    include!(concat!(env!("OUT_DIR"), "/osmpbf.rs"));
}