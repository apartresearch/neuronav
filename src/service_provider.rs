use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceProvider {
    Neuroscope,
    Json { path: String },
}
