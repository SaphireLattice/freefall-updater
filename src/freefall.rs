use std::path::Path;
use bytes::Bytes;
use crate::data::ReaderDate;
use std::fmt;

use regex::Regex;

pub struct Page {
    num: i32,
    date: ReaderDate,
    img_url: String,
    extra_url: Option<String>
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Page({}, {} - {} ({:?}))", self.num, self.date, self.img_url, self.extra_url)
    }
}

impl Page {
    pub async fn new(url: String) -> Result<Self, Box<dyn std::error::Error>> {
        lazy_static! {
            static ref RE_TITLE: Regex = Regex::new(r"(?ix)
                <title>\s*
                Freefall\s+
                ([0-9]+)\s+
                ([a-z]+)\s+
                ([0-9]+),\s+
                ([0-9]+)\s*</title>
            ").unwrap();
        }

        lazy_static! {
            static ref RE_IMG: Regex = Regex::new(r#"(?ix)<img\s+src="([^.]+\.[^".]+)""#).unwrap();
        }

        let body = reqwest::get(url)
            .await?
            .text()
            .await?;

        let captures = match RE_TITLE.captures(&body) {
            Some(cap) => cap,
            None => {
                return Err("failed to parse date".into());
            }
        };

        let date = match ReaderDate::from_title(
            captures.get(4).unwrap().as_str().to_string(),
            captures.get(2).unwrap().as_str().to_string(),
            captures.get(3).unwrap().as_str().to_string()
        ) {
            Ok(v) => v,
            Err(e) => {
                return Err(e);
            }
        };

        let mut img_url: Option::<String> = None;
        let mut extra_img_url: Option::<String> = None;

        for captures in RE_IMG.captures_iter(&body) {
            let url = captures[1].to_string();
            println!("Image SRC={}", url);

            if let None = img_url {
                img_url = Some(url);
            } else if let None = extra_img_url {
                if !url.contains("") {
                    extra_img_url = Some(url);
                };
            };
        }

        Ok(Self {
            num: captures.get(1).unwrap().as_str().parse().unwrap(),
            date,
            img_url: img_url.unwrap(),
            extra_url: extra_img_url
        })
    }

    pub async fn get_img(self, extra: bool) -> Result<Bytes, Box<dyn std::error::Error>> {
        let url = format!("http://freefall.purrsia.com{}", if !extra { self.img_url } else { self.extra_url.unwrap() });
        Ok(
            reqwest::get(url)
                .await?
                .bytes()
                .await?
        )
    }

    pub async fn save_img(self, extra: bool) -> Result<Box<Path>, Box<dyn std::error::Error>> {
        let bytes = self.get_img(extra);

        Err("".into())
    }
}