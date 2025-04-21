use crate::route::transition::WeightAndDistance;
use indexmap::map::Entry;
use indexmap::IndexMap;
use pathfinding::num_traits::Zero;
use rustc_hash::{FxHashSet, FxHasher};
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::hash::{BuildHasherDefault, Hash};
use std::num::NonZeroI64;
use std::ops::Index;

type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;

type Node = NonZeroI64;
type Cost = WeightAndDistance;

#[derive(Debug)]
struct SmallestHolder {
    cost: Cost,
    index: usize,
}

impl PartialEq for SmallestHolder {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for SmallestHolder {}

impl PartialOrd for SmallestHolder {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SmallestHolder {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}

/// Struct returned by [`dijkstra_reach`].
pub struct DijkstraReachable<FN> {
    to_see: BinaryHeap<SmallestHolder>,
    seen: FxHashSet<usize>,
    parents: FxIndexMap<Node, (usize, Cost)>,
    successors: FN,
}

/// Information about a node reached by [`dijkstra_reach`].
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct DijkstraReachableItem {
    /// The node that was reached by [`dijkstra_reach`].
    pub node: Node,
    /// The previous node that the current node came from.
    /// If the node is the first node, there will be no parent.
    ///
    /// Note: Uses 0 as a sentinel for "no-parent"
    pub parent: Option<Node>,
    /// The total cost from the starting node.
    pub total_cost: Cost,
}

impl<FN, IN> Iterator for DijkstraReachable<FN>
where
    FN: FnMut(&Node) -> IN,
    IN: IntoIterator<Item = (Node, Cost)>,
{
    type Item = DijkstraReachableItem;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(SmallestHolder { cost, index }) = self.to_see.pop() {
            if !self.seen.insert(index) {
                continue;
            }

            let (item, successors) = {
                let (node, (parent_index, cost)) = self.parents.get_index(index).unwrap();
                let item = Some(DijkstraReachableItem {
                    node: *node,
                    parent: self.parents.get_index(*parent_index).map(|x| *x.0),
                    total_cost: *cost,
                });

                (item, (self.successors)(node))
            };

            for (successor, move_cost) in successors {
                let new_cost = cost + move_cost;

                let mut pushed = false;

                let index = match self.parents.entry(successor.clone()) {
                    Entry::Vacant(e) => {
                        let n = e.index();
                        e.insert((index, new_cost));
                        pushed = true;
                        n
                    }
                    Entry::Occupied(mut e) => {
                        if e.get().1 > new_cost {
                            e.insert((index, new_cost));
                            pushed = true;
                            e.index()
                        } else {
                            continue;
                        }
                    }
                };

                if pushed {
                    self.to_see.push(SmallestHolder {
                        cost: new_cost,
                        index,
                    });
                }
            }

            return item;
        }

        None
    }
}

pub struct Dijkstra;

impl Dijkstra {
    /// Visit all nodes that are reachable from a start node. The node
    /// will be visited in order of cost, with the closest nodes first.
    ///
    /// The `successors` function receives the current node, and returns
    /// an iterator of successors associated with their move cost.
    pub fn reach<FN, IN>(&self, start: &Node, successors: FN) -> DijkstraReachable<FN>
    where
        FN: FnMut(&Node) -> IN,
        IN: IntoIterator<Item = (Node, Cost)>,
    {
        let mut to_see: BinaryHeap<SmallestHolder> = BinaryHeap::with_capacity(256);
        to_see.push(SmallestHolder {
            cost: Zero::zero(),
            index: 0,
        });

        let mut parents: FxIndexMap<Node, (usize, Cost)> =
            FxIndexMap::with_capacity_and_hasher(64, BuildHasherDefault::<FxHasher>::default());

        parents.insert(start.clone(), (usize::MAX, Zero::zero()));
        let seen = FxHashSet::default();

        DijkstraReachable {
            to_see,
            seen,
            parents,
            successors,
        }
    }
}
