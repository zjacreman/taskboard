use crate::task::Task;

/// Returns true if the task matches the given filter text: case-insensitive
/// substring match on the description and on tags. A leading '#' in the
/// filter is ignored for tag matching. Empty/whitespace filter matches all.
// TODO: remove allow(dead_code) once the filter is wired into the pipeline.
#[allow(dead_code)]
pub fn matches_filter(task: &Task, filter: &str) -> bool {
    let needle = filter.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    if task.description.to_lowercase().contains(&needle) {
        return true;
    }
    let tag_needle = needle.trim_start_matches('#');
    task.tags
        .iter()
        .any(|t| t.to_lowercase().contains(tag_needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::sample_tasks;

    #[test]
    fn test_matches_description_case_insensitive() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[0], "BUG"));
        assert!(!matches_filter(&tasks[1], "bug"));
    }

    #[test]
    fn test_matches_tag() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[1], "personal"));
        assert!(matches_filter(&tasks[0], "urgent"));
        assert!(!matches_filter(&tasks[1], "urgent"));
    }

    #[test]
    fn test_matches_tag_with_hash_prefix() {
        let tasks = sample_tasks();
        assert!(matches_filter(&tasks[0], "#work"));
        assert!(matches_filter(&tasks[2], "#WORK"));
    }

    #[test]
    fn test_empty_and_whitespace_matches_all() {
        let tasks = sample_tasks();
        for task in &tasks {
            assert!(matches_filter(task, ""));
            assert!(matches_filter(task, "   "));
        }
    }
}
