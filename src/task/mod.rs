use chrono::NaiveDate;
use std::path::PathBuf;

pub mod parser;
pub mod query;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    None,
    Lowest,
    Low,
    Medium,
    High,
    Highest,
}

impl Priority {
    pub fn cycle(&mut self) {
        *self = match self {
            Priority::None => Priority::Highest,
            Priority::Highest => Priority::High,
            Priority::High => Priority::Medium,
            Priority::Medium => Priority::Low,
            Priority::Low => Priority::Lowest,
            Priority::Lowest => Priority::None,
        };
    }

    pub fn from_emoji(s: &str) -> Option<Priority> {
        match s {
            "🔺" => Some(Priority::Highest),
            "⏫" => Some(Priority::High),
            "🔼" => Some(Priority::Medium),
            "🔽" => Some(Priority::Low),
            "⏬" => Some(Priority::Lowest),
            _ => None,
        }
    }

    pub fn to_emoji(self) -> &'static str {
        match self {
            Priority::Highest => "🔺",
            Priority::High => "⏫",
            Priority::Medium => "🔼",
            Priority::Low => "🔽",
            Priority::Lowest => "⏬",
            Priority::None => "",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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

impl Task {
    pub fn to_markdown(&self) -> String {
        let status = match self.status {
            TaskStatus::Todo => "- [ ]",
            TaskStatus::Done => "- [x]",
        };

        let mut parts = vec![status.to_string(), self.description.clone()];

        let priority_emoji = self.priority.to_emoji();
        if !priority_emoji.is_empty() {
            parts.push(priority_emoji.to_string());
        }

        if let Some(due) = self.due_date {
            parts.push(format!("📅 {}", due.format("%Y-%m-%d")));
        }

        if let Some(scheduled) = self.scheduled_date {
            parts.push(format!("⏳ {}", scheduled.format("%Y-%m-%d")));
        }

        if let Some(start) = self.start_date {
            parts.push(format!("🛫 {}", start.format("%Y-%m-%d")));
        }

        if let Some(ref recurrence) = self.recurrence {
            parts.push(format!("🔁 {}", recurrence));
        }

        if let Some(done) = self.done_date {
            parts.push(format!("✅ {}", done.format("%Y-%m-%d")));
        }

        for tag in &self.tags {
            parts.push(format!("#{}", tag));
        }

        parts.join(" ")
    }

    pub fn write_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(&self.source_file)?;
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

        if self.line_number == 0 || self.line_number > lines.len() {
            return Err(format!("Invalid line number {}", self.line_number).into());
        }

        let idx = self.line_number - 1;
        let old_line = &lines[idx];

        let indent = old_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();
        lines[idx] = format!("{}{}", indent, self.to_markdown());

        std::fs::write(&self.source_file, lines.join("\n"))?;
        Ok(())
    }
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
        assert_eq!(priority, Priority::Highest);
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
        assert_eq!(Priority::from_emoji("🔺"), Some(Priority::Highest));
        assert_eq!(Priority::from_emoji("⏫"), Some(Priority::High));
        assert_eq!(Priority::from_emoji("🔼"), Some(Priority::Medium));
        assert_eq!(Priority::from_emoji("🔽"), Some(Priority::Low));
        assert_eq!(Priority::from_emoji("⏬"), Some(Priority::Lowest));
        assert_eq!(Priority::from_emoji("x"), None);

        assert_eq!(Priority::Highest.to_emoji(), "🔺");
        assert_eq!(Priority::High.to_emoji(), "⏫");
        assert_eq!(Priority::None.to_emoji(), "");
    }

    fn make_task(description: &str, status: TaskStatus) -> Task {
        Task {
            description: description.to_string(),
            status,
            priority: Priority::None,
            due_date: None,
            scheduled_date: None,
            recurrence: None,
            done_date: None,
            start_date: None,
            tags: vec![],
            source_file: PathBuf::from("test.md"),
            line_number: 1,
        }
    }

    #[test]
    fn test_to_markdown_basic_todo() {
        let task = make_task("Buy groceries", TaskStatus::Todo);
        assert_eq!(task.to_markdown(), "- [ ] Buy groceries");
    }

    #[test]
    fn test_to_markdown_basic_done() {
        let task = make_task("Review PR", TaskStatus::Done);
        assert_eq!(task.to_markdown(), "- [x] Review PR");
    }

    #[test]
    fn test_to_markdown_with_priority() {
        let mut task = make_task("Urgent task", TaskStatus::Todo);
        task.priority = Priority::High;
        assert_eq!(task.to_markdown(), "- [ ] Urgent task ⏫");
    }

    #[test]
    fn test_to_markdown_with_due_date() {
        let mut task = make_task("Deadline", TaskStatus::Todo);
        task.due_date = Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap());
        assert_eq!(task.to_markdown(), "- [ ] Deadline 📅 2026-06-15");
    }

    #[test]
    fn test_to_markdown_with_all_metadata() {
        let task = Task {
            description: "Full task".to_string(),
            status: TaskStatus::Done,
            priority: Priority::High,
            due_date: Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
            scheduled_date: Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()),
            start_date: Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()),
            recurrence: Some("every week".to_string()),
            done_date: Some(NaiveDate::from_ymd_opt(2026, 6, 14).unwrap()),
            tags: vec!["work".to_string(), "urgent".to_string()],
            source_file: PathBuf::from("test.md"),
            line_number: 1,
        };
        assert_eq!(
            task.to_markdown(),
            "- [x] Full task ⏫ 📅 2026-06-15 ⏳ 2026-06-12 🛫 2026-06-10 🔁 every week ✅ 2026-06-14 #work #urgent"
        );
    }

    #[test]
    fn test_to_markdown_with_tags() {
        let mut task = make_task("Tagged", TaskStatus::Todo);
        task.tags = vec!["personal".to_string()];
        assert_eq!(task.to_markdown(), "- [ ] Tagged #personal");
    }

    #[test]
    fn test_write_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(&file_path, "# Header\n- [ ] Old task\n- [x] Done task\n").unwrap();

        let task = Task {
            description: "Updated task".to_string(),
            status: TaskStatus::Done,
            priority: Priority::None,
            due_date: None,
            scheduled_date: None,
            recurrence: None,
            done_date: Some(NaiveDate::from_ymd_opt(2026, 6, 11).unwrap()),
            start_date: None,
            tags: vec![],
            source_file: file_path.clone(),
            line_number: 2,
        };

        task.write_to_file().unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            content,
            "# Header\n- [x] Updated task ✅ 2026-06-11\n- [x] Done task"
        );
    }

    #[test]
    fn test_write_to_file_preserves_indent() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(&file_path, "  - [ ] Indented task\n").unwrap();

        let task = Task {
            description: "Still indented".to_string(),
            status: TaskStatus::Todo,
            priority: Priority::None,
            due_date: None,
            scheduled_date: None,
            recurrence: None,
            done_date: None,
            start_date: None,
            tags: vec![],
            source_file: file_path.clone(),
            line_number: 1,
        };

        task.write_to_file().unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "  - [ ] Still indented");
    }
}
