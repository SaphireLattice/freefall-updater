use chrono::NaiveDate;
use std::fmt;
use serde::{Serialize, Deserialize};
use serde::de::{self, Visitor};

#[derive(Deserialize, Debug)]
pub struct FreefallEntry {
    pub i: i32,
    pub h: Option<i32>,
    pub prefix: Option<String>,
    pub ext: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReaderEntry {
    pub i: i32,
    pub single: Option<bool>,
    pub height: Option<i32>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub extra: Option<String>,
    pub extra_height: Option<i32>,
    pub extra_original: Option<String>,
}

#[derive(Debug)]
pub struct ReaderDate(NaiveDate);
struct DateVisitor;

impl ReaderDate {
    pub fn from_title(year: String, month: String, day: String) -> Result<Self, Box<dyn std::error::Error>> {
        let month = match month.get(..3).unwrap() {
            "Jan" => 1,
            "Feb" => 2,
            "Mar" => 3,
            "Apr" => 4,
            "May" => 5,
            "Jun" => 6,
            "Jul" => 7,
            "Aug" => 8,
            "Sep" => 9,
            "Oct" => 10,
            "Nov" => 11,
            "Dec" => 12,
            _ => return Err(format!(
                    "invalid month \"{}\"",
                    month
                ).into())
        };

        Ok(
            ReaderDate(
                NaiveDate::from_ymd(
                    year.parse().unwrap(), month, day.parse().unwrap()
                )
            )
        )
    }
}

impl From<NaiveDate> for ReaderDate {
    fn from(date: NaiveDate) -> Self {
        ReaderDate(date)
    }
}

impl Into<NaiveDate> for ReaderDate {
    fn into(self) -> NaiveDate {
        self.0
    }
}

impl fmt::Display for ReaderDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for ReaderDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Visitor<'de> for DateVisitor {
    type Value = ReaderDate;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a string of format YYYY-MM-DD")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            Ok(date) => Ok(ReaderDate(date)),
            Err(e) => Err(
                E::custom(e)
            )
        }
    }
}

impl<'de> Deserialize<'de> for ReaderDate {
    fn deserialize<D>(deserializer: D) -> Result<ReaderDate, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DateVisitor)
    }
}