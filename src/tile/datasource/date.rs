use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{de, Deserialize, Deserializer};

const STORAGE_FORMAT: &str = "%y%m%d";
const REQUEST_FORMAT: &str = "%Y-%m-%d";

pub struct DateRange(pub DateTime<Utc>, pub DateTime<Utc>);

pub fn format_date(date: &DateTime<Utc>) -> String {
    return date.format(STORAGE_FORMAT).to_string();
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Err(err) => Err(err),
        Ok(date) => match NaiveDateTime::parse_from_str(&date, REQUEST_FORMAT) {
            Err(err) => Err(de::Error::custom(err.to_string())),
            Ok(date) => Ok(Utc.from_utc_datetime(&date)),
        },
    }
}