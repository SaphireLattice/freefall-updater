use anyhow::{Result, anyhow};
use serde_json::ser::Formatter;
use chrono::{DateTime, NaiveDate, Utc};
use chrono::serde::ts_seconds_option;
use std::fmt;
use std::io;
use serde::{Serialize, Deserialize};
use serde::de::{self, Visitor};

#[derive(Deserialize, Debug)]
pub struct FreefallEntry {
    pub i: i32,
    pub h: Option<i32>,
    pub prefix: Option<String>,
    pub ext: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReaderEntry {
    pub i: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_original: Option<String>,
    #[serde(default)]
    #[serde(with = "ts_seconds_option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct ReaderDate(NaiveDate);
struct DateVisitor;

impl ReaderDate {
    pub fn from_title(year: String, month: String, day: String) -> Result<Self, anyhow::Error> {
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
            _ => return Err(anyhow!("invalid month \"{}\"", month))
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

#[derive(Clone, Debug)]
pub struct DataFormatter<'a> {
    current_indent: usize,
    has_value: bool,
    indent: &'a [u8],
}

impl<'a> DataFormatter<'a> {
    pub fn new() -> Self {
        DataFormatter {
            current_indent: 0,
            has_value: false,
            indent: b"    ",
        }
    }
}

impl<'a> Formatter for DataFormatter<'a> {
    #[inline]
    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"[")
    }

    #[inline]
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(if first { b"\n" } else { b",\n" })?;
        indent(writer, self.current_indent, self.indent)?;
        Ok(())
    }

    #[inline]
    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"{")
    }

    #[inline]
    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(if first { b" " } else { b", " })
    }

    #[inline]
    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;
        writer.write_all(b" }")
    }

    #[inline]
    fn end_array_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.has_value = true;
        Ok(())
    }

    #[inline]
    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b": ")
    }

    #[inline]
    fn end_object_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.has_value = true;
        Ok(())
    }

    #[inline]
    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;

        if self.has_value {
            writer.write_all(b"\n")?;
            indent(writer, self.current_indent, self.indent)?;
        }

        writer.write_all(b"]")
    }
}

fn indent<W>(wr: &mut W, n: usize, s: &[u8]) -> io::Result<()>
where
    W: ?Sized + io::Write,
{
    for _ in 0..n {
        wr.write_all(s)?;
    }

    Ok(())
}
