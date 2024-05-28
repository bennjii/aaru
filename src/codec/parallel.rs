pub trait Parallel {
    type Item<'a>;


    fn for_each<F>(self, f: F) -> ()
        where
            F: for<'a> Fn(Self::Item<'_>) + Send + Sync;


    fn map_red<Map, Reduce, Identity, T>(
        self,
        map_op: Map,
        red_op: Reduce,
        ident: Identity
    ) -> T
        where
            Map: for<'a> Fn(Self::Item<'_>) -> T + Send + Sync,
            Reduce: Fn(T, T) -> T + Send + Sync,
            Identity: Fn() -> T + Send + Sync,
            T: Send;

    fn par_red<Reduce, Identity, Combine, T>(
        self,
        fold_op: Reduce,
        ident: Identity,
        combine: Combine
    ) -> T
        where
            Reduce: for<'a> Fn(T, Self::Item<'_>) -> T + Send + Sync,
            Identity: Fn() -> T + Send + Sync,
            Combine: Fn(T, T) -> T + Send + Sync,
            T: Send;
}