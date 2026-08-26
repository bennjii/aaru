use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

/// A mode of travel a speed limit or access restriction applies to.
///
/// Two grouping modes ([`Vehicle`](TravelMode::Vehicle),
/// [`MotorVehicle`](TravelMode::MotorVehicle)) cover the concrete modes
/// beneath them; see [`TravelMode::covers`].
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[repr(u8)]
pub enum TravelMode {
    /// Any vehicle (grouping mode).
    Vehicle,
    /// Any motorised vehicle (grouping mode).
    MotorVehicle,
    Car,
    Truck,
    Motorcycle,
    Foot,
    Bicycle,
    Bus,
    Hgv,
    Hov,
    Emergency,
}

impl TravelMode {
    /// Whether a restriction scoped to `self` applies to travel by `other`.
    ///
    /// `Vehicle` covers everything except `Foot`; `MotorVehicle` covers the
    /// motorised modes. Concrete modes only cover themselves.
    pub fn covers(&self, other: TravelMode) -> bool {
        use TravelMode::*;

        if *self == other {
            return true;
        }

        match self {
            Vehicle => !matches!(other, Foot),
            MotorVehicle => matches!(
                other,
                Car | Truck | Motorcycle | Bus | Hgv | Hov | Emergency | MotorVehicle
            ),
            _ => false,
        }
    }

    /// Whether this mode represents motor-vehicle traffic. Used to decide if
    /// a directional access restriction makes a segment one-way for driving.
    pub fn is_motor_traffic(&self) -> bool {
        use TravelMode::*;
        matches!(
            self,
            Vehicle | MotorVehicle | Car | Truck | Motorcycle | Bus | Hgv | Hov | Emergency
        )
    }
}
