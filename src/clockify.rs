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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    id: String,
    active_workspace: String,
}

#[derive(Clone, Debug)]
pub struct ApiUser {
    api_key: String,
    user: User,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeInterval {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub description: String,
    pub billable: bool,
    pub task_id: Option<String>,
    pub time_interval: TimeInterval,
    pub task: Option<Task>,
}

/// Retrieve user ID and active workspace ID from Clockify API.
pub async fn get_api_user(api_key: &str) -> Result<ApiUser, ClockifyError> {
    let client = build_client(api_key)?;

    // Get user/workspace info from Clockify.
    let response = client
        .get(format!("{CLOCKIFY_API_BASE}/user"))
        .send()
        .await?;
    let response_body = response.text().await?;
    Ok(ApiUser {
        api_key: api_key.to_owned(),
        user: serde_json::from_str(&response_body)?,
    })
}

/// Retrieve time entries for the given project from Clockify.
pub async fn retrieve_time_entries(
    api_user: &ApiUser,
    project_id: &str,
    year: u32,
    month: u32,
) -> Result<Vec<TimeEntry>, ClockifyError> {
    let client = build_client(&api_user.api_key)?;

    // Get tasks from Clockify.
    let response = client
        .get(format!(
            "{}/workspaces/{}/projects/{}/tasks",
            CLOCKIFY_API_BASE, api_user.user.active_workspace, project_id
        ))
        .send()
        .await?;
    let response_body = response.text().await?;
    let tasks: Vec<Task> = serde_json::from_str(&response_body)?;

    // Get time entries from Clockify.
    let mut time_entries: Vec<TimeEntry> = vec![];
    let start_end_range = StartEndRange::from_year_and_month(year, month);
    let (start, end) = (start_end_range.start(), start_end_range.end());
    // The API delivers 50 entries per page. Limiting retrieval to 5 page
    // requests in case something goes wrong, results in a maximum of 250
    // entries to be received. However, this might not be enough for everyone.
    // TODO: The maximum number of pages should be configurable. Note that at
    //       some point, the API limit of 10 requests per second will kick in
    //       and will have to be handled.
    for page in 1..=5 {
        let response = client
            .get(format!(
                "{}/workspaces/{}/user/{}/time-entries?project={}&start={}&end={}&page={}",
                CLOCKIFY_API_BASE,
                api_user.user.active_workspace,
                api_user.user.id,
                project_id,
                start,
                end,
                page
            ))
            .send()
            .await?;
        let response_body = response.text().await?;
        let entries: Vec<TimeEntry> = serde_json::from_str(&response_body)?;
        if entries.is_empty() {
            break;
        }
        time_entries.extend(entries);
    }

    Ok(resolve_task_ids(time_entries, tasks))
}

/// Build a reqwest client for accessing the API.
fn build_client(api_key: &str) -> Result<reqwest::Client, ClockifyError> {
    let mut headers = header::HeaderMap::new();
    headers.insert("X-Api-Key", HeaderValue::from_str(api_key)?);
    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .user_agent("clockify-to-time-sheet")
        .build()?;
    Ok(client)
}

/// Struct for providing the start and end filter values for limiting the time
/// entries query to the given year and month.
struct StartEndRange {
    year: u32,
    month: u32,
}

impl StartEndRange {
    fn from_year_and_month(year: u32, month: u32) -> Self {
        Self { year, month }
    }
    fn start(&self) -> String {
        format!("{}-{:02}-01T00:00:00Z", self.year, self.month)
    }
    fn end(&self) -> String {
        let (year, month) = if self.month == 12 {
            (self.year + 1, 1)
        } else {
            (self.year, self.month + 1)
        };
        format!("{year}-{month:02}-01T00:00:00Z")
    }
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
            if let Some(ref task_id) = entry.task_id {
                entry.task = tasks_map.get(task_id).cloned();
            }
            entry
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_end_range_start() {
        let range = StartEndRange::from_year_and_month(2022, 9);
        assert_eq!(range.start(), "2022-09-01T00:00:00Z");
        let range = StartEndRange::from_year_and_month(1999, 12);
        assert_eq!(range.start(), "1999-12-01T00:00:00Z");
    }

    #[test]
    fn test_start_end_range_end() {
        let range = StartEndRange::from_year_and_month(1999, 9);
        assert_eq!(range.end(), "1999-10-01T00:00:00Z");
        let range = StartEndRange::from_year_and_month(2022, 11);
        assert_eq!(range.end(), "2022-12-01T00:00:00Z");
        let range = StartEndRange::from_year_and_month(2022, 12);
        assert_eq!(range.end(), "2023-01-01T00:00:00Z");
    }

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
                task_id: Some("abcdef".to_string()),
                time_interval: TimeInterval {
                    start: Local::now(),
                    end: Local::now(),
                },
                task: None,
            },
            TimeEntry {
                description: "Entry 2".to_string(),
                billable: true,
                task_id: Some("ghijkl".to_string()),
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
        let time_entries = vec![
            TimeEntry {
                description: "Entry 1".to_string(),
                billable: true,
                task_id: Some("abcdef".to_string()),
                time_interval: TimeInterval {
                    start: Local::now(),
                    end: Local::now(),
                },
                task: None,
            },
            TimeEntry {
                description: "Entry 2".to_string(),
                billable: true,
                task_id: None,
                time_interval: TimeInterval {
                    start: Local::now(),
                    end: Local::now(),
                },
                task: None,
            },
        ];
        let expected_result = time_entries.clone();
        let result = resolve_task_ids(time_entries, tasks);
        assert_eq!(result, expected_result);
    }
}
