use std::ops::{Add, Sub, Mul};
use crate::geo::coord::latlng::{NanoDegree};
use crate::geo::LatLng;

#[derive(Copy, Clone)]
pub struct Vector<T: Copy> {
    pub x: T,
    pub y: T
}

impl<T: Sub<Output = T> + Add<Output = T> + Mul<Output = T> + Copy> Vector<T> {
    pub fn to(&self, other: Vector<T>) -> Self {
        Vector { x: self.x - other.x, y: self.y - other.y }
    }

    pub fn dot(&self, other: &Vector<T>) -> T {
        (self.x * other.x) + (self.y * other.y)
    }
}

impl<T: Sub<Output=T> + Copy> Sub for Vector<T> {
    type Output = Vector<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        Vector { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl From<&LatLng> for Vector<NanoDegree> {
    fn from(value: &LatLng) -> Vector<NanoDegree> {
        Vector { x: value.lng, y: value.lat }
    }
}