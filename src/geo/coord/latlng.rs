use std::fmt::{Debug, Formatter};

use crate::codec::osm::PrimitiveBlock;
use crate::geo::coord::vec::Vector;
use crate::geo::error::GeoError;
use crate::geo::project::SlippyTile;
use crate::geo::{Project, SRID3857_MAX_LNG};

#[cfg(feature = "grpc_server")]
use crate::server::route::router_service::Coordinate;
#[cfg(feature = "grpc_server")]
use tonic::Status;

pub type NanoDegree = i64;
pub type Degree = f64;

/// `LatLng`
/// The latitude, longitude pair structure, geotags an item with a location.
///
/// ```rust,ignore
/// use aaru::geo::coord::latlng::LatLng;
/// let latlng = LatLng::new(10, 10);
/// println!("Position: {}", latlng);
/// ```
#[derive(Clone, Copy, PartialOrd, PartialEq)]
pub struct LatLng {
    pub lng: NanoDegree,
    pub lat: NanoDegree,
}

#[cfg(feature = "grpc_server")]
impl TryFrom<Coordinate> for LatLng {
    type Error = GeoError;

    fn try_from(coord: Coordinate) -> Result<Self, Self::Error> {
        LatLng::from_degree(coord.latitude, coord.longitude)
    }
}

#[cfg(feature = "grpc_server")]
impl TryFrom<Option<Coordinate>> for LatLng {
    type Error = Status;

    fn try_from(value: Option<Coordinate>) -> Result<Self, Self::Error> {
        value.map_or(
            Err(Status::invalid_argument("Missing coordinate")),
            |coord| LatLng::try_from(coord).map_err(|err| Status::internal(format!("{:?}", err))),
        )
    }
}

impl From<(&i64, &i64)> for LatLng {
    /// Format is: (Lat, Lng)
    fn from((lat, lng): (&i64, &i64)) -> Self {
        Self::new(*lat, *lng)
    }
}

impl LatLng {
    /// Constructs a new `LatLng` from a given `lat` and `lng`.
    pub fn new(lat: NanoDegree, lng: NanoDegree) -> Self {
        LatLng { lat, lng }
    }

    #[cfg(feature = "grpc_server")]
    /// Converts from `LatLng` into the `Coordinate` proto message
    pub fn coordinate(&self) -> Coordinate {
        Coordinate {
            latitude: self.lat(),
            longitude: self.lng(),
        }
    }

    pub fn from_degree(lat: Degree, lng: Degree) -> Result<Self, GeoError> {
        if !(lat > -90f64 && lat < 90f64) {
            return Err(GeoError::InvalidCoordinate(format!(
                "Latitude must be greater than -90 and less than 90. Given: {}",
                lat
            )));
        }

        if !(lng < 180f64 && lng > -180f64) {
            return Err(GeoError::InvalidCoordinate(format!(
                "Longitude must be greater than -180 and less than 180. Given: {}",
                lng
            )));
        }

        Ok(Self::from_degree_unchecked(lat, lng))
    }

    pub fn from_degree_unchecked(lat: Degree, lng: Degree) -> Self {
        LatLng {
            lat: (lat * 1e7) as i64,
            lng: (lng * 1e7) as i64,
        }
    }

    pub fn lat(&self) -> Degree {
        self.lat as f64 * 1e-7
    }

    pub fn nano_lat(&self) -> NanoDegree {
        self.lat
    }

    pub fn lng(&self) -> Degree {
        self.lng as f64 * 1e-7
    }

    pub fn nano_lng(&self) -> NanoDegree {
        self.lng
    }

    pub fn expand(&self) -> (Degree, Degree) {
        (self.lng(), self.lat())
    }

    // Returns a [`lng`, `lat`] pair
    pub fn slice(&self) -> [Degree; 2] {
        [self.lng(), self.lat()]
    }

    pub fn hash(&self, zoom: u8) -> u32 {
        let SlippyTile((_, px), (_, py), zoom) = SlippyTile::project(self, zoom);

        let hash_size = (SRID3857_MAX_LNG / 2) / 2_u32.pow(zoom as u32);
        hash_size * ((SRID3857_MAX_LNG + px) / hash_size) + ((SRID3857_MAX_LNG + py) / hash_size)
    }

    /// Offsets the `LatLng` from the given parent primitive.
    /// According to: <https://arc.net/l/quote/ccrekhxu>
    pub fn offset(&mut self, group: &PrimitiveBlock) -> &mut Self {
        let granularity = group.granularity.unwrap_or(1) as NanoDegree;

        self.lat = group.lon_offset.unwrap_or(0) + (granularity * self.lat);
        self.lng = group.lat_offset.unwrap_or(0) + (granularity * self.lng);

        self
    }

    // Delta encoding (difference only)
    pub fn delta(lat: &i64, lng: &i64, prior: Self) -> Self {
        LatLng {
            lat: lat + prior.nano_lat(),
            lng: lng + prior.nano_lng(),
        }
    }

    pub fn vec_to(&self, other: &LatLng) -> (Degree, Degree) {
        (self.lng() - other.lng(), self.lat() - other.lat())
    }

    pub fn as_vec(&self) -> Vector<Degree> {
        Vector::from(self)
    }
}

impl Debug for LatLng {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "POINT({} {})", self.lng(), self.lat())
    }
}

impl rstar::Point for LatLng {
    type Scalar = NanoDegree;
    const DIMENSIONS: usize = 2;

    fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
        LatLng {
            lng: generator(0),
            lat: generator(1),
        }
    }

    fn nth(&self, index: usize) -> Self::Scalar {
        match index {
            0 => self.lat,
            1 => self.lng,
            _ => unreachable!(),
        }
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        match index {
            0 => &mut self.lat,
            1 => &mut self.lng,
            _ => unreachable!(),
        }
    }
}
