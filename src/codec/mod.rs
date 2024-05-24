pub mod decode;
pub(crate) mod blob;
pub(crate) mod block;
pub(crate) mod test;
mod element;

pub mod osm {
    include!(concat!(env!("OUT_DIR"), "/osmpbf.rs"));
}