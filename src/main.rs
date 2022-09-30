use anyhow::Result;
use chrono::{DateTime, Local};
use reqwest::header::{self, HeaderValue};
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    id: String,
    name: String,
}

fn task_by_id<'a>(tasks: &'a [Task], id: &str) -> Option<&'a Task> {
    for task in tasks {
        if task.id == id {
            return Some(task);
        }
    }
    None
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimeInterval {
    start: DateTime<Local>,
    end: DateTime<Local>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimeEntry {
    description: String,
    billable: bool,
    task_id: String,
    time_interval: TimeInterval,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string(CONFIG_FILE)?)?;

    let mut headers = header::HeaderMap::new();
    headers.insert("X-Api-Key", HeaderValue::from_str(&config.api_key)?);

    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .user_agent("Time entry importer")
        .build()?;

    let response = client
        .get(format!(
            "https://api.clockify.me/api/v1/workspaces/{}/projects/{}/tasks",
            config.workspace_id, config.project_id
        ))
        .send()
        .await?;
    let response_body = response.text().await?;
    // println!("Response Body: {}", response_body);
    let tasks: Vec<Task> = serde_json::from_str(&response_body)?;
    println!("Tasks: {:?}", tasks);

    let mut time_entries: Vec<TimeEntry> = vec![];
    for page in 1..=1 {
        let start = "2022-09-01T00:00:00Z";
        let end = "2022-09-06T00:00:00Z";
        let response = client
        .get(format!(
            "https://api.clockify.me/api/v1/workspaces/{}/user/{}/time-entries?project={}&start={}&end={}&page={}",
            config.workspace_id, config.user_id, config.project_id, start, end, page
        ))
        .send()
        .await?;
        let response_body = response.text().await?;
        // println!("Response Body: {}", response_body);
        let entries: Vec<TimeEntry> = serde_json::from_str(&response_body)?;
        println!("Entries: {:?}", entries);
        if entries.len() == 0 {
            break;
        }
        time_entries.extend(entries);
    }

    for time_entry in time_entries.iter().rev() {
        println!("Description: {}", time_entry.description);
        println!(
            "Task: {}",
            task_by_id(&tasks, &time_entry.task_id).unwrap().name
        );
        // TODO: Round start and end time to nearest 5 minutes.
        let duration = time_entry.time_interval.end - time_entry.time_interval.start;
        println!(
            "Time: {} - {} = {}",
            time_entry.time_interval.start, time_entry.time_interval.end, duration,
        );
        println!();
    }

    Ok(())
}
