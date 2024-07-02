use std::collections::HashMap;
use std::marker::PhantomData;
use geo::{Centroid, ConvexHull, LineString, Polygon};

use crate::geo::cluster::haversine::haversine_distance;
use crate::geo::{LatLng, Point};
use crate::geo::error::GeoError;

#[derive(PartialEq, Clone)]
pub enum Classification {
    Core(usize),
    Edge(usize),
    Noise,
}

pub struct Clustered<const N: usize, P, T: Point<P, N>> {
    pub id: u32,
    pub points: Vec<T>,

    centroid: LatLng,
    convex_hull: Polygon,

    phantom_data: PhantomData<P>
}

pub struct Cluster<const N: usize, P, T: Point<P, N>> {
    // a.k.a Spillage
    pub noise: Vec<T>,
    pub clustered: Vec<Clustered<N, P, T>>,

    phantom_data: PhantomData<P>
}

impl<const N: usize, P, T: Point<P, N>> TryFrom<(Vec<T>, u8)> for Clustered<N, P, T> {
    type Error = GeoError;

    fn try_from((value, zoom): (Vec<T>, u8)) -> Result<Self, Self::Error> {
        let polygon = Polygon::new(
            LineString::from_iter(value.iter().map(|p| p.lat_lng().slice())),
            vec![],
        );

        let convex_hull = polygon.convex_hull();
        let centroid = convex_hull.centroid().ok_or(GeoError::InvalidCoordinate("".to_string()))?;
        let lat_lng = LatLng::from_degree(centroid.x(), centroid.y())?;

        Ok(Self {
            id: lat_lng.hash(zoom),
            centroid: lat_lng,

            points: value,
            convex_hull,
            phantom_data: PhantomData::default()
        })
    }
}

impl<const N: usize, P, T: Point<P, N>> From<Vec<T>> for Cluster<N, P, T> {
    fn from(value: Vec<T>) -> Self {
        Self {
            noise: value,
            clustered: Vec::new(),
            phantom_data: PhantomData::default()
        }
    }
}

impl<const N: usize, P, T: Point<P, N>> TryFrom<(Vec<(u32, T)>, u8)> for Cluster<N, P, T> {
    type Error = GeoError;

    fn try_from((value, zoom): (Vec<(u32, T)>, u8)) -> Result<Self, GeoError> {
        let mut hashmap: HashMap<u32, Vec<T>> = HashMap::new();
        value
            .into_iter()
            .for_each(|(hash, p)| match hashmap.get_mut(&hash) {
                None => { hashmap.insert(hash, vec![p]); }
                Some(group) => { group.push(p); }
            });

        let grouped: Vec<Vec<T>> = hashmap.into_values().collect();

        let mut clustered: Vec<Clustered<N, P, T>> = vec![];
        let mut noise: Vec<T> = vec![];

        grouped
            .into_iter()
            .for_each(|mut group| {
                if group.len() >= 3 {
                    if let Ok(cluster) = Clustered::try_from((group, zoom)) {
                        clustered.push(cluster);
                    }
                } else {
                    noise.push(group.remove(0));
                }
            });

        Ok(Self { clustered, noise, phantom_data: PhantomData::default() })
    }
}

pub struct IntoCluster<const N: usize, P, T: Point<P, N>> {
    pub epsilon: f64,
    pub c_capacity: usize,

    distance: for<'a, 'b> fn(&'a T, &'a T) -> f64,
    c: Vec<Classification>,
    v: Vec<bool>,

    phantom_data: PhantomData<P>
}

impl<const N: usize, P, T: Point<P, N>> Default for IntoCluster<N, P, T> {
    fn default() -> Self {
        IntoCluster {
            epsilon: 1.0,
            c_capacity: 10,
            distance: haversine_distance,
            c: Vec::new(),
            v: Vec::new(),
            phantom_data: PhantomData::default()
        }
    }
}

impl<const N: usize, P, T: Point<P, N>> IntoCluster<N, P, T> {
    pub fn new() -> Self {
        IntoCluster::default()
    }

    pub fn distance(self, distance_fn: fn(_: &T, _: &T) -> f64) -> Self {
        Self {
            distance: distance_fn,
            ..self
        }
    }

    #[inline]
    fn range_query(&self, sample: &T, population: &[T]) -> Vec<usize> {
        population
            .iter()
            .enumerate()
            .filter(|(_, pt)| (self.distance)(sample, pt) < self.epsilon)
            .map(|(idx, _)| idx)
            .collect()
    }

    fn expand_cluster(
        &mut self,
        population: &[T],
        queue: &mut Vec<usize>,
        cluster: usize,
    ) -> bool {
        let mut new_cluster = false;
        while let Some(ind) = queue.pop() {
            let neighbors = self.range_query(&population[ind], population);
            if neighbors.len() < self.c_capacity {
                continue;
            }

            new_cluster = true;
            self.c[ind] = Classification::Core(cluster);

            for n_idx in neighbors {
                // n_idx is at least an edge point
                if self.c[n_idx] == Classification::Noise {
                    self.c[n_idx] = Classification::Edge(cluster);
                }

                if self.v[n_idx] {
                    continue;
                }

                self.v[n_idx] = true;
                queue.push(n_idx);
            }
        }
        new_cluster
    }

    pub fn cluster(mut self, population: Vec<T>, zoom: u8) -> Result<Cluster<N, P, T>, GeoError> {
        self.c = vec![Classification::Noise; population.len()];
        self.v = vec![false; population.len()];

        let mut cluster = 0;
        let mut queue = Vec::new();

        for idx in 0..population.len() {
            if self.v[idx] {
                continue;
            }

            self.v[idx] = true;

            queue.push(idx);

            if self.expand_cluster(population.as_slice(), &mut queue, cluster) {
                cluster += 1;
            }
        }

        let points = self.c
            .iter()
            .zip(population)
            .map(|(p, c)| match p {
                Classification::Core(id) => (*id as u32 + 1, c),
                _ => (0, c),
            })
            .collect();

        Cluster::try_from((points, zoom))
    }
}