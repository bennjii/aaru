use axum::async_trait;

pub struct Query<T, F> {
    pub parameters: T,
    pub filter: F
}

impl<T, F> Query<T, F> {
    pub fn new(parameters: T, filter: F) -> Self {
        Query { parameters, filter }
    }

    pub fn add_param<K>(self, new_param: K) -> Query<(T, K), F> {
        Query { parameters: (self.parameters, new_param), filter: self.filter }
    }

    pub fn params(&self) -> &T {
        &self.parameters
    }

    pub fn filter(&self) -> &F {
        &self.filter
    }
}

#[async_trait]
pub trait Queryable<In, Filter, Out> {
    type Item;
    type Error;
    type Parameters;
    type Connection<'a> where Self: 'a;

    const QUERY_TABLE: &'static str;

    async fn query(&self, input: Query<In, Option<Filter>>, params: Self::Parameters) -> Result<Out, Self::Error>;
    fn batch(&self, query: Query<Self::Parameters, (u8, u32, u32)>) -> In;
    fn filter(&self, filter: &Self::Parameters, item: &Self::Item) -> bool;
    fn connection(&self) -> Result<Self::Connection<'_>, Self::Error>;
}

