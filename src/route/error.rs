#[derive(Debug)]
pub enum RouteError {
    HashMapInsertError(String, u32)
}

impl From<(&str, u32)> for RouteError {
    fn from(value: (&str, u32)) -> Self {
        RouteError::HashMapInsertError(value.0.to_string(), value.1)
    }
}