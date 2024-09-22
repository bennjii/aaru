use std::f64::consts::E;
use std::ops::{Div, Mul};

use geo::{EuclideanDistance, LineString, Point};
use tonic::codegen::tokio_stream::StreamExt;
use wkt::ToWkt;
use log::{debug, info};

use crate::route::Graph;
use crate::route::graph::{Edge, NodeIx};

const DEFAULT_ERROR: f64 = 20f64;

pub struct Transition<'a> {
    // The original linestring
    linestring: LineString,
    graph: &'a Graph,
    error: Option<f64>
}

struct TransitionNode<'a> {
    candidate: &'a TransitionCandidate<'a>,
    emission_probability: f64,
    transition_probability: f64
}

struct RefinedTransitionLayer<'a> {
    nodes: Vec<TransitionNode<'a>>,
    segment: &'a TrajectorySegment<'a>,
}

struct TransitionLayer<'a> {
    candidates: Vec<TransitionCandidate<'a>>,
    segment: &'a TrajectorySegment<'a>,
}

struct TransitionCandidate<'a> {
    index: NodeIx,
    edge: Edge<'a>,
    position: Point,
    prev_best: Option<&'a TransitionCandidate<'a>>,
}

struct TrajectorySegment<'a> {
    source: &'a Point,
    target: &'a Point,

    // The Euclidean length of the segment
    // ahead of the current node.
    //
    //  This + ----------‐ + Next
    //           ^ Euclidean length of line
    //
    length: f64
}

impl TrajectorySegment {
    pub fn new([a, b]: &[Point]) -> Self {
        TrajectorySegment {
            source: a,
            target: b,
            length: a.euclidean_distance(b)
        }
    }
}

impl Transition {
    pub fn new(linestring: LineString, graph: &Graph) -> Self {
         Transition {
             linestring,
             graph,
             error: None
         }
    }

    pub fn set_error(self, error: f64) -> Self {
        Transition { error: Some(error), ..self }
    }

    /// Calculates the emission probability of `dist` (the GPS error from
    /// the observed point and matched point) with a given GPS `error`
    fn emission_probability<T: Div + Mul>(dist: T, error: T) -> f64 {
        let alpha = dist / error;
        E.pow(-0.5f64 * (alpha.pow(2) as f64))
    }

    /// Calculates the transition probability of between two candidates
    /// given the `shortest_dist` between them, and the `euclidean_dist`
    /// of the segment we are transitioning over
    fn transition_probability<T: Div + PartialOrd>(shortest_dist: T, euclidean_dist: T) -> f64 {
        match euclidean_dist >= shortest_dist {
            true => 1.0f64,
            false => (euclidean_dist / shortest_dist) as f64
        }
    }

    /// Refines the candidates into TransitionNodes with their respective
    /// emission and transition probabilities in the hidden markov model.
    ///
    /// Based on the method used in FMM
    fn refine_candidates(&self, layer: &TransitionLayer) -> RefinedTransitionLayer {
        let nodes = layer.candidates.iter()
            .map(|candidate| {
                let distance = candidate.position.euclidean_distance(layer.segment.source);
                let emission_probability = Transition::emission_probability(
                    distance, self.error.unwrap_or(DEFAULT_ERROR)
                );

                TransitionNode {
                    candidate,
                    emission_probability,
                    transition_probability: 0.0f64,
                }
            })
            .flat_map(|v| std::iter::repeat(v).zip(layer.candidates.iter()))
            // Only differing candidates are important
            .filter(|(a, b)| a.candidate.index != b.index)
            // Iterating over the cartesian product of the candidates
            .filter_map(|(a, b)| {
                // If we can route between these candidates
                if let Some((weight, _)) = self.graph.route(a.candidate.position, b.position) {
                    let transition_probability = Transition::transition_probability(
                        weight,
                        layer.segment.length
                    );

                    // Imbue candidate with the transition probability
                    Some(TransitionNode {
                        candidate: a.candidate,
                        emission_probability: a.emission_probability,
                        transition_probability,
                    })
                }

                None
            })
            .collect();

        RefinedTransitionLayer {
            nodes,
            segment: layer.segment
        }
    }

    pub fn collapse(&self, layers: Vec<RefinedTransitionLayer>) -> Vec<Point> {
        if let Some(last_layer) = layers.last() {
            // Find the optimal candidate in last_layer.candidates
            let optimal_node: &TransitionNode = last_layer.nodes.last().unwrap();
            let optimal_candidate: &TransitionCandidate = optimal_node.candidate;

             return std::iter::from_fn({
                let mut previous_best = optimal_candidate.prev_best;
                move || {
                    // Perform rollup on the candidates to walk-back the path
                    previous_best.take().map(|prev| {
                        previous_best = prev.prev_best;
                        prev
                    })
                }
            })
                 .fuse()
                 .inspect(|candidate| {
                     debug!("Candidate {:?} ({}) selected.", candidate.index, candidate.position.wkt_string())
                 })
                 .rev()
                 .map(|candidate| candidate.position)
                 .collect::<Vec<_>>();
        }

        // Return no points by default
        vec![]
    }

    pub fn backtrack(&self, distance: i64) -> impl Iterator<Item=TransitionLayer> + '_ {
        // Deconstruct the trajectory into individual segments
        let as_coordinates = self.linestring.clone().into_points();
        let segments = as_coordinates
            .windows(2)
            .map(TrajectorySegment::new)
            .collect::<Vec<_>>();

        // TODO: Merge ˄ and ˅ into one iterator?

        // Create transition layers from each segment
        let layers = segments
            .iter()
            .filter_map(|segment| {
                let candidates = self.graph
                    // Get all relevant (projected) nodes within 60m
                    .nearest_projected_nodes(&segment.source, distance)
                    .map(|(node, edge)| {
                        TransitionCandidate {
                            index,
                            edge,
                            position,
                            prev_best: unimplemented!(),
                        }
                    })
                    .collect::<Vec<_>>();

                Some(TransitionLayer { candidates, segment })
            })
            .collect::<Vec<_>>();

        // Now we refine the candidates
        layers
            .iter()
            .map(|layer| self.refine_candidates(layer))
    }
}