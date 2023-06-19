use std::path::Path;

use crate::data::NeuroscopePage;

use super::ServiceProvider;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Service {
    name: String,
    provider: ServiceProvider,
}

impl Service {
    pub fn new(name: impl Into<String>, provider: ServiceProvider) -> Self {
        Self {
            name: name.into(),
            provider,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn neuron_page(
        &self,
        model_name: impl AsRef<str>,
        model_path: impl AsRef<Path>,
        layer_index: u32,
        neuron_index: u32,
    ) -> Result<String> {
        let model_name = model_name.as_ref();
        let model_path = model_path.as_ref();
        match &self.provider {
            ServiceProvider::Neuroscope => {
                let neuroscope_path = model_path.join("neuroscope");
                if !neuroscope_path.is_dir() {
                    bail!(
                        "Neuroscope directory not found for model '{model_name}'. Should be at '{neuroscope_path:?}'.",
                    );
                }
                let path = neuroscope_path.join(format!("l{layer_index}n{neuron_index}.postcard",));
                let page = NeuroscopePage::from_file(path)?;
                Ok(serde_json::to_string(&page)
                    .expect("Failed to serialize page to JSON. This should always be possible."))
            }
            ServiceProvider::Json { path } => {
                let service_path = model_path.join(path);
                if !service_path.is_dir() {
                    bail!(
                        "Service directory not found for service '{}' and model '{model_name}'. Should be at '{service_path:?}'.", &self.name
                    );
                }
                let path = model_path
                    .join(path)
                    .join(format!("l{layer_index}n{neuron_index}.json",));
                let json = std::fs::read_to_string(path).with_context(|| format!("Failed to read JSON file for service '{}' and model '{model_name}' at layer '{layer_index}' neuron '{neuron_index}'", &self.name))?;
                Ok(json)
            }
        }
    }
}
