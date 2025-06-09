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
    /// Get the specificity level (depth in hierarchy) - higher means more specific
    pub fn specificity_level(&self) -> usize {
        self.hierarchy_chain().len()
    }

    /// Returns the hierarchy chain from most general to most specific
    #[inline]
    const fn hierarchy_chain(&self) -> &'static [TransportMode] {
        use TransportMode::*;

        match self {
            // === LAND TRANSPORT ===

            // General access
            LandAccess => &[LandAccess],

            // Non-vehicle transport (directly under land access)
            Foot => &[LandAccess, Foot],
            Dog => &[LandAccess, Dog],
            Ski => &[LandAccess, Ski],
            SkiNordic => &[LandAccess, Ski, SkiNordic],
            SkiAlpine => &[LandAccess, Ski, SkiAlpine],
            SkiTelemark => &[LandAccess, Ski, SkiTelemark],
            InlineSkates => &[LandAccess, InlineSkates],
            Horse => &[LandAccess, Horse],
            Portage => &[LandAccess, Portage],

            // Non-motorized vehicles
            Vehicle => &[LandAccess, Vehicle],

            // Non-motorized single-tracked vehicles
            Bicycle => &[LandAccess, Vehicle, Bicycle],
            ElectricBicycle => &[LandAccess, Vehicle, Bicycle, ElectricBicycle],
            Mtb => &[LandAccess, Vehicle, Bicycle, Mtb],
            CargoBike => &[LandAccess, Vehicle, Bicycle, CargoBike],
            KickScooter => &[LandAccess, Vehicle, KickScooter],

            // Non-motorized double-tracked vehicles
            Carriage => &[LandAccess, Vehicle, Carriage],
            CycleRickshaw => &[LandAccess, Vehicle, CycleRickshaw],
            HandCart => &[LandAccess, Vehicle, HandCart],
            Trailer => &[LandAccess, Vehicle, Trailer],
            Caravan => &[LandAccess, Vehicle, Caravan],

            // Motor vehicles
            MotorVehicle => &[LandAccess, Vehicle, MotorVehicle],

            // Motorized single-tracked vehicles
            Motorcycle => &[LandAccess, Vehicle, MotorVehicle, Motorcycle],
            Moped => &[LandAccess, Vehicle, MotorVehicle, Moped],
            SpeedPedelec => &[LandAccess, Vehicle, MotorVehicle, SpeedPedelec],
            Mofa => &[LandAccess, Vehicle, MotorVehicle, Mofa],
            SmallElectricVehicle => &[LandAccess, Vehicle, MotorVehicle, SmallElectricVehicle],

            // Motorized double-tracked vehicles
            Motorcar => &[LandAccess, Vehicle, MotorVehicle, Motorcar],
            Motorhome => &[LandAccess, Vehicle, MotorVehicle, Motorhome],
            TouristBus => &[LandAccess, Vehicle, MotorVehicle, TouristBus],
            Coach => &[LandAccess, Vehicle, MotorVehicle, Coach],
            Goods => &[LandAccess, Vehicle, MotorVehicle, Goods],
            Hgv => &[LandAccess, Vehicle, MotorVehicle, Goods, Hgv],
            HgvArticulated => &[
                LandAccess,
                Vehicle,
                MotorVehicle,
                Goods,
                Hgv,
                HgvArticulated,
            ],
            Bdouble => &[LandAccess, Vehicle, MotorVehicle, Goods, Hgv, Bdouble],
            Agricultural => &[LandAccess, Vehicle, MotorVehicle, Agricultural],
            AutoRickshaw => &[LandAccess, Vehicle, MotorVehicle, AutoRickshaw],
            Nev => &[LandAccess, Vehicle, MotorVehicle, Nev],
            GolfCart => &[LandAccess, Vehicle, MotorVehicle, GolfCart],
            Microcar => &[LandAccess, Vehicle, MotorVehicle, Microcar],
            Atv => &[LandAccess, Vehicle, MotorVehicle, Atv],
            Ohv => &[LandAccess, Vehicle, MotorVehicle, Ohv],
            Snowmobile => &[LandAccess, Vehicle, MotorVehicle, Snowmobile],

            // Vehicles by use/purpose (these often inherit from motor_vehicle or specific types)
            Psv => &[LandAccess, Vehicle, MotorVehicle, Psv],
            Bus => &[LandAccess, Vehicle, MotorVehicle, Psv, Bus],
            Taxi => &[LandAccess, Vehicle, MotorVehicle, Taxi],
            Minibus => &[LandAccess, Vehicle, MotorVehicle, Psv, Bus, Minibus],
            ShareTaxi => &[LandAccess, Vehicle, MotorVehicle, Taxi, ShareTaxi],
            Hov => &[LandAccess, Vehicle, MotorVehicle, Hov],
            Carpool => &[LandAccess, Vehicle, MotorVehicle, Hov, Carpool],
            CarSharing => &[LandAccess, Vehicle, MotorVehicle, CarSharing],
            Emergency => &[LandAccess, Vehicle, MotorVehicle, Emergency],
            Hazmat => &[LandAccess, Vehicle, MotorVehicle, Hazmat],
            HazmatWater => &[LandAccess, Vehicle, MotorVehicle, Hazmat, HazmatWater],
            SchoolBus => &[LandAccess, Vehicle, MotorVehicle, Psv, Bus, SchoolBus],
            Disabled => &[LandAccess, Vehicle, MotorVehicle, Disabled],

            // === WATER TRANSPORT ===
            WaterAccess => &[WaterAccess],
            Swimming => &[WaterAccess, Swimming],
            IceSkates => &[WaterAccess, IceSkates],

            Boat => &[WaterAccess, Boat],
            Motorboat => &[WaterAccess, Boat, Motorboat],
            Sailboat => &[WaterAccess, Boat, Sailboat],
            Canoe => &[WaterAccess, Boat, Canoe],
            FishingVessel => &[WaterAccess, Boat, FishingVessel],

            Ship => &[WaterAccess, Boat, Ship],
            Passenger => &[WaterAccess, Boat, Ship, Passenger],
            Cargo => &[WaterAccess, Boat, Ship, Cargo],
            Bulk => &[WaterAccess, Boat, Ship, Cargo, Bulk],
            Tanker => &[WaterAccess, Boat, Ship, Cargo, Tanker],
            TankerGas => &[WaterAccess, Boat, Ship, Cargo, Tanker, TankerGas],
            TankerOil => &[WaterAccess, Boat, Ship, Cargo, Tanker, TankerOil],
            TankerChemical => &[WaterAccess, Boat, Ship, Cargo, Tanker, TankerChemical],
            TankerSinglehull => &[WaterAccess, Boat, Ship, Cargo, Tanker, TankerSinglehull],
            Container => &[WaterAccess, Boat, Ship, Cargo, Container],
            Imdg => &[WaterAccess, Boat, Ship, Imdg],
            Isps => &[WaterAccess, Boat, Ship, Isps],

            // === RAIL TRANSPORT ===
            RailAccess => &[RailAccess],
            Train => &[RailAccess, Train],
            Tram => &[RailAccess, Tram],
            Metro => &[RailAccess, Metro],
        }
    }

    /// Check if this transport mode is affected by a restriction on the given mode
    pub fn is_restricted_by(&self, restriction_mode: TransportMode) -> bool {
        self.hierarchy_chain().contains(&restriction_mode)
    }

    /// Check if this transport mode matches any of the given restriction modes
    pub fn matches_any_restriction(&self, restrictions: &[TransportMode]) -> bool {
        let my_chain = self.hierarchy_chain();
        restrictions
            .iter()
            .any(|restriction| my_chain.contains(restriction))
    }

    /// Get all parent modes (excluding self)
    fn parent_modes(&self) -> &'static [TransportMode] {
        let chain = self.hierarchy_chain();
        if chain.len() > 1 {
            &chain[..chain.len() - 1]
        } else {
            &[]
        }
    }

    /// Check if this mode is a parent of another mode
    #[cfg(test)]
    fn is_parent_of(&self, other: TransportMode) -> bool {
        other.parent_modes().contains(self)
    }

    /// Get the immediate parent (most specific parent)
    pub fn immediate_parent(&self) -> Option<TransportMode> {
        let parents = self.parent_modes();
        parents.last().copied()
    }

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
