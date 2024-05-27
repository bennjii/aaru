#[derive(Debug)]
pub enum RouteError {
    HashMapInsertError(String, i32)
}

impl From<(&str, i32)> for RouteError {
    fn from(value: (&str, i32)) -> Self {
        RouteError::HashMapInsertError(value.0.to_string(), value.1)
    }
}