use std::fmt::Display;
use crate::mvt::{Layer, Tile, Value};

impl From<Vec<Layer>> for Tile {
    fn from(value: Vec<Layer>) -> Self {
        Self {
            layers: value,
        }
    }
}

impl From<Layer> for Tile {
    fn from(value: Layer) -> Self {
        Tile::from(vec![value])
    }
}

impl Value {
    pub fn from_int(value: i64) -> Self {
        Self {
            int_value: Some(value),
            ..Self::default()
        }
    }

    pub fn from_float(float: f32) -> Self {
        Self {
            float_value: Some(float),
            ..Self::default()
        }
    }

    pub fn from_string(value: String) -> Self {
        Self {
            string_value: Some(value),
            ..Self::default()
        }
    }
}