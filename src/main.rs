use anyhow::Result;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimeInterval {
    start: String,
    end: String,
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

    let start = "2022-09-01T00:00:00Z";
    let end = "2022-09-02T00:00:00Z";
    let page = 1;
    let response = client
    .get(format!(
        "https://api.clockify.me/api/v1/workspaces/{}/user/{}/time-entries?project={}&start={}&end={}&page={}",
        config.workspace_id, config.user_id, config.project_id, start, end, page
    ))
    .send()
    .await?;
    let response_body = response.text().await?;
    // println!("Response Body: {}", response_body);
    let time_entries: Vec<TimeEntry> = serde_json::from_str(&response_body)?;
    println!("Time entries: {:?}", time_entries);

    Ok(())
}
