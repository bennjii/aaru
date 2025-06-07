pub mod access;

use crate::osm::{Parser, Tags};
pub use access::AccessTag;

pub trait Access {
    fn access(&self) -> Vec<AccessTag>;
}

impl Access for Tags {
    fn access(&self) -> Vec<AccessTag> {
        Vec::<AccessTag>::parse(self).unwrap_or_default()
    }
}
