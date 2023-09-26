use crate::clockify::TimeEntry;
use chrono::{DateTime, Duration, Local};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeSheetEntry {
    pub description: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub break_: Duration,
}

/// Transform Clockify time entries into time sheet entries.
/// - Convert entries into into `TimeSheetEntry` by extracting the corresponding
///   information.
/// - Merge subsequent entries with equal description by using in the `break_`
///   field accordingly.
pub fn transform_time_entries(time_entries: Vec<TimeEntry>) -> Vec<TimeSheetEntry> {
    merge_time_sheet_entries(convert_time_entries(time_entries))
}

/// Convert Clockify time entries into `TimeSheetEntry` by extracting the
/// corresponding information.
/// Use `task.name` as the description for the time sheet entry, if available.
/// Fall back to using `description`, if no task is available.
fn convert_time_entries(time_entries: Vec<TimeEntry>) -> Vec<TimeSheetEntry> {
    time_entries
        .into_iter()
        .rev() // reverse as Clockify starts newest entry first
        .map(|entry| TimeSheetEntry {
            description: entry
                .task
                .map(|task| task.name)
                .unwrap_or(entry.description),
            start: entry.time_interval.start,
            end: entry.time_interval.end,
            break_: Duration::zero(),
        })
        .collect()
}

/// Merge subsequent time sheet entries with equal descriptions.
/// - Time sheet entries are not merged across date boundaries.
/// - With each merge, the `Duration` in the `break_` field is increased by the
///   time between the end of the first and the start of the second entry.
///   This way, the correct total of the list is kept.
/// - If descriptions alternate, entries are not merged as this would result
///   in time sheet entries overlapping each other. While the total of the list
///   would still be correct in this case due to the break times, this causes
///   the list to become hardly readable.
fn merge_time_sheet_entries(time_entries: Vec<TimeSheetEntry>) -> Vec<TimeSheetEntry> {
    let mut result: Vec<TimeSheetEntry> = Vec::with_capacity(time_entries.len());
    for entry in time_entries {
        if let Some(last) = result.last_mut() {
            if last.description == entry.description
                && last.end.date_naive() == entry.end.date_naive()
            {
                last.break_ = last.break_ + (entry.start - last.end);
                last.end = entry.end;
            } else {
                result.push(entry);
            }
        } else {
            result.push(entry);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clockify::*;
    use chrono::prelude::*;

    #[test]
    fn test_convert_simple_entries_use_task_name_reverted() {
        let time_entries = vec![
            TimeEntry {
                description: "Entry 2".to_string(),
                billable: true,
                task_id: Some("ghijkl".to_string()),
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 14, 45, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 15, 15, 15).unwrap(),
                },
                task: Some(Task {
                    id: "ghijkl".to_string(),
                    name: "Task 2".to_string(),
                }),
            },
            TimeEntry {
                description: "Entry 1".to_string(),
                billable: true,
                task_id: Some("abcdef".to_string()),
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
                },
                task: Some(Task {
                    id: "abcdef".to_string(),
                    name: "Task 1".to_string(),
                }),
            },
        ];
        let expected_result = vec![
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: time_entries[1].time_interval.start,
                end: time_entries[1].time_interval.end,
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 2".to_string(),
                start: time_entries[0].time_interval.start,
                end: time_entries[0].time_interval.end,
                break_: Duration::zero(),
            },
        ];
        let result = convert_time_entries(time_entries);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_convert_missing_task_uses_description() {
        let time_entries = vec![TimeEntry {
            description: "Entry 1".to_string(),
            billable: true,
            task_id: None,
            time_interval: TimeInterval {
                start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
            },
            task: None,
        }];
        let expected_result = vec![TimeSheetEntry {
            description: "Entry 1".to_string(),
            start: time_entries[0].time_interval.start,
            end: time_entries[0].time_interval.end,
            break_: Duration::zero(),
        }];
        let result = convert_time_entries(time_entries);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_merge_subsequent_time_sheet_entries_of_same_task() {
        let time_sheet_entries = vec![
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 14, 45, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 15, 15, 15).unwrap(),
                break_: Duration::zero(),
            },
        ];
        let expected_result = vec![TimeSheetEntry {
            description: "Task 1".to_string(),
            // Start of first entry.
            start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
            // Start of last entry.
            end: Local.with_ymd_and_hms(2022, 10, 1, 15, 15, 15).unwrap(),
            // Break from 12:25:30 to 14:45:00 -> 2:19:30 = 8370 sec.
            break_: Duration::seconds(8370),
        }];
        let result = merge_time_sheet_entries(time_sheet_entries);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_merge_keep_time_sheet_entries_of_alternating_tasks() {
        let time_sheet_entries = vec![
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 2".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 13, 0, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 13, 30, 0).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 14, 45, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 15, 15, 15).unwrap(),
                break_: Duration::zero(),
            },
        ];
        let expected_result = time_sheet_entries.clone();
        let result = merge_time_sheet_entries(time_sheet_entries);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_merge_keep_time_sheet_entries_of_different_days() {
        let time_sheet_entries = vec![
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 2, 14, 45, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 2, 15, 15, 15).unwrap(),
                break_: Duration::zero(),
            },
        ];
        let expected_result = time_sheet_entries.clone();
        let result = merge_time_sheet_entries(time_sheet_entries);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_merge_subsequent_time_sheet_entries_of_same_task_multiple_breaks() {
        let time_sheet_entries = vec![
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 14, 45, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 15, 15, 15).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 16, 0, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 16, 15, 0).unwrap(),
                break_: Duration::zero(),
            },
        ];
        let expected_result = vec![TimeSheetEntry {
            description: "Task 1".to_string(),
            // Start of first entry.
            start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
            // Start of last entry.
            end: Local.with_ymd_and_hms(2022, 10, 1, 16, 15, 0).unwrap(),
            // Break from 12:25:30 to 14:45:00 -> 2:19:30 = 8370 sec.
            // Break from 15:15:15 to 16:00:00 -> 0:44:45 = 2685 sec.
            break_: Duration::seconds(8370 + 2685),
        }];
        let result = merge_time_sheet_entries(time_sheet_entries);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_transform_complex_time_entries_example() {
        let time_entries = vec![
            TimeEntry {
                description: "Entry 6".to_string(),
                billable: true,
                task_id: Some("abcdef".to_string()),
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 16, 0, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 17, 0, 0).unwrap(),
                },
                task: Some(Task {
                    id: "abcdef".to_string(),
                    name: "Task 1".to_string(),
                }),
            },
            TimeEntry {
                description: "Entry 5".to_string(),
                billable: true,
                task_id: None,
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 15, 50, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 15, 55, 0).unwrap(),
                },
                task: None,
            },
            TimeEntry {
                description: "Entry 5".to_string(),
                billable: true,
                task_id: None,
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 15, 30, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 15, 45, 0).unwrap(),
                },
                task: None,
            },
            TimeEntry {
                description: "Entry 4".to_string(),
                billable: true,
                task_id: Some("ghijkl".to_string()),
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 15, 5, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 15, 10, 30).unwrap(),
                },
                task: Some(Task {
                    id: "ghijkl".to_string(),
                    name: "Task 2".to_string(),
                }),
            },
            TimeEntry {
                description: "Entry 3".to_string(),
                billable: true,
                task_id: Some("abcdef".to_string()),
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 14, 45, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 15, 0, 15).unwrap(),
                },
                task: Some(Task {
                    id: "abcdef".to_string(),
                    name: "Task 1".to_string(),
                }),
            },
            TimeEntry {
                description: "Entry 2".to_string(),
                billable: true,
                task_id: Some("abcdef".to_string()),
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
                },
                task: Some(Task {
                    id: "abcdef".to_string(),
                    name: "Task 1".to_string(),
                }),
            },
            TimeEntry {
                description: "Entry 1".to_string(),
                billable: true,
                task_id: Some("abcdef".to_string()),
                time_interval: TimeInterval {
                    start: Local.with_ymd_and_hms(2022, 9, 30, 12, 10, 0).unwrap(),
                    end: Local.with_ymd_and_hms(2022, 9, 30, 12, 25, 30).unwrap(),
                },
                task: Some(Task {
                    id: "abcdef".to_string(),
                    name: "Task 1".to_string(),
                }),
            },
        ];
        let expected_result = vec![
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 9, 30, 12, 10, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 9, 30, 12, 25, 30).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 15, 0, 15).unwrap(),
                break_: Local.with_ymd_and_hms(2022, 10, 1, 14, 45, 0).unwrap()
                    - Local.with_ymd_and_hms(2022, 10, 1, 12, 25, 30).unwrap(),
            },
            TimeSheetEntry {
                description: "Task 2".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 15, 5, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 15, 10, 30).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Entry 5".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 15, 30, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 15, 55, 0).unwrap(),
                break_: Local.with_ymd_and_hms(2022, 10, 1, 15, 50, 0).unwrap()
                    - Local.with_ymd_and_hms(2022, 10, 1, 15, 45, 0).unwrap(),
            },
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 16, 0, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 17, 0, 0).unwrap(),
                break_: Duration::zero(),
            },
        ];
        let result = transform_time_entries(time_entries);
        assert_eq!(result, expected_result);
    }
}
