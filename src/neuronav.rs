use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::Service;

#[derive(Debug, Clone)]
pub struct Neuronav {
    path: PathBuf,
    services: HashMap<String, Service>,
}

impl Neuronav {
    fn config_path(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().join("config.json")
    }

    pub fn initialize<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create directory for '{path:?}'"))?;
        if path
            .read_dir()
            .with_context(|| format!("Failed to read directory '{path:?}'"))?
            .count()
            > 0
        {
            bail!("Directory '{path:?}' is not empty.");
        }
        let result = Self {
            path,
            services: HashMap::new(),
        };
        result
            .save()
            .context("Failed to save the newly initialized neuronav.")?;
        Ok(result)
    }

    pub fn from_dir<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.is_dir() {
            bail!("Path '{path:?}' is not a directory.");
        }
        let config_path = Self::config_path(path.as_path());
        if !config_path.is_file() {
            bail!("Config for neuronav not found. Should be at '{config_path:?}'.");
        }
        let config_string = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config at '{config_path:?}'."))?;
        let services: Vec<Service> = serde_json::from_str(&config_string)
            .with_context(|| format!("Failed to parse config at '{config_path:?}'."))?;
        let services = services
            .into_iter()
            .map(|service| (service.name().to_owned(), service))
            .collect();
        Ok(Self { path, services })
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path(self.path.as_path());
        let services: Vec<Service> = self.services.values().cloned().collect();
        let config_string =
            serde_json::to_string(&services).context("Failed to serialize config to JSON.")?;
        fs::write(&config_path, config_string)
            .with_context(|| format!("Failed to write config to '{config_path:?}'."))?;
        Ok(())
    }

    pub fn model_dir<S: AsRef<str>>(&self, model_name: S) -> PathBuf {
        self.path.join(model_name.as_ref())
    }

    pub fn handle_request(
        &self,
        model_name: impl AsRef<str>,
        service_name: impl AsRef<str>,
        layer_index: u32,
        neuron_index: u32,
    ) -> Result<String> {
        let model_name = model_name.as_ref();
        let service_name = service_name.as_ref();
        let model_dir = self.model_dir(model_name);
        if !model_dir.is_dir() {
            bail!(
                "Model directory not found for model '{model_name}'. Should be at '{model_dir:?}'.",
            );
        }
        let service = self
            .services
            .get(service_name)
            .with_context(|| format!("Service '{}' not found.", service_name))?;
        service.neuron_page(model_name, model_dir, layer_index, neuron_index)
    }
}
