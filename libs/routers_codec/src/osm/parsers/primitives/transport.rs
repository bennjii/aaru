use strum::{AsRefStr, Display, EnumIter, EnumString};

/// Land-based transportation modes with hierarchical structure
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum LandTransportMode {
    /// General access restriction for all land-based transport
    Access,

    // === Non-vehicle transport ===
    /// Pedestrians on foot
    #[strum(serialize = "foot")]
    Foot,
    /// Dogs being walked (usually by pedestrians)
    #[strum(serialize = "dog")]
    Dog,
    /// Cross-country skiing
    #[strum(serialize = "ski")]
    Ski,
    /// Nordic/cross-country skiing specifically
    #[strum(serialize = "ski:nordic")]
    SkiNordic,
    /// Alpine/downhill skiing
    #[strum(serialize = "ski:alpine")]
    SkiAlpine,
    /// Telemark skiing (free-heel)
    #[strum(serialize = "ski:telemark")]
    SkiTelemark,
    /// Inline skating/rollerblading
    #[strum(serialize = "inline_skates")]
    InlineSkates,
    /// Horse riders/equestrians
    #[strum(serialize = "horse")]
    Horse,
    /// Person carrying a boat (portage)
    #[strum(serialize = "portage")]
    Portage,

    // === Vehicle transport (top-level category) ===
    /// Any vehicle (motorized or non-motorized)
    #[strum(serialize = "vehicle")]
    Vehicle(VehicleType),
}

/// Vehicle types - encompasses both motorized and non-motorized vehicles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum VehicleType {
    /// Non-motorized vehicles
    NonMotorized(NonMotorizedVehicle),
    /// Motorized vehicles
    Motorized(MotorizedVehicle),
}

/// Non-motorized vehicle types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum NonMotorizedVehicle {
    /// Single-tracked non-motorized vehicles
    SingleTracked(NonMotorizedSingleTrack),
    /// Double-tracked non-motorized vehicles
    DoubleTracked(NonMotorizedDoubleTrack),
}

/// Single-tracked non-motorized vehicles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum NonMotorizedSingleTrack {
    /// Bicycles/cyclists
    #[strum(serialize = "bicycle")]
    Bicycle,
    /// Electric bicycles (speed limited, e.g., 25 km/h)
    #[strum(serialize = "electric_bicycle")]
    ElectricBicycle,
    /// Mountain bikes (when different restrictions apply)
    #[strum(serialize = "mtb")]
    Mtb,
    /// Cargo bikes for transporting heavy/bulky loads
    #[strum(serialize = "cargo_bike")]
    CargoBike,
    /// Kick scooters (non-motorized, human-powered)
    #[strum(serialize = "kick_scooter")]
    KickScooter,
}

/// Double-tracked non-motorized vehicles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum NonMotorizedDoubleTrack {
    /// Horse-drawn carriages or other animal-drawn vehicles
    #[strum(serialize = "carriage")]
    Carriage,
    /// Human-powered pedal vehicles with 2 tracks
    #[strum(serialize = "cycle_rickshaw")]
    CycleRickshaw,
    /// Hand-pulled or hand-pushed carts
    #[strum(serialize = "hand_cart")]
    HandCart,
    /// Trailers that need to be towed by another vehicle
    #[strum(serialize = "trailer")]
    Trailer,
    /// Travel trailers/caravans
    #[strum(serialize = "caravan")]
    Caravan,
}

/// Motorized vehicle types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum MotorizedVehicle {
    /// All motorized vehicles (top-level category)
    #[strum(serialize = "motor_vehicle")]
    MotorVehicle,
    /// Single-tracked motorized vehicles
    SingleTracked(MotorizedSingleTrack),
    /// Double-tracked motorized vehicles
    DoubleTracked(MotorizedDoubleTrack),
    /// Vehicles categorized by use/purpose
    ByUse(VehicleByUse),
}

/// Single-tracked motorized vehicles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum MotorizedSingleTrack {
    /// Motorcycles (2-wheeled, allowed on motorways)
    #[strum(serialize = "motorcycle")]
    Motorcycle,
    /// Mopeds (motorized bicycles, speed restricted, ~50cc/45km/h)
    #[strum(serialize = "moped")]
    Moped,
    /// Speed pedelecs (electric bikes up to 45km/h, license required)
    #[strum(serialize = "speed_pedelec")]
    SpeedPedelec,
    /// Mofa (low performance moped, max ~25km/h)
    #[strum(serialize = "mofa")]
    Mofa,
    /// Small electric vehicles (electric scooters, 20-30km/h)
    #[strum(serialize = "small_electric_vehicle")]
    SmallElectricVehicle,
}

/// Double-tracked motorized vehicles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum MotorizedDoubleTrack {
    /// Cars/automobiles (generic double-tracked motor vehicles)
    #[strum(serialize = "motorcar")]
    Motorcar,
    /// Motorhomes/RVs
    #[strum(serialize = "motorhome")]
    Motorhome,
    /// Tourist buses (long-distance, non-public transport)
    #[strum(serialize = "tourist_bus")]
    TouristBus,
    /// Coaches (long-distance bus travel)
    #[strum(serialize = "coach")]
    Coach,
    /// Light commercial vehicles (goods vehicles ≤3.5 tonnes)
    #[strum(serialize = "goods")]
    Goods,
    /// Heavy goods vehicles (>3.5 tonnes)
    #[strum(serialize = "hgv")]
    Hgv,
    /// Articulated heavy goods vehicles
    #[strum(serialize = "hgv_articulated")]
    HgvArticulated,
    /// B-double trucks (EuroCombi, up to 60t)
    #[strum(serialize = "bdouble")]
    Bdouble,
    /// Agricultural motor vehicles (tractors, etc., 25km/h limit)
    #[strum(serialize = "agricultural")]
    Agricultural,
    /// 3-wheeled motorized vehicles
    #[strum(serialize = "auto_rickshaw")]
    AutoRickshaw,
    /// Neighborhood electric vehicles (small, low-speed)
    #[strum(serialize = "nev")]
    Nev,
    /// Golf carts and similar small electric vehicles
    #[strum(serialize = "golf_cart")]
    GolfCart,
    /// Microcars/light quadricycles
    #[strum(serialize = "microcar")]
    Microcar,
    /// All-terrain vehicles/quads (≤1.27m width)
    #[strum(serialize = "atv")]
    Atv,
    /// Off-highway vehicles (unlicensed off-road)
    #[strum(serialize = "ohv")]
    Ohv,
    /// Snowmobiles
    #[strum(serialize = "snowmobile")]
    Snowmobile,
}

/// Vehicles categorized by use/purpose rather than physical characteristics
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum VehicleByUse {
    /// Public service vehicles (general category)
    #[strum(serialize = "psv")]
    Psv,
    /// Buses (heavy public service vehicles)
    #[strum(serialize = "bus")]
    Bus,
    /// Taxis
    #[strum(serialize = "taxi")]
    Taxi,
    /// Minibuses (light public service vehicles)
    #[strum(serialize = "minibus")]
    Minibus,
    /// Share taxis (demand responsive transit)
    #[strum(serialize = "share_taxi")]
    ShareTaxi,
    /// High-occupancy vehicles/carpools
    #[strum(serialize = "hov")]
    Hov,
    /// Carpool vehicles (alternative to hov)
    #[strum(serialize = "carpool")]
    Carpool,
    /// Car sharing vehicles
    #[strum(serialize = "car_sharing")]
    CarSharing,
    /// Emergency vehicles (ambulance, fire, police)
    #[strum(serialize = "emergency")]
    Emergency,
    /// Vehicles carrying hazardous materials
    #[strum(serialize = "hazmat")]
    Hazmat,
    /// Vehicles carrying water-polluting materials
    #[strum(serialize = "hazmat:water")]
    HazmatWater,
    /// School buses
    #[strum(serialize = "school_bus")]
    SchoolBus,
    /// Blue badge/disabled permit holders
    #[strum(serialize = "disabled")]
    Disabled,
}

/// Water-based transportation modes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum WaterTransportMode {
    /// General water-based access
    Access,
    /// Swimming (use without craft)
    #[strum(serialize = "swimming")]
    Swimming,
    /// Ice skating (when water is frozen)
    #[strum(serialize = "ice_skates")]
    IceSkates,
    /// Small boats and pleasure crafts (<20m in CEVNI)
    #[strum(serialize = "boat")]
    Boat,
    /// Motor boats and yachts using motor
    #[strum(serialize = "motorboat")]
    Motorboat,
    /// Sailing boats using sails (not motor)
    #[strum(serialize = "sailboat")]
    Sailboat,
    /// Boats without sail/motor (canoes, kayaks, dinghies)
    #[strum(serialize = "canoe")]
    Canoe,
    /// Fishing vessels of any size
    #[strum(serialize = "fishing_vessel")]
    FishingVessel,
    /// Commercial vessels of any size
    #[strum(serialize = "ship")]
    Ship,
    /// Passenger ships (ferries, cruise ships)
    #[strum(serialize = "passenger")]
    Passenger,
    /// Cargo ships (general)
    #[strum(serialize = "cargo")]
    Cargo,
    /// Dry bulk cargo ships
    #[strum(serialize = "bulk")]
    Bulk,
    /// Wet bulk cargo/compressed gas tankers
    #[strum(serialize = "tanker")]
    Tanker,
    /// Gas tankers (compressed/liquefied gas)
    #[strum(serialize = "tanker:gas")]
    TankerGas,
    /// Oil tankers (crude oil and products)
    #[strum(serialize = "tanker:oil")]
    TankerOil,
    /// Chemical tankers
    #[strum(serialize = "tanker:chemical")]
    TankerChemical,
    /// Single hull tankers (more restrictive rules)
    #[strum(serialize = "tanker:singlehull")]
    TankerSinglehull,
    /// Container ships
    #[strum(serialize = "container")]
    Container,
    /// IMDG dangerous cargo ships
    #[strum(serialize = "imdg")]
    Imdg,
    /// ISPS regulated ships
    #[strum(serialize = "isps")]
    Isps,
}

/// Rail-based transportation modes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum RailTransportMode {
    /// General rail-based access
    Access,
    // Note: Specific rail transport modes would be added here
    // but the wiki section was truncated. Common ones might include:
    // Train, Tram, Metro, etc.
}

/// Complete transport mode enumeration covering all categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, EnumString, EnumIter, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum TransportMode {
    /// Land-based transportation
    Land(LandTransportMode),
    /// Water-based transportation
    Water(WaterTransportMode),
    /// Rail-based transportation
    Rail(RailTransportMode),
}
