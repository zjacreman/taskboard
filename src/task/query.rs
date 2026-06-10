use super::{Task, TaskStatus, Priority};
use chrono::NaiveDate;

#[derive(Debug)]
pub enum Filter {
    Done,
    NotDone,
    Includes(String),
    DescriptionIncludes(String),
    Tag(String),
    Folder(String),
    DueBefore(NaiveDate),
    DueAfter(NaiveDate),
    DueOn(NaiveDate),
    ScheduledBefore(NaiveDate),
    ScheduledAfter(NaiveDate),
    ScheduledOn(NaiveDate),
    HappensBefore(NaiveDate),
    HappensAfter(NaiveDate),
    HappensOn(NaiveDate),
    PriorityAbove(Priority),
    PriorityBelow(Priority),
    PriorityIs(Priority),
    HasRecurrence,
    RecurrenceIncludes(String),
    Limit(usize),
}

#[derive(Debug)]
pub enum SortField {
    Due,
    Scheduled,
    Priority,
    Description,
    Tag,
    Folder,
    Done,
    Created,
}

#[derive(Debug)]
pub struct Query {
    pub filters: Vec<Filter>,
    pub sort_by: Option<SortField>,
    pub group_by: Option<SortField>,
}

pub fn execute_query(query_str: &str, tasks: &[Task]) -> Result<Vec<Task>, String> {
    let query = parse_query(query_str)?;
    let mut result: Vec<Task> = tasks.to_vec();

    for filter in &query.filters {
        result.retain(|t| matches_filter(t, filter));
    }

    if let Some(sort_field) = &query.sort_by {
        result.sort_by(|a, b| compare_tasks(a, b, sort_field));
    }

    Ok(result)
}

fn parse_query(query_str: &str) -> Result<Query, String> {
    let mut filters = Vec::new();
    let mut sort_by = None;
    let mut group_by = None;

    let tokens: Vec<&str> = query_str.split_whitespace().collect();
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i] {
            "done" => {
                filters.push(Filter::Done);
                i += 1;
            }
            "not" if i + 1 < tokens.len() && tokens[i + 1] == "done" => {
                filters.push(Filter::NotDone);
                i += 2;
            }
            "includes" if i + 1 < tokens.len() => {
                filters.push(Filter::Includes(tokens[i + 1].to_string()));
                i += 2;
            }
            "description" if i + 1 < tokens.len() && tokens[i + 1] == "includes" && i + 2 < tokens.len() => {
                filters.push(Filter::DescriptionIncludes(tokens[i + 2].to_string()));
                i += 3;
            }
            "tag" if i + 1 < tokens.len() => {
                filters.push(Filter::Tag(tokens[i + 1].to_string()));
                i += 2;
            }
            "folder" if i + 1 < tokens.len() => {
                filters.push(Filter::Folder(tokens[i + 1].to_string()));
                i += 2;
            }
            "due" if i + 2 < tokens.len() => {
                let date = parse_date(tokens[i + 2])?;
                match tokens[i + 1] {
                    "before" => filters.push(Filter::DueBefore(date)),
                    "after" => filters.push(Filter::DueAfter(date)),
                    "on" => filters.push(Filter::DueOn(date)),
                    _ => return Err(format!("Unknown due filter: {}", tokens[i + 1])),
                }
                i += 3;
            }
            "scheduled" if i + 2 < tokens.len() => {
                let date = parse_date(tokens[i + 2])?;
                match tokens[i + 1] {
                    "before" => filters.push(Filter::ScheduledBefore(date)),
                    "after" => filters.push(Filter::ScheduledAfter(date)),
                    "on" => filters.push(Filter::ScheduledOn(date)),
                    _ => return Err(format!("Unknown scheduled filter: {}", tokens[i + 1])),
                }
                i += 3;
            }
            "happens" if i + 2 < tokens.len() => {
                let date = parse_date(tokens[i + 2])?;
                match tokens[i + 1] {
                    "before" => filters.push(Filter::HappensBefore(date)),
                    "after" => filters.push(Filter::HappensAfter(date)),
                    "on" => filters.push(Filter::HappensOn(date)),
                    _ => return Err(format!("Unknown happens filter: {}", tokens[i + 1])),
                }
                i += 3;
            }
            "priority" if i + 2 < tokens.len() && tokens[i + 1] == "is" => {
                let priority = parse_priority(tokens[i + 2])?;
                filters.push(Filter::PriorityIs(priority));
                i += 3;
            }
            "has" if i + 1 < tokens.len() && tokens[i + 1] == "recurrence" => {
                filters.push(Filter::HasRecurrence);
                i += 2;
            }
            "recurrence" if i + 1 < tokens.len() && tokens[i + 1] == "includes" && i + 2 < tokens.len() => {
                filters.push(Filter::RecurrenceIncludes(tokens[i + 2].to_string()));
                i += 3;
            }
            "limit" if i + 1 < tokens.len() => {
                let n = tokens[i + 1].parse::<usize>().map_err(|_| "Invalid limit")?;
                filters.push(Filter::Limit(n));
                i += 2;
            }
            "sort" if i + 2 < tokens.len() && tokens[i + 1] == "by" => {
                sort_by = Some(parse_sort_field(tokens[i + 2])?);
                i += 3;
            }
            "group" if i + 2 < tokens.len() && tokens[i + 1] == "by" => {
                group_by = Some(parse_sort_field(tokens[i + 2])?);
                i += 3;
            }
            _ => return Err(format!("Unknown query token: {}", tokens[i])),
        }
    }

    Ok(Query {
        filters,
        sort_by,
        group_by,
    })
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    let today = chrono::Local::now().date_naive();
    match s {
        "today" => Ok(today),
        "tomorrow" => Ok(today + chrono::Duration::days(1)),
        "yesterday" => Ok(today - chrono::Duration::days(1)),
        _ => {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date: {}", s))
        }
    }
}

fn parse_priority(s: &str) -> Result<Priority, String> {
    match s {
        "high" => Ok(Priority::High),
        "medium" => Ok(Priority::Medium),
        "low" => Ok(Priority::Low),
        "none" => Ok(Priority::None),
        _ => Err(format!("Unknown priority: {}", s)),
    }
}

fn parse_sort_field(s: &str) -> Result<SortField, String> {
    match s {
        "due" => Ok(SortField::Due),
        "scheduled" => Ok(SortField::Scheduled),
        "priority" => Ok(SortField::Priority),
        "description" => Ok(SortField::Description),
        "tag" => Ok(SortField::Tag),
        "folder" => Ok(SortField::Folder),
        "done" => Ok(SortField::Done),
        "created" => Ok(SortField::Created),
        _ => Err(format!("Unknown sort field: {}", s)),
    }
}

fn matches_filter(task: &Task, filter: &Filter) -> bool {
    match filter {
        Filter::Done => task.status == TaskStatus::Done,
        Filter::NotDone => task.status == TaskStatus::Todo,
        Filter::Includes(text) => task.description.to_lowercase().contains(&text.to_lowercase()),
        Filter::DescriptionIncludes(text) => task.description.to_lowercase().contains(&text.to_lowercase()),
        Filter::Tag(tag) => task.tags.contains(&tag.to_string()),
        Filter::Folder(folder) => task.source_file.to_string_lossy().contains(folder.as_str()),
        Filter::DueBefore(date) => task.due_date.is_some_and(|d| d < *date),
        Filter::DueAfter(date) => task.due_date.is_some_and(|d| d > *date),
        Filter::DueOn(date) => task.due_date == Some(*date),
        Filter::ScheduledBefore(date) => task.scheduled_date.is_some_and(|d| d < *date),
        Filter::ScheduledAfter(date) => task.scheduled_date.is_some_and(|d| d > *date),
        Filter::ScheduledOn(date) => task.scheduled_date == Some(*date),
        Filter::HappensBefore(date) => {
            task.due_date.is_some_and(|d| d < *date) ||
            task.scheduled_date.is_some_and(|d| d < *date)
        }
        Filter::HappensAfter(date) => {
            task.due_date.is_some_and(|d| d > *date) ||
            task.scheduled_date.is_some_and(|d| d > *date)
        }
        Filter::HappensOn(date) => {
            task.due_date == Some(*date) || task.scheduled_date == Some(*date)
        }
        Filter::PriorityAbove(p) => task.priority > *p,
        Filter::PriorityBelow(p) => task.priority < *p,
        Filter::PriorityIs(p) => task.priority == *p,
        Filter::HasRecurrence => task.recurrence.is_some(),
        Filter::RecurrenceIncludes(text) => {
            task.recurrence.as_ref().is_some_and(|r| r.to_lowercase().contains(&text.to_lowercase()))
        }
        Filter::Limit(_) => true,
    }
}

fn compare_tasks(a: &Task, b: &Task, field: &SortField) -> std::cmp::Ordering {
    match field {
        SortField::Due => a.due_date.cmp(&b.due_date),
        SortField::Scheduled => a.scheduled_date.cmp(&b.scheduled_date),
        SortField::Priority => b.priority.cmp(&a.priority),
        SortField::Description => a.description.cmp(&b.description),
        SortField::Tag => a.tags.first().cmp(&b.tags.first()),
        SortField::Folder => a.source_file.cmp(&b.source_file),
        SortField::Done => {
            let a_val = if a.status == TaskStatus::Done { 1 } else { 0 };
            let b_val = if b.status == TaskStatus::Done { 1 } else { 0 };
            a_val.cmp(&b_val)
        }
        SortField::Created => a.line_number.cmp(&b.line_number),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Task, TaskStatus, Priority};
    use chrono::NaiveDate;
    use std::path::PathBuf;

    fn sample_tasks() -> Vec<Task> {
        vec![
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
        ]
    }

    #[test]
    fn test_filter_done() {
        let tasks = sample_tasks();
        let result = execute_query("done", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Review PR");
    }

    #[test]
    fn test_filter_not_done() {
        let tasks = sample_tasks();
        let result = execute_query("not done", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_includes() {
        let tasks = sample_tasks();
        let result = execute_query("includes bug", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_filter_tag() {
        let tasks = sample_tasks();
        let result = execute_query("tag personal", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Buy groceries");
    }

    #[test]
    fn test_filter_priority_high() {
        let tasks = sample_tasks();
        let result = execute_query("priority is high", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Review PR");
    }

    #[test]
    fn test_filter_has_recurrence() {
        let tasks = sample_tasks();
        let result = execute_query("has recurrence", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_sort_by_priority() {
        let tasks = sample_tasks();
        let result = execute_query("sort by priority", &tasks).unwrap();
        assert_eq!(result[0].priority, Priority::High);
        assert_eq!(result[1].priority, Priority::Medium);
        assert_eq!(result[2].priority, Priority::None);
    }

    #[test]
    fn test_combined_query() {
        let tasks = sample_tasks();
        let result = execute_query("not done tag work sort by priority", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_empty_query() {
        let tasks = sample_tasks();
        let result = execute_query("", &tasks).unwrap();
        assert_eq!(result.len(), 3);
    }
}
