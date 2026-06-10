use super::{Task, TaskStatus, Priority};
use chrono::NaiveDate;
use std::path::Path;

pub fn parse_file(content: &str, source_file: &Path) -> Vec<Task> {
    let mut tasks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        if let Some(task) = parse_line(line, line_num + 1, source_file) {
            tasks.push(task);
        }
    }

    tasks
}

fn parse_line(line: &str, line_number: usize, source_file: &Path) -> Option<Task> {
    let trimmed = line.trim();

    // Must start with "- [ ]" or "- [x]"
    let (status, rest) = if let Some(stripped) = trimmed.strip_prefix("- [ ]") {
        (TaskStatus::Todo, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("- [x]").or_else(|| trimmed.strip_prefix("- [X]")) {
        (TaskStatus::Done, stripped)
    } else {
        return None;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let mut description = String::new();
    let mut due_date = None;
    let mut scheduled_date = None;
    let mut recurrence = None;
    let mut done_date = None;
    let mut start_date = None;
    let mut priority = Priority::None;
    let mut tags = Vec::new();

    let mut chars = rest.chars().peekable();
    let mut current_word = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '📅' => {
                description.push_str(current_word.trim());
                current_word.clear();
                due_date = parse_date_from_iter(&mut chars);
            }
            '🛫' => {
                description.push_str(current_word.trim());
                current_word.clear();
                start_date = parse_date_from_iter(&mut chars);
            }
            '🔁' => {
                description.push_str(current_word.trim());
                current_word.clear();
                recurrence = Some(parse_until_emoji_or_end(&mut chars));
            }
            '✅' => {
                description.push_str(current_word.trim());
                current_word.clear();
                done_date = parse_date_from_iter(&mut chars);
            }
            '⏳' => {
                description.push_str(current_word.trim());
                current_word.clear();
                scheduled_date = parse_date_from_iter(&mut chars);
            }
            '🔺' | '⏫' | '🔼' | '🔽' | '⏬' => {
                description.push_str(current_word.trim());
                current_word.clear();
                priority = Priority::from_emoji(&ch.to_string()).unwrap_or(Priority::None);
            }
            '#' => {
                description.push_str(current_word.trim());
                current_word.clear();
                let tag = parse_tag(&mut chars);
                if !tag.is_empty() {
                    tags.push(tag);
                }
            }
            _ => {
                current_word.push(ch);
            }
        }
    }

    if !current_word.trim().is_empty() {
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str(current_word.trim());
    }

    Some(Task {
        description: description.trim().to_string(),
        status,
        priority,
        due_date,
        scheduled_date,
        recurrence,
        done_date,
        start_date,
        tags,
        source_file: source_file.to_path_buf(),
        line_number,
    })
}

fn parse_date_from_iter(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<NaiveDate> {
    let mut date_str = String::new();

    // Skip whitespace
    while chars.peek() == Some(&' ') {
        chars.next();
    }

    // Read date (YYYY-MM-DD or YYYY/MM/DD)
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() || ch == '-' || ch == '/' {
            date_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    // Try parsing with different formats
    NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(&date_str, "%Y/%m/%d"))
        .ok()
}

fn parse_until_emoji_or_end(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut text = String::new();

    // Skip whitespace
    while chars.peek() == Some(&' ') {
        chars.next();
    }

    while let Some(&ch) = chars.peek() {
        if is_emoji(ch) || ch == '#' {
            break;
        }
        text.push(ch);
        chars.next();
    }

    text.trim().to_string()
}

fn parse_tag(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut tag = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '/' {
            tag.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    tag
}

fn is_emoji(ch: char) -> bool {
    // Check for common task emojis
    matches!(ch, '📅' | '🛫' | '🔁' | '✅' | '⏳' | '🔺' | '⏫' | '🔼' | '🔽' | '⏬')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_basic_tasks() {
        let content = std::fs::read_to_string("tests/fixtures/basic.md").unwrap();
        let tasks = parse_file(&content, &PathBuf::from("tests/fixtures/basic.md"));

        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[0].description, "Buy groceries");
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(tasks[1].description, "Review pull request");
        assert_eq!(tasks[1].status, TaskStatus::Done);
    }

    #[test]
    fn test_parse_full_metadata() {
        let content = std::fs::read_to_string("tests/fixtures/full_metadata.md").unwrap();
        let tasks = parse_file(&content, &PathBuf::from("tests/fixtures/full_metadata.md"));

        let due_task = &tasks[0];
        assert_eq!(due_task.description, "Task with due date");
        assert_eq!(due_task.due_date, Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()));

        let scheduled_task = &tasks[1];
        assert_eq!(scheduled_task.scheduled_date, Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()));

        let recurrence_task = &tasks[2];
        assert_eq!(recurrence_task.recurrence, Some("every week".to_string()));

        let priority_task = &tasks[3];
        assert_eq!(priority_task.priority, Priority::High);

        let all_fields = &tasks[4];
        assert_eq!(all_fields.due_date, Some(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()));
        assert_eq!(all_fields.scheduled_date, Some(NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()));
        assert_eq!(all_fields.recurrence, Some("every week".to_string()));
        assert_eq!(all_fields.priority, Priority::High);

        let done_task = &tasks[5];
        assert_eq!(done_task.done_date, Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()));

        let tagged_task = &tasks[6];
        assert_eq!(tagged_task.tags, vec!["work", "urgent"]);
    }

    #[test]
    fn test_parse_edge_cases() {
        let content = std::fs::read_to_string("tests/fixtures/edge_cases.md").unwrap();
        let tasks = parse_file(&content, &PathBuf::from("tests/fixtures/edge_cases.md"));

        // Should parse tasks with extra spaces
        assert!(tasks.iter().any(|t| t.description == "Task with extra spaces"));

        // Should handle special characters
        assert!(tasks.iter().any(|t| t.description.contains("<>&\"'")));

        // Should find nested sub-tasks
        assert!(tasks.iter().any(|t| t.description == "Sub-task"));
        assert!(tasks.iter().any(|t| t.description == "Done sub-task"));

        // Should not include non-task lines
        assert!(!tasks.iter().any(|t| t.description == "Not a task (no checkbox)"));
    }

    #[test]
    fn test_line_numbers() {
        let content = "- [ ] First\n- [ ] Second\n- [x] Third";
        let tasks = parse_file(content, &PathBuf::from("test.md"));

        assert_eq!(tasks[0].line_number, 1);
        assert_eq!(tasks[1].line_number, 2);
        assert_eq!(tasks[2].line_number, 3);
    }
}
