use std::fmt::Display;
use crate::mvt::{Layer, Tile};

impl From<Vec<Layer>> for Tile {
    fn from(value: Vec<Layer>) -> Self {
        Self {
            layers: value,
        }
    }
}