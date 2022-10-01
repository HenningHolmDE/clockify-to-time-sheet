use anyhow::Result;
use clockify_to_time_sheet::clockify::{retrieve_time_entries, Task};
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

fn task_by_id<'a>(tasks: &'a [Task], id: &str) -> Option<&'a Task> {
    for task in tasks {
        if task.id == id {
            return Some(task);
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string(CONFIG_FILE)?)?;

    let (time_entries, tasks) = retrieve_time_entries(
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
            task_by_id(&tasks, &time_entry.task_id).unwrap().name
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
