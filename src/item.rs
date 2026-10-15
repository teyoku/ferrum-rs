#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    name: String,
    description: String,
}

impl Item {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.name.to_lowercase() == name.trim().to_lowercase()
    }
}
