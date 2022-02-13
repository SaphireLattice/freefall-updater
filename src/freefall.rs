use crate::data::ReaderDate;
use anyhow::{anyhow, Result};
use bytes::Bytes;
use std::fmt;
use url::Url;

// Convenience macro, based on once_cell's documentation:
//   https://docs.rs/once_cell/latest/once_cell/index.html#lazily-compiled-regex
macro_rules! regex {
    ($re:expr $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

pub struct Page {
    pub num: i32,
    pub date: ReaderDate,
    pub img_url: String,
    pub extra_url: Option<String>,
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "#{} {} - {} ({:?})",
            self.num, self.date, self.img_url, self.extra_url
        )
    }
}

impl Page {
    pub async fn new(url: String) -> Result<Self, anyhow::Error> {
        let body = reqwest::get(url).await?.text().await?;

        const RE_TITLE: &'static str = r"(?ix)
                <title>\s*
                Freefall\s+
                ([0-9]+)\s+
                ([a-z]+)\s+
                ([0-9]+),\s+
                ([0-9]+)\s*</title>";

        let captures = match regex!(RE_TITLE).captures(&body) {
            Some(cap) => cap,
            None => {
                return Err(anyhow!("Failed to find date via regex: {}", body));
            }
        };

        let date = ReaderDate::from_title(
            captures.get(4).unwrap().as_str().to_string(),
            captures.get(2).unwrap().as_str().to_string(),
            captures.get(3).unwrap().as_str().to_string(),
        )?;

        let mut img_url: Option<String> = None;
        let mut extra_img_url: Option<String> = None;

        for captures in regex!(r#"(?ix)<img\s+src="([^.]+\.[^".]+)""#).captures_iter(&body) {
            let url = captures[1].to_string();
            println!("Image {}", url);

            if img_url.is_none() {
                img_url = Some(url);
            } else if extra_img_url.is_none() && !url.contains("/linkpages/") {
                extra_img_url = Some(url);
            };
        }

        Ok(Self {
            num: captures.get(1).unwrap().as_str().parse().unwrap(),
            date,
            img_url: img_url.unwrap(),
            extra_url: extra_img_url,
        })
    }

    pub async fn get_img(&self, extra: bool) -> Result<Bytes, anyhow::Error> {
        let mut url = Url::parse("http://freefall.purrsia.com")?;

        let path = if !extra {
            self.img_url.clone()
        } else {
            self.extra_url.clone().unwrap()
        };
        url.set_path(&path);

        let bytes = reqwest::get(url).await?.bytes().await?;

        Ok(bytes)
    }
}
