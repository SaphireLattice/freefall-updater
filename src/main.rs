#[macro_use]
extern crate lazy_static;

mod data;
mod freefall;
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let body = reqwest::get("http://freefall.purrsia.com/fabsdata.js").await?
        .text().await?;

    let data: Vec<data::FreefallEntry> = serde_json::from_str(
        body
            .strip_prefix("FreefallData(").ok_or("Prefix match failed")?
            .strip_suffix(")").ok_or("Suffix match failed")?
        )?;

    println!("data = {:?}", data);

    let dataa: Vec<data::ReaderDate> = serde_json::from_str("[\"2021-01-01\", \"2020-01-02\"]")?;

    println!("data = {:?}", dataa);

    let serialized = serde_json::to_string(&dataa).unwrap();
    println!("serialized = {}", serialized);

    let last_known = 3579;
    let last = data.last().unwrap().i;
    if last_known == last {
        return Ok(());
    }

    for i in last_known+1..=last {
        let url = if i == last {
            "http://freefall.purrsia.com/default.htm".to_string()
        } else {
            format!("http://freefall.purrsia.com/ff{:02}00/{}{:05}.htm", i / 100 + 1, "fc", i)
        };
        let page = freefall::Page::new(url).await?;
        println!("page = {}", page);
    }

    let page = freefall::Page::new("http://freefall.purrsia.com/default.htm".to_string()).await?;
    println!("page = {}", page);
    let page = freefall::Page::new("http://freefall.purrsia.com/ff3600/fc03588.htm".to_string()).await?;
    println!("page = {}", page);

    Ok(())
}


