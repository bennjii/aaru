#[derive(Debug)]
pub enum RouteError {
    HashMapInsertError(String, u8)
}

impl From<(&str, u8)> for RouteError {
    fn from(value: (&str, u8)) -> Self {
        RouteError::HashMapInsertError(value.0.to_string(), value.1)
    }
}