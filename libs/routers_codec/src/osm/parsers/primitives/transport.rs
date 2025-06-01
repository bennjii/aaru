use strum::{AsRefStr, Display, EnumString};

/// Flattened transport mode enumeration for easy string parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum TransportMode {
    // === LAND TRANSPORT ===

    // General access
    #[strum(serialize = "access")]
    LandAccess,

    // Non-vehicle transport
    #[strum(serialize = "foot")]
    Foot,
    #[strum(serialize = "dog")]
    Dog,
    #[strum(serialize = "ski")]
    Ski,
    #[strum(serialize = "ski:nordic")]
    SkiNordic,
    #[strum(serialize = "ski:alpine")]
    SkiAlpine,
    #[strum(serialize = "ski:telemark")]
    SkiTelemark,
    #[strum(serialize = "inline_skates")]
    InlineSkates,
    #[strum(serialize = "horse")]
    Horse,
    #[strum(serialize = "portage")]
    Portage,

    // Non-motorized single-tracked vehicles
    #[strum(serialize = "bicycle")]
    Bicycle,
    #[strum(serialize = "electric_bicycle")]
    ElectricBicycle,
    #[strum(serialize = "mtb")]
    Mtb,
    #[strum(serialize = "cargo_bike")]
    CargoBike,
    #[strum(serialize = "kick_scooter")]
    KickScooter,

    // Non-motorized double-tracked vehicles
    #[strum(serialize = "carriage")]
    Carriage,
    #[strum(serialize = "cycle_rickshaw")]
    CycleRickshaw,
    #[strum(serialize = "hand_cart")]
    HandCart,
    #[strum(serialize = "trailer")]
    Trailer,
    #[strum(serialize = "caravan")]
    Caravan,

    // General vehicle categories
    #[strum(serialize = "vehicle")]
    Vehicle,
    #[strum(serialize = "motor_vehicle")]
    MotorVehicle,

    // Motorized single-tracked vehicles
    #[strum(serialize = "motorcycle")]
    Motorcycle,
    #[strum(serialize = "moped")]
    Moped,
    #[strum(serialize = "speed_pedelec")]
    SpeedPedelec,
    #[strum(serialize = "mofa")]
    Mofa,
    #[strum(serialize = "small_electric_vehicle")]
    SmallElectricVehicle,

    // Motorized double-tracked vehicles
    #[strum(serialize = "motorcar")]
    Motorcar,
    #[strum(serialize = "motorhome")]
    Motorhome,
    #[strum(serialize = "tourist_bus")]
    TouristBus,
    #[strum(serialize = "coach")]
    Coach,
    #[strum(serialize = "goods")]
    Goods,
    #[strum(serialize = "hgv")]
    Hgv,
    #[strum(serialize = "hgv_articulated")]
    HgvArticulated,
    #[strum(serialize = "bdouble")]
    Bdouble,
    #[strum(serialize = "agricultural")]
    Agricultural,
    #[strum(serialize = "auto_rickshaw")]
    AutoRickshaw,
    #[strum(serialize = "nev")]
    Nev,
    #[strum(serialize = "golf_cart")]
    GolfCart,
    #[strum(serialize = "microcar")]
    Microcar,
    #[strum(serialize = "atv")]
    Atv,
    #[strum(serialize = "ohv")]
    Ohv,
    #[strum(serialize = "snowmobile")]
    Snowmobile,

    // Vehicles by use/purpose
    #[strum(serialize = "psv")]
    Psv,
    #[strum(serialize = "bus")]
    Bus,
    #[strum(serialize = "taxi")]
    Taxi,
    #[strum(serialize = "minibus")]
    Minibus,
    #[strum(serialize = "share_taxi")]
    ShareTaxi,
    #[strum(serialize = "hov")]
    Hov,
    #[strum(serialize = "carpool")]
    Carpool,
    #[strum(serialize = "car_sharing")]
    CarSharing,
    #[strum(serialize = "emergency")]
    Emergency,
    #[strum(serialize = "hazmat")]
    Hazmat,
    #[strum(serialize = "hazmat:water")]
    HazmatWater,
    #[strum(serialize = "school_bus")]
    SchoolBus,
    #[strum(serialize = "disabled")]
    Disabled,

    // === WATER TRANSPORT ===
    #[strum(serialize = "water_access")]
    WaterAccess,
    #[strum(serialize = "swimming")]
    Swimming,
    #[strum(serialize = "ice_skates")]
    IceSkates,
    #[strum(serialize = "boat")]
    Boat,
    #[strum(serialize = "motorboat")]
    Motorboat,
    #[strum(serialize = "sailboat")]
    Sailboat,
    #[strum(serialize = "canoe")]
    Canoe,
    #[strum(serialize = "fishing_vessel")]
    FishingVessel,
    #[strum(serialize = "ship")]
    Ship,
    #[strum(serialize = "passenger")]
    Passenger,
    #[strum(serialize = "cargo")]
    Cargo,
    #[strum(serialize = "bulk")]
    Bulk,
    #[strum(serialize = "tanker")]
    Tanker,
    #[strum(serialize = "tanker:gas")]
    TankerGas,
    #[strum(serialize = "tanker:oil")]
    TankerOil,
    #[strum(serialize = "tanker:chemical")]
    TankerChemical,
    #[strum(serialize = "tanker:singlehull")]
    TankerSinglehull,
    #[strum(serialize = "container")]
    Container,
    #[strum(serialize = "imdg")]
    Imdg,
    #[strum(serialize = "isps")]
    Isps,

    // === RAIL TRANSPORT ===
    #[strum(serialize = "rail_access")]
    RailAccess,
    // Add more rail-specific modes as needed
    #[strum(serialize = "train")]
    Train,
    #[strum(serialize = "tram")]
    Tram,
    #[strum(serialize = "metro")]
    Metro,
}

impl TransportMode {
    /// Check if this is a land-based transport mode
    pub fn is_land(&self) -> bool {
        matches!(
            self,
            Self::LandAccess
                | Self::Foot
                | Self::Dog
                | Self::Ski
                | Self::SkiNordic
                | Self::SkiAlpine
                | Self::SkiTelemark
                | Self::InlineSkates
                | Self::Horse
                | Self::Portage
                | Self::Bicycle
                | Self::ElectricBicycle
                | Self::Mtb
                | Self::CargoBike
                | Self::KickScooter
                | Self::Carriage
                | Self::CycleRickshaw
                | Self::HandCart
                | Self::Trailer
                | Self::Caravan
                | Self::Vehicle
                | Self::MotorVehicle
                | Self::Motorcycle
                | Self::Moped
                | Self::SpeedPedelec
                | Self::Mofa
                | Self::SmallElectricVehicle
                | Self::Motorcar
                | Self::Motorhome
                | Self::TouristBus
                | Self::Coach
                | Self::Goods
                | Self::Hgv
                | Self::HgvArticulated
                | Self::Bdouble
                | Self::Agricultural
                | Self::AutoRickshaw
                | Self::Nev
                | Self::GolfCart
                | Self::Microcar
                | Self::Atv
                | Self::Ohv
                | Self::Snowmobile
                | Self::Psv
                | Self::Bus
                | Self::Taxi
                | Self::Minibus
                | Self::ShareTaxi
                | Self::Hov
                | Self::Carpool
                | Self::CarSharing
                | Self::Emergency
                | Self::Hazmat
                | Self::HazmatWater
                | Self::SchoolBus
                | Self::Disabled
        )
    }

    /// Check if this is a water-based transport mode
    pub fn is_water(&self) -> bool {
        matches!(
            self,
            Self::WaterAccess
                | Self::Swimming
                | Self::IceSkates
                | Self::Boat
                | Self::Motorboat
                | Self::Sailboat
                | Self::Canoe
                | Self::FishingVessel
                | Self::Ship
                | Self::Passenger
                | Self::Cargo
                | Self::Bulk
                | Self::Tanker
                | Self::TankerGas
                | Self::TankerOil
                | Self::TankerChemical
                | Self::TankerSinglehull
                | Self::Container
                | Self::Imdg
                | Self::Isps
        )
    }

    /// Check if this is a rail-based transport mode
    pub fn is_rail(&self) -> bool {
        matches!(
            self,
            Self::RailAccess | Self::Train | Self::Tram | Self::Metro
        )
    }

    /// Check if this is a motorized vehicle
    pub fn is_motorized(&self) -> bool {
        matches!(
            self,
            Self::MotorVehicle
                | Self::Motorcycle
                | Self::Moped
                | Self::SpeedPedelec
                | Self::Mofa
                | Self::SmallElectricVehicle
                | Self::Motorcar
                | Self::Motorhome
                | Self::TouristBus
                | Self::Coach
                | Self::Goods
                | Self::Hgv
                | Self::HgvArticulated
                | Self::Bdouble
                | Self::Agricultural
                | Self::AutoRickshaw
                | Self::Nev
                | Self::GolfCart
                | Self::Microcar
                | Self::Atv
                | Self::Ohv
                | Self::Snowmobile
                | Self::Psv
                | Self::Bus
                | Self::Taxi
                | Self::Minibus
                | Self::ShareTaxi
                | Self::Emergency
                | Self::Motorboat
        )
    }

    /// Check if this is a non-motorized vehicle
    pub fn is_non_motorized(&self) -> bool {
        matches!(
            self,
            Self::Bicycle
                | Self::ElectricBicycle
                | Self::Mtb
                | Self::CargoBike
                | Self::KickScooter
                | Self::Carriage
                | Self::CycleRickshaw
                | Self::HandCart
                | Self::Trailer
                | Self::Caravan
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_parsing() {
        // Test some common transport modes
        assert_eq!(
            TransportMode::from_str("foot").unwrap(),
            TransportMode::Foot
        );
        assert_eq!(
            TransportMode::from_str("bicycle").unwrap(),
            TransportMode::Bicycle
        );
        assert_eq!(
            TransportMode::from_str("motorcar").unwrap(),
            TransportMode::Motorcar
        );
        assert_eq!(
            TransportMode::from_str("boat").unwrap(),
            TransportMode::Boat
        );
        assert_eq!(
            TransportMode::from_str("hazmat:water").unwrap(),
            TransportMode::HazmatWater
        );

        // Test serialization
        assert_eq!(TransportMode::Foot.to_string(), "foot");
        assert_eq!(TransportMode::HazmatWater.to_string(), "hazmat:water");
    }

    #[test]
    fn test_categorization() {
        assert!(TransportMode::Foot.is_land());
        assert!(TransportMode::Boat.is_water());
        assert!(TransportMode::Train.is_rail());

        assert!(TransportMode::Motorcar.is_motorized());
        assert!(TransportMode::Bicycle.is_non_motorized());

        assert!(!TransportMode::Foot.is_motorized());
        assert!(!TransportMode::Foot.is_non_motorized());
    }
}
