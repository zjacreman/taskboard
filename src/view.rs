#[derive(Debug, Clone)]
pub struct View {
    pub name: String,
    pub query: String,
    pub sort_by: String,
    pub group_by: String,
}

impl View {
    pub fn new(name: &str, query: &str, sort_by: &str, group_by: &str) -> Self {
        Self {
            name: name.to_string(),
            query: query.to_string(),
            sort_by: sort_by.to_string(),
            group_by: group_by.to_string(),
        }
    }
}

impl Default for View {
    fn default() -> Self {
        Self::new("All Tasks", "", "due_date", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_creation() {
        let view = View::new("My View", "not done tag work", "priority", "due");
        assert_eq!(view.name, "My View");
        assert_eq!(view.query, "not done tag work");
        assert_eq!(view.sort_by, "priority");
        assert_eq!(view.group_by, "due");
    }

    #[test]
    fn test_default_view() {
        let view = View::default();
        assert_eq!(view.name, "All Tasks");
        assert_eq!(view.query, "");
        assert_eq!(view.sort_by, "due_date");
        assert_eq!(view.group_by, "");
    }
}
