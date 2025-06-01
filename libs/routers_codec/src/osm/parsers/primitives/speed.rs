use std::fmt;

type Speed = u16;

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
