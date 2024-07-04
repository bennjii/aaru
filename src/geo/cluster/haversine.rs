use crate::geo::{MEAN_EARTH_RADIUS, TileItem};

pub fn haversine_distance<const N: usize, P, T: TileItem<P, N>>(lhs: &T, rhs: &T) -> f64 {
    let (l_lat, l_lng) = lhs.lat_lng().expand();
    let (r_lat, r_lng) = rhs.lat_lng().expand();

    let two: f64 = 1f64 + 1f64;
    let theta1 = l_lng.to_radians();
    let theta2 = r_lng.to_radians();
    let delta_theta = (r_lng - l_lng).to_radians();
    let delta_lambda = (r_lat - l_lat).to_radians();

    let a = (delta_theta / two).sin().powi(2)
        + theta1.cos() * theta2.cos()
        * (delta_lambda / two).sin().powi(2);

    let c = two * a.sqrt().asin();
    MEAN_EARTH_RADIUS * c
}
