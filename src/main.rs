use anyhow::Result;
use clockify_to_time_sheet::{
    clockify::retrieve_time_entries, transform::transform_time_entries, writer::write_csv,
};
use serde::Deserialize;
use std::fs;

static CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Deserialize)]
struct Config {
    api_key: String,
    // TODO: User ID and workspace ID (for default workspace) should be read
    //       via the Clockify API.
    user_id: String,
    workspace_id: String,
    // TODO: Project name should be provided via command line argument and ID
    //       should be looked up via the Clockify API.
    project_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string(CONFIG_FILE)?)?;

    // TODO: Year and month should be provided via command line parameters.
    let year = 2022u32;
    let month = 9u32;

    let time_entries = retrieve_time_entries(
        &config.api_key,
        &config.user_id,
        &config.workspace_id,
        &config.project_id,
        year,
        month,
    )
    .await?;

    let time_sheet_entries = transform_time_entries(time_entries);

    // TODO: It should be possible to change the name of the CSV file via a
    //       command line parameter.
    let file = fs::File::create(format!("{}-{:02}.csv", year, month))?;
    write_csv(file, &time_sheet_entries)?;

    Ok(())
}
