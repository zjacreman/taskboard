#[cfg(test)]
use crate::task::{Task, TaskStatus, Priority};
#[cfg(test)]
use chrono::NaiveDate;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
pub fn sample_tasks() -> Vec<Task> {
    vec![
        Task {
            description: "Fix bug".to_string(),
            status: TaskStatus::Todo,
            priority: Priority::Medium,
            due_date: None,
            scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()),
            recurrence: Some("every week".to_string()),
            done_date: None,
            start_date: None,
            tags: vec!["work".to_string(), "urgent".to_string()],
            source_file: PathBuf::from("bugs.md"),
            line_number: 10,
        },
        Task {
            description: "Buy groceries".to_string(),
            status: TaskStatus::Todo,
            priority: Priority::None,
            due_date: Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
            scheduled_date: None,
            recurrence: None,
            done_date: None,
            start_date: None,
            tags: vec!["personal".to_string()],
            source_file: PathBuf::from("tasks.md"),
            line_number: 1,
        },
        Task {
            description: "Review PR".to_string(),
            status: TaskStatus::Done,
            priority: Priority::High,
            due_date: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
            scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 6, 9).unwrap()),
            recurrence: None,
            done_date: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
            start_date: None,
            tags: vec!["work".to_string()],
            source_file: PathBuf::from("work.md"),
            line_number: 5,
        },
    ]
}
