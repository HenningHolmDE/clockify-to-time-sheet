use anyhow::Result;
use clockify_to_time_sheet::clockify::retrieve_time_entries;
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

    for time_entry in time_entries.iter().rev() {
        println!("Description: {}", time_entry.description);
        println!(
            "Task: {}",
            time_entry
                .task
                .as_ref()
                .map(|task| &task.name)
                .unwrap_or(&time_entry.task_id)
        );

        let duration = time_entry.time_interval.end - time_entry.time_interval.start;
        println!(
            "Time: {} - {} = {}",
            time_entry.time_interval.start, time_entry.time_interval.end, duration,
        );
        println!();
    }

    Ok(())
}
