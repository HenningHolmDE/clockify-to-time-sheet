use anyhow::Result;
use clap::Parser;
use clockify_to_time_sheet::{
    clockify::retrieve_time_entries, transform::transform_time_entries, writer::write_csv,
};
use serde::Deserialize;
use std::fs;

static CONFIG_FILE: &str = "config.toml";

/// Command line arguments
#[derive(Parser, Debug)]
struct Args {
    /// Name of CSV output file (default: [YYYY]-[MM].csv)
    #[arg(short, long)]
    output: Option<String>,
    /// Year of the time entries to retrieve
    year: u32,
    /// Month of the time entries to retrieve
    month: u32,
}

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
    let args = Args::parse();

    let config: Config = toml::from_str(&fs::read_to_string(CONFIG_FILE)?)?;

    let time_entries = retrieve_time_entries(
        &config.api_key,
        &config.user_id,
        &config.workspace_id,
        &config.project_id,
        args.year,
        args.month,
    )
    .await?;

    let time_sheet_entries = transform_time_entries(time_entries);

    let file = fs::File::create(
        args.output
            .unwrap_or(format!("{}-{:02}.csv", args.year, args.month,)),
    )?;
    write_csv(file, &time_sheet_entries)?;

    Ok(())
}
