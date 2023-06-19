use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelConfig {
    name: String,
    available_services: HashSet<String>,
}

impl ModelConfig {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn has_serice(&self, service_name: impl AsRef<str>) -> bool {
        self.available_services.contains(service_name.as_ref())
    }

    pub fn available_services(&self) -> &HashSet<String> {
        &self.available_services
    }

    pub fn add_service(&mut self, service_name: impl Into<String>) {
        self.available_services.insert(service_name.into());
    }
}
