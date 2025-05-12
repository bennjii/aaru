use geo::Distance;
use rustc_hash::FxHashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use crate::route::transition::RoutingContext;

/// A generic read-through cache for a hashmap-backed data structure
#[derive(Debug)]
pub struct CacheMap<K, V, Meta>
where
    K: Hash + Eq + Copy + Debug,
    V: Debug,
    Meta: Debug,
{
    map: FxHashMap<K, Arc<V>>,
    metadata: Meta,
}

impl<K, V, Meta> CacheMap<K, V, Meta>
where
    CacheMap<K, V, Meta>: Calculable<K, V>,
    K: Hash + Eq + Copy + Debug,
    V: Debug,
    Meta: Debug,
{
    /// Exposes a query call for the cache map, allowing the caller
    /// to use the cache in its intended read-through pattern design.
    ///
    /// ### Behaviour
    ///
    /// This function is only exposed for [`CacheMap`] implementations
    /// which implement [`Calculable`].
    ///
    /// The function returns the value, [`V`] wrapped in a reference counter.
    /// This, therefore does not require [`V`] to be `Clone`. However, it
    /// consumes an owned value of the key, [`K`], which is required for the
    /// call to the [`Calculable::calculate`] function.
    pub fn query<'a>(&mut self, ctx: &RoutingContext<'a>, key: K) -> Arc<V> {
        if let Some(value) = self.map.get(&key) {
            return Arc::clone(value);
        }

        let calculated = Arc::new(self.calculate(ctx, key));
        self.map.insert(key, calculated.clone());

        Arc::clone(&calculated)
    }
}

impl<K, V, Meta> Default for CacheMap<K, V, Meta>
where
    K: Hash + Eq + Copy + Debug,
    V: Debug,
    Meta: Default + Debug,
{
    fn default() -> Self {
        Self {
            map: FxHashMap::default(),
            metadata: Meta::default(),
        }
    }
}

/// Implementation of a routing-domain calculable KV pair.
///
/// Asserts that the value, [`V`] can be generated from the key, [`K`],
/// given routing context, and the base structure.
///
/// ### Examples
///
/// The [`SuccessorsCache`] and [`PredicateCache`] are both examples
/// of calculable elements.
///
/// The [`SuccessorsCache`], given an underlying map key,
/// can derive the successors using the routing map and an
/// upper-bounded dijkstra algorithm.
pub trait Calculable<K, V> {
    /// The concrete implementation of the function which derives the
    /// value, [`V`], from the key, [`K`].
    ///
    /// The function parameters include relevant [`RoutingContext`] which
    /// may be required for the calculation.
    fn calculate(&mut self, ctx: &RoutingContext, key: K) -> V;
}

mod successor {
    use geo::Haversine;
    use petgraph::Direction;

    use crate::route::graph::NodeIx;
    use crate::route::transition::*;

    use super::*;

    /// The weights, given as output from the [`SuccessorsCache::calculate`] function.
    type SuccessorWeights = Vec<(NodeIx, WeightAndDistance)>;

    /// The cache map definition for the successors.
    ///
    /// It accepts a [`NodeIx`] as input, from which it will obtain all outgoing
    /// edges and obtain the distances to each one as a [`WeightAndDistance`].
    pub type SuccessorsCache = CacheMap<NodeIx, SuccessorWeights, ()>;

    impl Calculable<NodeIx, SuccessorWeights> for SuccessorsCache {
        fn calculate(&mut self, ctx: &RoutingContext, key: NodeIx) -> SuccessorWeights {
            // Calc. once
            let source = ctx.map.get_position(&key).unwrap();

            let successors = ctx
                .map
                .graph
                .edges_directed(key, Direction::Outgoing)
                .map(|(_, next, (w, _))| {
                    (
                        next,
                        if key != next {
                            let target = ctx.map.get_position(&next).unwrap();

                            // In centimeters (1m = 100cm)
                            WeightAndDistance(
                                CumulativeFraction {
                                    numerator: *w,
                                    denominator: 1,
                                },
                                (Haversine.distance(source, target) * 100f64) as u32,
                            )
                        } else {
                            // Total accrued distance
                            WeightAndDistance(
                                CumulativeFraction {
                                    numerator: *w,
                                    denominator: 1,
                                },
                                0,
                            )
                        },
                    )
                })
                .collect::<Vec<_>>();

            successors
        }
    }
}

mod predicate {
    use crate::route::graph::NodeIx;
    use crate::route::transition::primitives::Dijkstra;
    use crate::route::transition::*;

    use super::*;

    const DEFAULT_THRESHOLD: f64 = 200_000f64; // 2km in cm

    #[derive(Debug)]
    pub struct PredicateMetadata {
        /// The successors cache used to back the successors and
        /// prevent repeated calculations.
        successors: SuccessorsCache,

        /// The threshold by which the solver is bounded, in centimeters.
        threshold_distance: f64,
    }

    impl Default for PredicateMetadata {
        fn default() -> Self {
            Self {
                successors: SuccessorsCache::default(),
                threshold_distance: DEFAULT_THRESHOLD,
            }
        }
    }

    /// Predicates represents a hashmap of the input [`NodeIx`] as the key,
    /// and the pair of corresponding [`NodeIx`] and [`WeightAndDistance`] values
    /// which are reachable from the input index after performing an upper-bounded
    /// dijkstra calculation
    ///
    /// The output from the [`PredicateCache::calculate`] function.
    type Predicates = FxHashMap<NodeIx, (NodeIx, WeightAndDistance)>;

    /// The predicate cache through which a backing of [`Predicates`] is
    /// made from a [`NodeIx`] key, cached on first calculation and read thereafter.
    pub type PredicateCache = CacheMap<NodeIx, Predicates, PredicateMetadata>;

    impl Calculable<NodeIx, Predicates> for PredicateCache {
        fn calculate(&mut self, ctx: &RoutingContext, key: NodeIx) -> Predicates {
            let threshold = self.metadata.threshold_distance;

            Dijkstra
                .reach(&key, move |node| {
                    ArcIter::new(self.metadata.successors.query(ctx, *node))
                })
                .take_while(|p| {
                    // Bounded by the threshold distance (centimeters)
                    (p.total_cost.1 as f64) < threshold
                })
                .map(|pre| {
                    let parent = pre.parent.unwrap_or_default();
                    (pre.node, (parent, pre.total_cost))
                })
                .collect::<Predicates>()
        }
    }
}

/// Iterator wrapper that keeps the Arc alive while yielding `&T`
struct ArcIter<T> {
    data: Arc<Vec<T>>,
    index: usize,
}

impl<'a, T> ArcIter<T> {
    #[inline(always)]
    fn new(data: Arc<Vec<T>>) -> Self {
        ArcIter { data, index: 0 }
    }
}

impl<'a, T> Iterator for ArcIter<T>
where
    T: Copy,
{
    type Item = T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let item = *self.data.get(self.index)?;
        self.index += 1;
        Some(item)
    }
}

pub use predicate::PredicateCache;
pub use successor::SuccessorsCache;
