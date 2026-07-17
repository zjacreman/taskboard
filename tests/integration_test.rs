use std::path::PathBuf;

#[test]
fn test_full_pipeline() {
    let basic_content = std::fs::read_to_string("tests/fixtures/basic.md").unwrap();
    let basic_tasks = taskboard::task::parser::parse_file(
        &basic_content,
        &PathBuf::from("tests/fixtures/basic.md"),
    );

    let full_content = std::fs::read_to_string("tests/fixtures/full_metadata.md").unwrap();
    let full_tasks = taskboard::task::parser::parse_file(
        &full_content,
        &PathBuf::from("tests/fixtures/full_metadata.md"),
    );

    let mut all_tasks = basic_tasks;
    all_tasks.extend(full_tasks);

    let done_tasks = taskboard::task::query::execute_query("done", &all_tasks).unwrap();
    assert!(done_tasks
        .iter()
        .all(|t| t.status == taskboard::task::TaskStatus::Done));

    let not_done = taskboard::task::query::execute_query("not done", &all_tasks).unwrap();
    assert!(not_done
        .iter()
        .all(|t| t.status == taskboard::task::TaskStatus::Todo));

    let high_priority =
        taskboard::task::query::execute_query("priority is high", &all_tasks).unwrap();
    assert!(high_priority
        .iter()
        .all(|t| t.priority == taskboard::task::Priority::High));

    let sorted = taskboard::task::query::execute_query("sort by priority", &all_tasks).unwrap();
    for i in 0..sorted.len() - 1 {
        assert!(sorted[i].priority >= sorted[i + 1].priority);
    }
}

#[test]
fn test_large_file_performance() {
    let start = std::time::Instant::now();
    let content = std::fs::read_to_string("tests/fixtures/large.md").unwrap();
    let tasks =
        taskboard::task::parser::parse_file(&content, &PathBuf::from("tests/fixtures/large.md"));
    let duration = start.elapsed();

    assert_eq!(tasks.len(), 1000);
    assert!(
        duration.as_millis() < 500,
        "Parsing took {}ms, expected < 500ms",
        duration.as_millis()
    );
}
