use chrono::NaiveDate;
use std::path::PathBuf;

pub mod parser;
pub mod query;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    Done,
}

impl TaskStatus {
    pub fn cycle(&mut self) {
        *self = match self {
            TaskStatus::Todo => TaskStatus::Done,
            TaskStatus::Done => TaskStatus::Todo,
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    None,
    Lowest,
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn cycle(&mut self) {
        *self = match self {
            Priority::None => Priority::High,
            Priority::High => Priority::Medium,
            Priority::Medium => Priority::Low,
            Priority::Low => Priority::Lowest,
            Priority::Lowest => Priority::None,
        };
    }

    pub fn from_emoji(s: &str) -> Option<Priority> {
        match s {
            "⏫" => Some(Priority::High),
            "🔼" => Some(Priority::Medium),
            "🔽" => Some(Priority::Low),
            "⏬" => Some(Priority::Lowest),
            _ => None,
        }
    }

    pub fn to_emoji(&self) -> &str {
        match self {
            Priority::High => "⏫",
            Priority::Medium => "🔼",
            Priority::Low => "🔽",
            Priority::Lowest => "⏬",
            Priority::None => "",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub scheduled_date: Option<NaiveDate>,
    pub recurrence: Option<String>,
    pub done_date: Option<NaiveDate>,
    pub start_date: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub source_file: PathBuf,
    pub line_number: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_cycle() {
        let mut status = TaskStatus::Todo;
        status.cycle();
        assert_eq!(status, TaskStatus::Done);
        status.cycle();
        assert_eq!(status, TaskStatus::Todo);
    }

    #[test]
    fn test_priority_cycle() {
        let mut priority = Priority::None;
        priority.cycle();
        assert_eq!(priority, Priority::High);
        priority.cycle();
        assert_eq!(priority, Priority::Medium);
        priority.cycle();
        assert_eq!(priority, Priority::Low);
        priority.cycle();
        assert_eq!(priority, Priority::Lowest);
        priority.cycle();
        assert_eq!(priority, Priority::None);
    }

    #[test]
    fn test_priority_emoji_roundtrip() {
        assert_eq!(Priority::from_emoji("⏫"), Some(Priority::High));
        assert_eq!(Priority::from_emoji("🔼"), Some(Priority::Medium));
        assert_eq!(Priority::from_emoji("🔽"), Some(Priority::Low));
        assert_eq!(Priority::from_emoji("⏬"), Some(Priority::Lowest));
        assert_eq!(Priority::from_emoji("x"), None);

        assert_eq!(Priority::High.to_emoji(), "⏫");
        assert_eq!(Priority::None.to_emoji(), "");
    }
}
