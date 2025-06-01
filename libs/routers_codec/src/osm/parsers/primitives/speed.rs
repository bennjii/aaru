use std::fmt;

pub type Speed = u16;

#[derive(Clone, Copy, Debug)]
pub enum SpeedValue {
    /// Speed in kilometers per hour
    Kmh(Speed),
    /// Speed in miles per hour (Multiply by 1.609344)
    Mph(Speed),
    /// Speed in knots (Multiply by 1.852)
    Knots(Speed),
    /// No speed limit (typically represented as "none" in OSM)
    None,
    /// Variable speed limit (electronic signs, etc.)
    Variable,
    /// Speed limit is inherited from a higher-level way/relation
    Inherited,
    /// Walk speed (typically 5-6 km/h)
    Walk,
}

impl SpeedValue {
    /// Shows the speed as represented in Kilometers per Hour.
    pub fn in_kmh(&self) -> Option<Speed> {
        match self {
            SpeedValue::Kmh(speed) => Some(*speed),
            SpeedValue::Mph(speed) => Some(((*speed as f64) * 1.609344) as Speed),
            SpeedValue::Knots(speed) => Some(((*speed as f64) * 1.852) as Speed),
            // Non-transformative
            _ => None,
        }
    }

    /// Parses a speed value from a given speed-string.
    /// An example value might be `50 mph`. In which case,
    /// the returned value must be `Some(Mph(50))`.
    ///
    /// To convert/standardise a speed value, you may use
    /// the `in_kmh(..)` function to represent the speed
    /// value in kilometers per hour.
    pub fn parse(value: String, unit: String) -> Option<Self> {
        let numeric = value.parse::<Speed>().ok()?;

        Some(match unit.as_str() {
            // Numeric units
            "mph" => SpeedValue::Mph(numeric),
            "kph" => SpeedValue::Kmh(numeric),
            "knots" => SpeedValue::Knots(numeric),

            // Non-numeric
            "variable" => SpeedValue::Variable,
            "inherited" => SpeedValue::Inherited,
            "none" => SpeedValue::None,
            "walk" => SpeedValue::Walk,

            // Unspecified, by default, is kph
            _ => SpeedValue::Kmh(numeric),
        })
    }
}

impl fmt::Display for SpeedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpeedValue::Kmh(speed) => write!(f, "{}", speed),
            SpeedValue::Mph(speed) => write!(f, "{} mph", speed),
            SpeedValue::Knots(speed) => write!(f, "{} knots", speed),
            SpeedValue::None => write!(f, "none"),
            SpeedValue::Variable => write!(f, "variable"),
            SpeedValue::Inherited => write!(f, "inherited"),
            SpeedValue::Walk => write!(f, "walk"),
        }
    }
}
