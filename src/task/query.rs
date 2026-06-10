use super::{Task, TaskStatus, Priority};
use chrono::NaiveDate;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Filter {
    Done,
    NotDone,
    Includes(String),
    NotIncludes(String),
    DescriptionIncludes(String),
    Tag(String),
    NotTag(String),
    Folder(String),
    NotFolder(String),
    PathIncludes(String),
    PathNotIncludes(String),
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
    NoRecurrence,
    RecurrenceIncludes(String),
    HasDueDate,
    NoDueDate,
    HasScheduledDate,
    NoScheduledDate,
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
#[allow(dead_code)]
pub struct Query {
    pub filters: Vec<Filter>,
    pub sort_by: Option<SortField>,
    pub group_by: Option<SortField>,
}

pub fn execute_query(query_str: &str, tasks: &[Task]) -> Result<Vec<Task>, String> {
    let query = parse_query(query_str)?;

    if query.group_by.is_some() {
        log::debug!("group_by is not yet implemented");
    }

    let mut result: Vec<Task> = tasks.to_vec();

    for filter in &query.filters {
        result.retain(|t| matches_filter(t, filter));
    }

    if let Some(sort_field) = &query.sort_by {
        result.sort_by(|a, b| compare_tasks(a, b, sort_field));
    }

    for filter in &query.filters {
        if let Filter::Limit(n) = filter {
            result.truncate(*n);
            break;
        }
    }

    Ok(result)
}

fn parse_query(query_str: &str) -> Result<Query, String> {
    let mut filters = Vec::new();
    let mut sort_by = None;
    let mut group_by = None;

    for line in query_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let tokens = tokenize_line(line);
        let mut i = 0;

        while i < tokens.len() {
            match tokens[i].as_str() {
                "done" => {
                    filters.push(Filter::Done);
                    i += 1;
                }
                "not" if i + 1 < tokens.len() && tokens[i + 1] == "done" => {
                    filters.push(Filter::NotDone);
                    i += 2;
                }
                "not" if i + 2 < tokens.len() && tokens[i + 1] == "includes" => {
                    filters.push(Filter::NotIncludes(tokens[i + 2].clone()));
                    i += 3;
                }
                "not" if i + 2 < tokens.len() && tokens[i + 1] == "tag" => {
                    filters.push(Filter::NotTag(tokens[i + 2].clone()));
                    i += 3;
                }
                "not" if i + 2 < tokens.len() && tokens[i + 1] == "folder" => {
                    filters.push(Filter::NotFolder(tokens[i + 2].clone()));
                    i += 3;
                }
                "does" if i + 3 < tokens.len() && tokens[i + 1] == "not" && tokens[i + 2] == "include" => {
                    filters.push(Filter::NotIncludes(tokens[i + 3].clone()));
                    i += 4;
                }
                "path" if i + 2 < tokens.len() && tokens[i + 1] == "includes" => {
                    filters.push(Filter::PathIncludes(tokens[i + 2].clone()));
                    i += 3;
                }
                "path" if i + 4 < tokens.len() && tokens[i + 1] == "does" && tokens[i + 2] == "not" && tokens[i + 3] == "include" => {
                    filters.push(Filter::PathNotIncludes(tokens[i + 4].clone()));
                    i += 5;
                }
                "path" if i + 3 < tokens.len() && tokens[i + 1] == "does" && tokens[i + 2] == "include" => {
                    filters.push(Filter::PathIncludes(tokens[i + 3].clone()));
                    i += 4;
                }
                "includes" if i + 1 < tokens.len() => {
                    filters.push(Filter::Includes(tokens[i + 1].clone()));
                    i += 2;
                }
                "description" if i + 2 < tokens.len() && tokens[i + 1] == "includes" => {
                    filters.push(Filter::DescriptionIncludes(tokens[i + 2].clone()));
                    i += 3;
                }
                "tag" if i + 1 < tokens.len() => {
                    filters.push(Filter::Tag(tokens[i + 1].clone()));
                    i += 2;
                }
                "folder" if i + 1 < tokens.len() => {
                    filters.push(Filter::Folder(tokens[i + 1].clone()));
                    i += 2;
                }
                "due" if i + 1 < tokens.len() => {
                    if is_relative_date(&tokens[i + 1]) {
                        let date = parse_date(&tokens[i + 1])?;
                        filters.push(Filter::DueOn(date));
                        i += 2;
                    } else if i + 2 < tokens.len() {
                        let date = parse_date(&tokens[i + 2])?;
                        match tokens[i + 1].as_str() {
                            "before" => filters.push(Filter::DueBefore(date)),
                            "after" => filters.push(Filter::DueAfter(date)),
                            "on" => filters.push(Filter::DueOn(date)),
                            _ => return Err(format!("Unknown due filter: {}", tokens[i + 1])),
                        }
                        i += 3;
                    } else {
                        return Err("Incomplete due filter".to_string());
                    }
                }
                "scheduled" if i + 1 < tokens.len() => {
                    if is_relative_date(&tokens[i + 1]) {
                        let date = parse_date(&tokens[i + 1])?;
                        filters.push(Filter::ScheduledOn(date));
                        i += 2;
                    } else if i + 2 < tokens.len() {
                        let date = parse_date(&tokens[i + 2])?;
                        match tokens[i + 1].as_str() {
                            "before" => filters.push(Filter::ScheduledBefore(date)),
                            "after" => filters.push(Filter::ScheduledAfter(date)),
                            "on" => filters.push(Filter::ScheduledOn(date)),
                            _ => return Err(format!("Unknown scheduled filter: {}", tokens[i + 1])),
                        }
                        i += 3;
                    } else {
                        return Err("Incomplete scheduled filter".to_string());
                    }
                }
                "happens" if i + 2 < tokens.len() => {
                    let date = parse_date(&tokens[i + 2])?;
                    match tokens[i + 1].as_str() {
                        "before" => filters.push(Filter::HappensBefore(date)),
                        "after" => filters.push(Filter::HappensAfter(date)),
                        "on" => filters.push(Filter::HappensOn(date)),
                        _ => return Err(format!("Unknown happens filter: {}", tokens[i + 1])),
                    }
                    i += 3;
                }
                "priority" if i + 2 < tokens.len() && tokens[i + 1] == "is" => {
                    let priority = parse_priority(&tokens[i + 2])?;
                    filters.push(Filter::PriorityIs(priority));
                    i += 3;
                }
                "priority" if i + 2 < tokens.len() && tokens[i + 1] == "above" => {
                    let priority = parse_priority(&tokens[i + 2])?;
                    filters.push(Filter::PriorityAbove(priority));
                    i += 3;
                }
                "priority" if i + 2 < tokens.len() && tokens[i + 1] == "below" => {
                    let priority = parse_priority(&tokens[i + 2])?;
                    filters.push(Filter::PriorityBelow(priority));
                    i += 3;
                }
                "has" if i + 1 < tokens.len() && tokens[i + 1] == "recurrence" => {
                    filters.push(Filter::HasRecurrence);
                    i += 2;
                }
                "has" if i + 2 < tokens.len() && tokens[i + 1] == "due" && tokens[i + 2] == "date" => {
                    filters.push(Filter::HasDueDate);
                    i += 3;
                }
                "has" if i + 2 < tokens.len() && tokens[i + 1] == "scheduled" && tokens[i + 2] == "date" => {
                    filters.push(Filter::HasScheduledDate);
                    i += 3;
                }
                "no" if i + 1 < tokens.len() && tokens[i + 1] == "recurrence" => {
                    filters.push(Filter::NoRecurrence);
                    i += 2;
                }
                "no" if i + 2 < tokens.len() && tokens[i + 1] == "due" && tokens[i + 2] == "date" => {
                    filters.push(Filter::NoDueDate);
                    i += 3;
                }
                "no" if i + 2 < tokens.len() && tokens[i + 1] == "scheduled" && tokens[i + 2] == "date" => {
                    filters.push(Filter::NoScheduledDate);
                    i += 3;
                }
                "recurrence" if i + 2 < tokens.len() && tokens[i + 1] == "includes" => {
                    filters.push(Filter::RecurrenceIncludes(tokens[i + 2].clone()));
                    i += 3;
                }
                "limit" if i + 1 < tokens.len() => {
                    let n = tokens[i + 1].parse::<usize>().map_err(|_| "Invalid limit")?;
                    filters.push(Filter::Limit(n));
                    i += 2;
                }
                "sort" if i + 2 < tokens.len() && tokens[i + 1] == "by" => {
                    sort_by = Some(parse_sort_field(&tokens[i + 2])?);
                    i += 3;
                }
                "group" if i + 2 < tokens.len() && tokens[i + 1] == "by" => {
                    group_by = Some(parse_sort_field(&tokens[i + 2])?);
                    i += 3;
                }
                _ => return Err(format!("Unknown query token: {}", tokens[i])),
            }
        }
    }

    Ok(Query {
        filters,
        sort_by,
        group_by,
    })
}

fn tokenize_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars = line.chars();

    for ch in chars {
        match ch {
            '"' | '\'' => {
                if in_quotes {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    in_quotes = false;
                } else {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    in_quotes = true;
                }
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn is_relative_date(s: &str) -> bool {
    matches!(s, "today" | "tomorrow" | "yesterday")
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
        Filter::Includes(text) => {
            let lower = text.to_lowercase();
            task.description.to_lowercase().contains(&lower)
                || task.tags.iter().any(|t| t.to_lowercase().contains(&lower))
                || task.recurrence.as_ref().is_some_and(|r| r.to_lowercase().contains(&lower))
        }
        Filter::NotIncludes(text) => {
            let lower = text.to_lowercase();
            !task.description.to_lowercase().contains(&lower)
                && !task.tags.iter().any(|t| t.to_lowercase().contains(&lower))
                && !task.recurrence.as_ref().is_some_and(|r| r.to_lowercase().contains(&lower))
        }
        Filter::DescriptionIncludes(text) => task.description.to_lowercase().contains(&text.to_lowercase()),
        Filter::Tag(tag) => task.tags.contains(&tag.to_string()),
        Filter::NotTag(tag) => !task.tags.contains(&tag.to_string()),
        Filter::Folder(folder) => {
            task.source_file.components().any(|c| {
                c.as_os_str().to_string_lossy() == folder.as_str()
            })
        }
        Filter::NotFolder(folder) => {
            !task.source_file.components().any(|c| {
                c.as_os_str().to_string_lossy() == folder.as_str()
            })
        }
        Filter::PathIncludes(text) => {
            task.source_file.to_string_lossy().to_lowercase().contains(&text.to_lowercase())
        }
        Filter::PathNotIncludes(text) => {
            !task.source_file.to_string_lossy().to_lowercase().contains(&text.to_lowercase())
        }
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
        Filter::NoRecurrence => task.recurrence.is_none(),
        Filter::RecurrenceIncludes(text) => {
            task.recurrence.as_ref().is_some_and(|r| r.to_lowercase().contains(&text.to_lowercase()))
        }
        Filter::HasDueDate => task.due_date.is_some(),
        Filter::NoDueDate => task.due_date.is_none(),
        Filter::HasScheduledDate => task.scheduled_date.is_some(),
        Filter::NoScheduledDate => task.scheduled_date.is_none(),
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
    use crate::task::Priority;
    use crate::test_helpers::sample_tasks;

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

    #[test]
    fn test_filter_priority_above() {
        let tasks = sample_tasks();
        let result = execute_query("priority above medium", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Review PR");
    }

    #[test]
    fn test_filter_priority_below() {
        let tasks = sample_tasks();
        let result = execute_query("priority below medium", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Buy groceries");
    }

    #[test]
    fn test_limit() {
        let tasks = sample_tasks();
        let result = execute_query("limit 2", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_limit_with_sort() {
        let tasks = sample_tasks();
        let result = execute_query("sort by priority limit 1", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].priority, Priority::High);
    }

    #[test]
    fn test_includes_searches_tags() {
        let tasks = sample_tasks();
        let result = execute_query("includes urgent", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_includes_searches_recurrence() {
        let tasks = sample_tasks();
        let result = execute_query("includes week", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_description_includes_only_description() {
        let tasks = sample_tasks();
        let result = execute_query("description includes urgent", &tasks).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_folder_matches_component() {
        let mut tasks = sample_tasks();
        tasks[2].source_file = std::path::PathBuf::from("bugs/critical.md");
        let result = execute_query("folder bugs", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_scheduled_today_shorthand() {
        let tasks = sample_tasks();
        let result = execute_query("scheduled today", &tasks).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_scheduled_before_today() {
        let tasks = sample_tasks();
        let result = execute_query("scheduled before today", &tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn test_not_includes() {
        let tasks = sample_tasks();
        let result = execute_query("not includes bug", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_does_not_include() {
        let tasks = sample_tasks();
        let result = execute_query("does not include bug", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_path_includes() {
        let tasks = sample_tasks();
        let result = execute_query("path includes bugs", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_path_not_includes() {
        let tasks = sample_tasks();
        let result = execute_query("path does not include bugs", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_path_not_includes_reading_list() {
        let mut tasks = sample_tasks();
        tasks[0].source_file = std::path::PathBuf::from("boards/Reading List.md");
        let result = execute_query("path does not include \"Reading List\"", &tasks).unwrap();
        assert_eq!(result.len(), 2);
        assert!(!result.iter().any(|t| t.source_file.to_string_lossy().contains("Reading List")));
    }

    #[test]
    fn test_no_recurrence() {
        let tasks = sample_tasks();
        let result = execute_query("no recurrence", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_no_due_date() {
        let tasks = sample_tasks();
        let result = execute_query("no due date", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_has_due_date() {
        let tasks = sample_tasks();
        let result = execute_query("has due date", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_not_tag() {
        let tasks = sample_tasks();
        let result = execute_query("not tag urgent", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_multi_line_query() {
        let tasks = sample_tasks();
        let result = execute_query("not done\ntag work", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }
}
