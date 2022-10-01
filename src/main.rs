use anyhow::Result;
use clockify_to_time_sheet::{clockify::retrieve_time_entries, transform::transform_time_entries};
use serde::Deserialize;
use std::fs;

static CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Deserialize)]
struct Config {
    api_key: String,
    user_id: String,
    workspace_id: String,
    project_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string(CONFIG_FILE)?)?;

    let time_entries = retrieve_time_entries(
        &config.api_key,
        &config.user_id,
        &config.workspace_id,
        &config.project_id,
    )
    .await?;

    let time_sheet_entries = transform_time_entries(time_entries);

    for entry in time_sheet_entries {
        println!("Description: {}", entry.description);
        println!("Start: {}", entry.start);
        println!("End: {}", entry.end);
        println!("Break: {}", entry.break_);
    }

    Ok(())
}
