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

// TODO: Docs
pub trait Calculable<K, V> {
    fn calculate(&mut self, ctx: &RoutingContext, key: K) -> V;
}

impl<K, V, Meta> CacheMap<K, V, Meta>
where
    CacheMap<K, V, Meta>: Calculable<K, V>,
    K: Hash + Eq + Copy + Debug,
    V: Debug,
    Meta: Debug,
{
    pub fn query<'a>(&mut self, ctx: &RoutingContext<'a>, key: K) -> Arc<V> {
        if let Some(value) = self.map.get(&key) {
            return Arc::clone(value);
        }

        let calculated = Arc::new(self.calculate(ctx, key));
        self.map.insert(key, calculated.clone());

        Arc::clone(&calculated)
    }
}

mod successor {
    use geo::Haversine;
    use petgraph::Direction;

    use crate::codec::element::variants::OsmEntryId;
    use crate::route::transition::*;

    use super::*;

    type SuccessorWeights = Vec<(OsmEntryId, WeightAndDistance)>;
    pub type SuccessorsCache = CacheMap<OsmEntryId, SuccessorWeights, ()>;

    impl Calculable<OsmEntryId, SuccessorWeights> for SuccessorsCache {
        fn calculate(&mut self, ctx: &RoutingContext, key: OsmEntryId) -> SuccessorWeights {
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
    use crate::codec::element::variants::OsmEntryId;
    use crate::route::transition::primitives::Dijkstra;
    use crate::route::transition::*;

    use super::*;

    const DEFAULT_THRESHOLD: f64 = 200_000f64; // 2km in cm

    #[derive(Debug)]
    pub struct PredicateMetadata {
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

    type Predicates = FxHashMap<OsmEntryId, (OsmEntryId, WeightAndDistance)>;
    pub type PredicateCache = CacheMap<OsmEntryId, Predicates, PredicateMetadata>;

    impl Calculable<OsmEntryId, Predicates> for PredicateCache {
        fn calculate(&mut self, ctx: &RoutingContext, key: OsmEntryId) -> Predicates {
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
