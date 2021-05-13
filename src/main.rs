#[macro_use]
extern crate lazy_static;

mod data;
mod freefall;

use anyhow::{Context, Result, anyhow};
use serde_json::ser::Formatter;
use serde::de::DeserializeOwned;
use std::io::BufWriter;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use bytes::Bytes;
use std::io::BufReader;
use std::fs::File;
use std::path::Path;
use std::fs;

use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let local_data_file = "freefall/data.json";

    let body = reqwest::get("http://freefall.purrsia.com/fabsdata.js").await
        .context("Failed to fetch upstream data")?
        .text().await?;

    let data: Vec<data::FreefallEntry> = serde_json::from_str(
            body
                .strip_prefix("FreefallData(").ok_or("Prefix match failed")?
                .strip_suffix(")").ok_or("Suffix match failed")?
        )
        .context("Failed to normalize JSONP to JSON")?;

    let mut local_data: Vec<data::ReaderEntry> = read_from_file(local_data_file)
        .with_context(|| format!("Failed to read local reader data from {}", local_data_file))?;

    local_data.last();

    let last_known = 3498; local_data.last().unwrap().i;
    let last = 3502; data.last().unwrap().i;
    if last_known == last {
        println!("Local copy up to date!");
        return Ok(());
    }

    println!("Local copy out of sync.\nLast known: {}\nLatest: {}", last_known, last);

    let mut dates: Vec<Option<String>> = if last_known % 100 != 99 {
        read_from_file(format!("freefall/dates_{}.json", last_known / 100))
            .with_context(|| format!("Failed to read freefall/dates_{}.json", last_known / 100))?
    } else {
        let mut v = Vec::new();
        v.push(None);
        v
    };

    for i in last_known+1..last {
        let url = if i == last {
            "http://freefall.purrsia.com/default.htm".to_string()
        } else {
            format!("http://freefall.purrsia.com/ff{:02}00/{}{:05}.htm", (i - 1) / 100 + 1, "fc", i)
        };

        println!("Fetching #{}", i);
        let page = freefall::Page::new(url).await
            .context("Failed to fetch the page")?;
        println!("Got page {}", page);

        let bytes = page.get_img(false).await?;

        /*
        TODO: Implement size change detection
        let (width, height) = match blob_size(&bytes.to_vec()) {
            Ok(dim) => (dim.width, dim.height),
            Err(why) => return Err(why.into())
        };
        */

        let path = save_page_img(&page, bytes, "static/freefall/".to_string()).await?;
        println!("Saved image: {:?}", path.to_str().unwrap());

        if i % 100 == 0 {
            assert_eq!(dates.len(), 100);
            let bin = (i - 1) / 100;

            save_to_file(format!("static/freefall/dates_{}.json", bin), &dates, serde_json::ser::CompactFormatter)?;

            fs::remove_file(format!("freefall/dates_{}.json", bin))
                .with_context(|| format!("Failed to remove freefall/dates_{}.json", bin))?;
            println!("Removed old dates: freefall/dates_{}.json", bin);

            dates = Vec::new();
        }
        dates.push(Some((&page.date).to_string()));
        println!("");
    }

    let date_save_path = if last % 100 == 99 {
        format!("static/freefall/dates_{}.json", last / 100)
    } else {
        format!("freefall/dates_{}.json", last / 100)
    };
    save_to_file(&date_save_path, &dates, serde_json::ser::CompactFormatter)?;

    local_data.last_mut().unwrap().i = last;
    save_to_file(local_data_file, local_data, data::DataFormatter::new())?;

    Ok(())
}

fn read_from_file<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> Result<Vec<T>, anyhow::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let data = serde_json::from_reader(reader)?;
    Ok(data)
}

fn save_to_file<P: AsRef<Path>, T: Serialize, F: Formatter>(path: P, data: T, formatter: F) -> Result<bool, anyhow::Error> {
//*
    let file = File::create(&path)?;
    let writer = BufWriter::new(file);

    let mut ser = serde_json::Serializer::with_formatter(writer, formatter);
    data.serialize(&mut ser).unwrap();


    println!("Saved data: {:?}", path.as_ref());

    Ok(true)
// *
/* /

    let buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
    data.serialize(&mut ser).unwrap();

    println!("[{:?}]: {}", path.as_ref(), String::from_utf8(ser.into_inner()).unwrap());

    Ok(true)
// */
}


pub async fn save_page_img(page: &freefall::Page, bytes: Bytes, target_dir: String) -> Result<PathBuf, anyhow::Error> {
    let mut path = PathBuf::from(target_dir);
    if !page.img_url.ends_with("png") { return Err(anyhow!("Unsupported image type for image \"{}\"", page.img_url)); };
    let filename = format!("{}.{}", page.num, "png");
    path.push(filename);

    let mut file = File::create(&path)?;
    file.write_all(&bytes.to_vec())?;

    Ok(path)
}