use crate::transform::TimeSheetEntry;
use chrono::{DateTime, Duration, Local, Timelike};
use std::io;

/// Write given time sheet entries as CSV to the given writer. The fields are
/// formatted as required by the time sheet and time values are rounded to the
/// nearest minute and the date is only written for the first entry of a day.
pub fn write_csv<W: io::Write>(
    wtr: W,
    time_sheet_entries: &Vec<TimeSheetEntry>,
) -> Result<(), csv::Error> {
    let mut wtr = csv::Writer::from_writer(wtr);
    wtr.write_record(["date", "start", "end", "break", "description"])?;
    let mut last_date: Option<String> = None;
    for entry in time_sheet_entries {
        let date = entry.start.format("%d.%m.%y").to_string();
        let date = if last_date.as_ref() == Some(&date) {
            String::new()
        } else {
            last_date = Some(date.clone());
            date
        };
        wtr.write_record([
            &date,
            &format_time_field(&entry.start),
            &format_time_field(&entry.end),
            &format_break_field(&entry.break_),
            &entry.description,
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

/// Format a time field (start/end) to hh:mm format while rounding up to the
/// next minute, if the second is >=30. (12:30:29 -> 12:30, 12:30:30 -> 12:31)
fn format_time_field(time: &DateTime<Local>) -> String {
    let mut hour = time.hour();
    let mut minute = time.minute();
    if time.second() >= 30 {
        minute += 1;
    }
    if minute >= 60 {
        minute -= 60;
        hour += 1;
    }
    format!("{:02}:{:02}", hour, minute)
}

/// Format the break field to h:mm format while rounding up to the next minute,
/// if the second is >=30. (01:30:29 -> 1:30, 01:30:30 -> 1:31)
/// Leave the field empty, if no break is recorded for the entry.
fn format_break_field(duration: &Duration) -> String {
    if duration.num_seconds() < 30 {
        return String::new();
    }
    let mut hour = duration.num_hours();
    let mut minute = duration.num_minutes() % 60;
    if duration.num_seconds() % 60 >= 30 {
        minute += 1;
    }
    if minute >= 60 {
        minute -= 60;
        hour += 1;
    }
    format!("{}:{:02}", hour, minute)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;

    #[test]
    fn test_format_time_field_round_down() {
        let time = Local.with_ymd_and_hms(2022, 10, 1, 8, 9, 15).unwrap();
        assert_eq!(format_time_field(&time), "08:09");
        let time = Local.with_ymd_and_hms(2022, 10, 1, 11, 59, 29).unwrap();
        assert_eq!(format_time_field(&time), "11:59");
    }

    #[test]
    fn test_format_time_field_round_up() {
        let time = Local.with_ymd_and_hms(2022, 10, 1, 12, 10, 45).unwrap();
        assert_eq!(format_time_field(&time), "12:11");
        let time = Local.with_ymd_and_hms(2022, 10, 1, 9, 5, 30).unwrap();
        assert_eq!(format_time_field(&time), "09:06");
        let time = Local.with_ymd_and_hms(2022, 10, 1, 8, 59, 30).unwrap();
        assert_eq!(format_time_field(&time), "09:00");
    }

    #[test]
    fn test_format_break_field_round_down() {
        let duration = Duration::seconds(0);
        assert_eq!(format_break_field(&duration), "");
        let duration = Duration::seconds(29);
        assert_eq!(format_break_field(&duration), "");
        let duration = Duration::seconds(60);
        assert_eq!(format_break_field(&duration), "0:01");
        let duration = Duration::seconds(59 * 60);
        assert_eq!(format_break_field(&duration), "0:59");
        let duration = Duration::seconds(60 * 60);
        assert_eq!(format_break_field(&duration), "1:00");
    }

    #[test]
    fn test_format_break_field_round_up() {
        let duration = Duration::seconds(30);
        assert_eq!(format_break_field(&duration), "0:01");
        let duration = Duration::seconds(60 + 30);
        assert_eq!(format_break_field(&duration), "0:02");
        let duration = Duration::seconds(59 * 60 + 30);
        assert_eq!(format_break_field(&duration), "1:00");
        let duration = Duration::seconds(60 * 60 + 30);
        assert_eq!(format_break_field(&duration), "1:01");
    }

    #[test]
    fn test_writer() {
        let entries = vec![
            TimeSheetEntry {
                description: "Task 1".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 8, 0, 29).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 8, 59, 30).unwrap(),
                break_: Duration::zero(),
            },
            TimeSheetEntry {
                description: "Task 2".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 1, 13, 0, 31).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 1, 14, 59, 30).unwrap(),
                break_: Duration::seconds(3630),
            },
            TimeSheetEntry {
                description: "Task 3".to_string(),
                start: Local.with_ymd_and_hms(2022, 10, 2, 8, 0, 0).unwrap(),
                end: Local.with_ymd_and_hms(2022, 10, 2, 9, 0, 0).unwrap(),
                break_: Duration::zero(),
            },
        ];
        let mut buffer: Vec<u8> = Vec::new();
        write_csv(&mut buffer, &entries).unwrap();
        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            r#"date,start,end,break,description
01.10.22,08:00,09:00,,Task 1
,13:01,15:00,1:01,Task 2
02.10.22,08:00,09:00,,Task 3
"#
        );
    }
}
