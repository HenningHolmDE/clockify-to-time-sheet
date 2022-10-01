use chrono::{DateTime, Local};
use reqwest::header::{self, HeaderValue};
use serde::Deserialize;
use thiserror::Error;

static CLOCKIFY_API_BASE: &str = "https://api.clockify.me/api/v1";

#[derive(Debug, Error)]
pub enum ClockifyError {
    #[error("REST API error")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON deserialization error")]
    Deserialization(#[from] serde_json::Error),
    #[error("Invalid API-Key error")]
    InvalidApiKey(#[from] reqwest::header::InvalidHeaderValue),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeInterval {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub description: String,
    pub billable: bool,
    pub task_id: String,
    pub time_interval: TimeInterval,
}

pub async fn retrieve_time_entries(
    api_key: &str,
    user_id: &str,
    workspace_id: &str,
    project_id: &str,
) -> Result<(Vec<TimeEntry>, Vec<Task>), ClockifyError> {
    // Set up REST client.
    let mut headers = header::HeaderMap::new();
    headers.insert("X-Api-Key", HeaderValue::from_str(api_key)?);
    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .user_agent("clockify-to-time-sheet")
        .build()?;

    // Get tasks from Clockify.
    let response = client
        .get(format!(
            "{}/workspaces/{}/projects/{}/tasks",
            CLOCKIFY_API_BASE, workspace_id, project_id
        ))
        .send()
        .await?;
    let response_body = response.text().await?;
    // println!("Response Body: {}", response_body);
    let tasks: Vec<Task> = serde_json::from_str(&response_body)?;
    println!("Tasks: {:?}", tasks);

    // Get time entries from Clockify.
    let mut time_entries: Vec<TimeEntry> = vec![];
    for page in 1..=1 {
        let start = "2022-09-01T00:00:00Z";
        let end = "2022-09-06T00:00:00Z";
        let response = client
            .get(format!(
                "{}/workspaces/{}/user/{}/time-entries?project={}&start={}&end={}&page={}",
                CLOCKIFY_API_BASE, workspace_id, user_id, project_id, start, end, page
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

    Ok((time_entries, tasks))
}
