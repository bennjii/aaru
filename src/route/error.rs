use crate::route::graph::Weight;

#[derive(Debug)]
pub enum RouteError {
    HashMapInsertError(String, Weight),
    Other(String),
}

impl From<(&str, Weight)> for RouteError {
    fn from(value: (&str, Weight)) -> Self {
        RouteError::HashMapInsertError(value.0.to_string(), value.1)
    }
}
