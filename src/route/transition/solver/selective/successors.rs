use crate::codec::element::variants::OsmEntryId;
use crate::route::graph::NodeIx;
use crate::route::transition::solver::selective::cumulative::CumulativeFraction;
use crate::route::transition::solver::selective::weight_and_distance::WeightAndDistance;
use crate::route::transition::RoutingContext;
use geo::{Distance, Haversine};
use log::debug;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use std::num::NonZeroI64;
use std::sync::Arc;

// DG.UB.PN.OD.T: Dynamically-Generated Upper-Bounded Piecewise-N Origin-Destination Table (😅)
#[derive(Debug)]
pub struct SuccessorsLookupTable {
    // TODO: Move ref-cell inside?
    successors: FxHashMap<NonZeroI64, Arc<Vec<(NonZeroI64, WeightAndDistance)>>>,
}

impl SuccessorsLookupTable {
    #[inline]
    pub fn new() -> Self {
        Self {
            successors: FxHashMap::default(),
        }
    }

    pub fn __remove_me(node: &NonZeroI64) -> NodeIx {
        OsmEntryId::as_node(node.get())
    }

    #[inline]
    fn calculate(ctx: RoutingContext, node: &NonZeroI64) -> Vec<(NonZeroI64, WeightAndDistance)> {
        debug!("cache miss");

        // Calc. once
        let source = ctx.map.get_position(&Self::__remove_me(node)).unwrap();

        let successors = ctx
            .map
            .graph
            .edges_directed(Self::__remove_me(node), Direction::Outgoing)
            .map(|(_, next, (w, _))| {
                (
                    NonZeroI64::new(next.identifier).unwrap(),
                    if Self::__remove_me(node) != next {
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

    #[inline]
    pub(crate) fn lookup(
        &mut self,
        ctx: RoutingContext,
        node: &NonZeroI64,
    ) -> Arc<Vec<(NonZeroI64, WeightAndDistance)>> {
        Arc::clone(
            self.successors
                .entry(*node)
                .or_insert_with_key(|node| Arc::new(SuccessorsLookupTable::calculate(ctx, node))),
        )
    }
}
