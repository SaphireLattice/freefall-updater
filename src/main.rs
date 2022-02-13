mod data;
mod freefall;

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use chrono::TimeZone;
use chrono::Utc;
use once_cell::unsync::Lazy;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::ser::Formatter;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

// Used as a horrible hack, as PathBuf hasn't been const-ified
type LazyPath = Lazy<PathBuf, fn() -> PathBuf>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const DYNAMIC_DIR: LazyPath = Lazy::new(|| "freefall".into());
    const LOCAL_DATA_FILE: LazyPath = Lazy::new(|| DYNAMIC_DIR.join("data.json"));

    let body = reqwest::get("http://freefall.purrsia.com/fabsdata.js")
        .await
        .context("Failed to fetch upstream data")?
        .text()
        .await?;

    let data: Vec<data::FreefallEntry> = serde_json::from_str(
        body.strip_prefix("FreefallData(")
            .ok_or("Prefix match failed")?
            .strip_suffix(")")
            .ok_or("Suffix match failed")?,
    )
    .context("Failed to normalize JSONP to JSON")?;

    let mut local_data: Vec<data::ReaderEntry> =
        read_from_file(&*LOCAL_DATA_FILE).with_context(|| "Failed to read local data")?;

    let local_last = local_data.last().unwrap();

    let last_known = local_last.i;
    let last = data.last().unwrap().i;
    println!(
        "Last update check {:?} UTC",
        local_last.checked.unwrap_or_else(|| Utc.timestamp(0, 0))
    );

    if last_known == last {
        println!("Local copy up to date!");
        local_data.last_mut().unwrap().checked = Some(Utc::now());
        save_to_file(&*LOCAL_DATA_FILE, local_data, data::DataFormatter::new())?;
        return Ok(());
    }

    println!(
        "Local copy out of sync.\nLast known: {}\nLatest: {}",
        last_known, last
    );

    let mut dates: Vec<Option<String>> = if last_known % 100 != 99 {
        read_from_file(DYNAMIC_DIR.join(format!("dates_{}.json", last_known / 100)))?
    } else {
        Vec::new()
    };

    let static_dir: PathBuf = "static/freefall".into();

    for i in (last_known + 1)..=last {
        let url = if i == last {
            "http://freefall.purrsia.com/default.htm".to_string()
        } else {
            format!(
                "http://freefall.purrsia.com/ff{:02}00/{}{:05}.htm",
                (i - 1) / 100 + 1,
                "fc",
                i
            )
        };

        println!("Fetching #{}", i);
        let page = freefall::Page::new(url)
            .await
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

        let path = save_page_img(&page, bytes, &static_dir).await?;
        println!("Saved image: {:?}", path.to_str().unwrap());

        dates.push(Some((&page.date).to_string()));

        if i % 100 == 99 {
            assert_eq!(dates.len(), 100);
            let bin = i / 100;

            save_to_file(
                static_dir.join(format!("dates_{}.json", bin)),
                &dates,
                serde_json::ser::CompactFormatter,
            )?;

            let dates_file = DYNAMIC_DIR.join(format!("dates_{}.json", bin));
            fs::remove_file(&dates_file)
                .with_context(|| format!("Failed to remove {:?}", &dates_file))?;
            println!("Removed old dates: {:?}", &dates_file);

            dates = Vec::new();
        }

        println!();
    }

    if last % 100 != 99 {
        save_to_file(
            DYNAMIC_DIR.join(format!("dates_{}.json", last / 100)),
            &dates,
            serde_json::ser::CompactFormatter,
        )?;
    };

    local_data.last_mut().unwrap().i = last;
    local_data.last_mut().unwrap().checked = Some(Utc::now());
    save_to_file(&*LOCAL_DATA_FILE, local_data, data::DataFormatter::new())?;

    Ok(())
}

fn read_from_file<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> Result<Vec<T>, anyhow::Error> {
    let path: &Path = path.as_ref();
    let file = File::open(path).with_context(|| format!("Failed to open {:?}", path))?;
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).with_context(|| format!("Failed to parse {:?}", path))
}

fn save_to_file<P: AsRef<Path>, T: Serialize, F: Formatter>(
    path: P,
    data: T,
    formatter: F,
) -> Result<(), anyhow::Error> {
    let path: &Path = path.as_ref();
    let file = File::create(&path).with_context(|| format!("Failed to create {:?}", path))?;
    let writer = BufWriter::new(file);

    let mut ser = serde_json::Serializer::with_formatter(writer, formatter);
    data.serialize(&mut ser)
        .with_context(|| format!("Failed to serialise into {:?}", path))?;

    println!("Saved data: {:?}", path);

    Ok(())
    // *
    /* /

        let buf = Vec::new();
        let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
        data.serialize(&mut ser).unwrap();

        println!("[{:?}]: {}", path.as_ref(), String::from_utf8(ser.into_inner()).unwrap());

        Ok(true)
    // */
}

pub async fn save_page_img(
    page: &freefall::Page,
    bytes: Bytes,
    target_dir: &PathBuf,
) -> Result<PathBuf, anyhow::Error> {
    if !page.img_url.ends_with("png") {
        return Err(anyhow!(
            "Unsupported image type for image \"{}\"",
            page.img_url
        ));
    };
    let filename = format!("{}.{}", page.num, "png");
    let path = target_dir.join(filename);

    let mut file = File::create(&path)?;
    file.write_all(&bytes.to_vec())?;

    Ok(path)
}
