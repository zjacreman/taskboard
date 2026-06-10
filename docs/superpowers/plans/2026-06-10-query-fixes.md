# Query Engine Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix five bugs/missing features in the task query engine (`src/task/query.rs`).

**Architecture:** All changes are in `src/task/query.rs`. Each issue is independent and can be implemented in any order. Tests live in the same file's `#[cfg(test)]` module.

**Tech Stack:** Rust, chrono, log crate (already in Cargo.toml)

---

## File Map

- Modify: `src/task/query.rs` — all five fixes
- Test: `src/task/query.rs` (inline `#[cfg(test)]` module)

---

### Task 1: Fix Limit filter (Issue 1)

**Problem:** `Filter::Limit(n)` is parsed and `matches_filter` always returns `true`, but `execute_query` never truncates results.

**Files:**
- Modify: `src/task/query.rs:50-63` (`execute_query` function)

- [ ] **Step 1: Add limit truncation after sorting in `execute_query`**

In `execute_query`, after the sort block (line 60) and before `Ok(result)` (line 62), add:

```rust
    for filter in &query.filters {
        if let Filter::Limit(n) = filter {
            result.truncate(*n);
        }
    }
```

- [ ] **Step 2: Add test for limit filter**

Add to the `tests` module:

```rust
    #[test]
    fn test_filter_limit() {
        let tasks = sample_tasks();
        let result = execute_query("limit 2", &tasks).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_limit_with_sort() {
        let tasks = sample_tasks();
        let result = execute_query("sort by priority limit 1", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].priority, Priority::High);
    }
```

- [ ] **Step 3: Run tests**

Run: `nix-shell --run "cargo test"`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/task/query.rs
git commit -m "fix: apply Limit filter truncation after filtering and sorting"
```

---

### Task 2: Log debug for group_by (Issue 2)

**Problem:** `group_by` is parsed but silently ignored.

**Files:**
- Modify: `src/task/query.rs:50-63` (`execute_query` function)

- [ ] **Step 1: Add debug log when group_by is set**

In `execute_query`, after the sort block and before the limit block (from Task 1), add:

```rust
    if query.group_by.is_some() {
        log::debug!("group_by is not yet implemented");
    }
```

- [ ] **Step 2: Run tests**

Run: `nix-shell --run "cargo test"`
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add src/task/query.rs
git commit -m "fix: log debug message when group_by is set (not yet implemented)"
```

---

### Task 3: Add parser support for PriorityAbove/PriorityBelow (Issue 3)

**Problem:** `Filter::PriorityAbove` and `Filter::PriorityBelow` variants exist and are handled in `matches_filter`, but no parser branch produces them.

**Files:**
- Modify: `src/task/query.rs:129-133` (priority parsing in `parse_query`)

- [ ] **Step 1: Add parser branches for priority above/below**

In `parse_query`, the current `"priority"` arm at line 129 only handles `"priority is <p>"`. Replace it with:

```rust
            "priority" if i + 2 < tokens.len() => {
                match tokens[i + 1] {
                    "is" => {
                        let priority = parse_priority(tokens[i + 2])?;
                        filters.push(Filter::PriorityIs(priority));
                        i += 3;
                    }
                    "above" => {
                        let priority = parse_priority(tokens[i + 2])?;
                        filters.push(Filter::PriorityAbove(priority));
                        i += 3;
                    }
                    "below" => {
                        let priority = parse_priority(tokens[i + 2])?;
                        filters.push(Filter::PriorityBelow(priority));
                        i += 3;
                    }
                    _ => return Err(format!("Unknown priority filter: {}", tokens[i + 1])),
                }
            }
```

- [ ] **Step 2: Add tests for PriorityAbove and PriorityBelow**

Add to the `tests` module:

```rust
    #[test]
    fn test_filter_priority_above() {
        let tasks = sample_tasks();
        // Medium > Low, High > Low; None and Medium are not > Medium
        // sample_tasks: None, High, Medium
        // PriorityAbove(Medium) should return only High
        let result = execute_query("priority above medium", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Review PR");
    }

    #[test]
    fn test_filter_priority_below() {
        let tasks = sample_tasks();
        // PriorityBelow(Medium) should return only None (Lowest/Low not in sample)
        // sample_tasks: None, High, Medium
        // None < Medium is true
        let result = execute_query("priority below medium", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Buy groceries");
    }
```

- [ ] **Step 3: Run tests**

Run: `nix-shell --run "cargo test"`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/task/query.rs
git commit -m "feat: add parser support for priority above/below filters"
```

---

### Task 4: Fix Folder filter substring match (Issue 4)

**Problem:** `Folder` filter uses `.contains()` which matches substrings. `folder projects` matches `my_projects/todo.md`.

**Files:**
- Modify: `src/task/query.rs:210` (`matches_filter` Folder arm)

- [ ] **Step 1: Replace substring match with component-boundary match**

Replace line 210:
```rust
        Filter::Folder(folder) => task.source_file.to_string_lossy().contains(folder.as_str()),
```

With:
```rust
        Filter::Folder(folder) => task.source_file.components().any(|c| c.as_os_str() == folder.as_str()),
```

- [ ] **Step 2: Add tests for folder boundary matching**

Add to the `tests` module. First add a helper task with a nested path, then test:

```rust
    #[test]
    fn test_filter_folder_component_match() {
        let mut tasks = sample_tasks();
        tasks.push(Task {
            description: "Nested task".to_string(),
            status: TaskStatus::Todo,
            priority: Priority::None,
            due_date: None,
            scheduled_date: None,
            recurrence: None,
            done_date: None,
            start_date: None,
            tags: vec![],
            source_file: PathBuf::from("projects/todo.md"),
            line_number: 20,
        });
        tasks.push(Task {
            description: "My projects task".to_string(),
            status: TaskStatus::Todo,
            priority: Priority::None,
            due_date: None,
            scheduled_date: None,
            recurrence: None,
            done_date: None,
            start_date: None,
            tags: vec![],
            source_file: PathBuf::from("my_projects/todo.md"),
            line_number: 21,
        });

        let result = execute_query("folder projects", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Nested task");
    }
```

- [ ] **Step 3: Run tests**

Run: `nix-shell --run "cargo test"`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/task/query.rs
git commit -m "fix: match folder filter on directory component boundaries"
```

---

### Task 5: Differentiate Includes from DescriptionIncludes (Issue 5)

**Problem:** Both `Includes` and `DescriptionIncludes` search only `task.description`. `Includes` should also search tags and recurrence.

**Files:**
- Modify: `src/task/query.rs:207` (`matches_filter` Includes arm)

- [ ] **Step 1: Broaden Includes to search description, tags, and recurrence**

Replace line 207:
```rust
        Filter::Includes(text) => task.description.to_lowercase().contains(&text.to_lowercase()),
```

With:
```rust
        Filter::Includes(text) => {
            let text_lower = text.to_lowercase();
            task.description.to_lowercase().contains(&text_lower)
                || task.tags.iter().any(|t| t.to_lowercase().contains(&text_lower))
                || task.recurrence.as_ref().is_some_and(|r| r.to_lowercase().contains(&text_lower))
        }
```

- [ ] **Step 2: Add tests for Includes searching tags and recurrence**

Add to the `tests` module:

```rust
    #[test]
    fn test_filter_includes_matches_tags() {
        let tasks = sample_tasks();
        // "urgent" is a tag on "Fix bug", not in any description
        let result = execute_query("includes urgent", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_filter_includes_matches_recurrence() {
        let tasks = sample_tasks();
        // "week" appears in recurrence "every week" for "Fix bug"
        let result = execute_query("includes week", &tasks).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, "Fix bug");
    }

    #[test]
    fn test_filter_description_includes_only_description() {
        let tasks = sample_tasks();
        // "urgent" is a tag but NOT in any description
        let result = execute_query("description includes urgent", &tasks).unwrap();
        assert_eq!(result.len(), 0);
    }
```

- [ ] **Step 3: Run tests**

Run: `nix-shell --run "cargo test"`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/task/query.rs
git commit -m "fix: make Includes search description, tags, and recurrence"
```

---

### Task 6: Final verification

- [ ] **Step 1: Run full test suite**

Run: `nix-shell --run "cargo test"`
Expected: All tests pass

- [ ] **Step 2: Run clippy**

Run: `nix-shell --run "cargo clippy -- -D warnings"`
Expected: No warnings or errors

- [ ] **Step 3: Fix any clippy issues**

If clippy reports issues, fix them and re-run.

- [ ] **Step 4: Final commit if needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings"
```
