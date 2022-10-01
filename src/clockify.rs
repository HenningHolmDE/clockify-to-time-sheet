use chrono::{DateTime, Local};
use reqwest::header::{self, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;
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

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeInterval {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub description: String,
    pub billable: bool,
    pub task_id: String,
    pub time_interval: TimeInterval,
    pub task: Option<Task>,
}

/// Resolve task IDs in time entries to corresponding tasks and populate `task`
/// fields with task data.
fn resolve_task_ids(time_entries: Vec<TimeEntry>, tasks: Vec<Task>) -> Vec<TimeEntry> {
    // Convert task list into a hash map for faster lookup.
    let tasks_map = tasks
        .into_iter()
        .map(|task| (task.id.clone(), task))
        .collect::<HashMap<_, _>>();

    // Clone corresponding tasks into `task` fields of time entries.
    time_entries
        .into_iter()
        .map(|mut entry| {
            entry.task = tasks_map.get(&entry.task_id).cloned();
            entry
        })
        .collect()
}

/// Retrieve time entries for the given project from Clockify.
pub async fn retrieve_time_entries(
    api_key: &str,
    user_id: &str,
    workspace_id: &str,
    project_id: &str,
) -> Result<Vec<TimeEntry>, ClockifyError> {
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
    let tasks: Vec<Task> = serde_json::from_str(&response_body)?;

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
        let entries: Vec<TimeEntry> = serde_json::from_str(&response_body)?;
        if entries.len() == 0 {
            break;
        }
        time_entries.extend(entries);
    }

    Ok(resolve_task_ids(time_entries, tasks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_task_ids() {
        let tasks = vec![
            Task {
                id: "abcdef".to_string(),
                name: "Task 1".to_string(),
            },
            Task {
                id: "ghijkl".to_string(),
                name: "Task 2".to_string(),
            },
        ];
        let time_entries = vec![
            TimeEntry {
                description: "Entry 1".to_string(),
                billable: true,
                task_id: "abcdef".to_string(),
                time_interval: TimeInterval {
                    start: Local::now(),
                    end: Local::now(),
                },
                task: None,
            },
            TimeEntry {
                description: "Entry 2".to_string(),
                billable: true,
                task_id: "ghijkl".to_string(),
                time_interval: TimeInterval {
                    start: Local::now(),
                    end: Local::now(),
                },
                task: None,
            },
        ];
        let mut expected_result = time_entries.clone();
        expected_result[0].task = Some(tasks[0].clone());
        expected_result[1].task = Some(tasks[1].clone());
        let result = resolve_task_ids(time_entries, tasks);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_resolve_task_ids_unknown_id() {
        let tasks = vec![Task {
            id: "ghijkl".to_string(),
            name: "Task 2".to_string(),
        }];
        let time_entries = vec![TimeEntry {
            description: "Entry 1".to_string(),
            billable: true,
            task_id: "abcdef".to_string(),
            time_interval: TimeInterval {
                start: Local::now(),
                end: Local::now(),
            },
            task: None,
        }];
        let expected_result = time_entries.clone();
        let result = resolve_task_ids(time_entries, tasks);
        assert_eq!(result, expected_result);
    }
}
