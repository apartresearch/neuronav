use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::{ModelConfig, Service, ServiceProvider};

#[derive(Debug, Clone)]
pub struct Neuronav {
    path: PathBuf,
    models: HashMap<String, ModelConfig>,
    services: HashMap<String, Service>,
}

impl Neuronav {
    fn config_path(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().join("config.json")
    }

    fn load_model_configs(path: impl AsRef<Path>) -> Result<Vec<ModelConfig>> {
        let path = path.as_ref();
        path.read_dir()
            .with_context(|| "Failed to read directory.")?
            .filter_map(|entry| {
                match entry.with_context(|| format!("Failed to read entry in directory '{path:?}'"))
                {
                    Err(error) => Some(Err(error)),
                    Ok(entry) => {
                        let model_path = entry.path();
                        if !model_path.is_dir() {
                            return None;
                        }
                        let model_config_path = model_path.join("model.json");
                        let model_config_path = model_config_path.as_path();
                        if !model_config_path.is_file() {
                            return Some(Err(anyhow::anyhow!(
                                "Model config not found. Should be at '{model_config_path:?}'.",
                            )));
                        }
                        let model_config: ModelConfig =
                            serde_json::from_reader(File::open(model_config_path).unwrap())
                                .with_context(|| {
                                    format!(
                                "Failed to deserialize model config at '{model_config_path:?}'.",
                            )
                                })
                                .unwrap();
                        Some(Ok(model_config))
                    }
                }
            })
            .collect()
    }

    fn save_model_configs(&self) -> Result<()> {
        let path = self.path.as_path();
        for entry in path
            .read_dir()
            .with_context(|| format!("Failed to read directory '{path:?}'."))?
        {
            let entry =
                entry.with_context(|| format!("Failed to read entry in directory '{path:?}'."))?;
            let model_path = entry.path();
            if !model_path.is_dir() {
                continue;
            }
            let model_config_path = model_path.join("model.json");
            let model_config_path = model_config_path.as_path();
            if !model_config_path.is_file() {
                bail!("Model config not found. Should be at '{model_config_path:?}'.")
            }
            let old_model_config: ModelConfig =
                serde_json::from_reader(File::open(model_config_path).with_context(|| {
                    format!("Failed to open model config at '{model_config_path:?}'.")
                })?)
                .with_context(|| {
                    format!("Failed to parse model config at '{model_config_path:?}'.")
                })?;
            let model_name = old_model_config.name();
            let new_model_config = self
                .models
                .get(model_name)
                .with_context(|| format!("Model '{model_name}' not found in neuronav."))?;
            serde_json::to_writer(
                File::open(model_config_path).with_context(|| {
                    format!("Failed to open model config at '{model_config_path:?}'.")
                })?,
                new_model_config,
            )
            .with_context(|| {
                format!("Failed to write model config to file at '{model_config_path:?}'.")
            })?
        }
        Ok(())
    }

    pub fn initialize<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        log::info!("Initializing neuronav at '{path:?}'...");
        fs::create_dir_all(path)
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
            path: path.to_path_buf(),
            services: HashMap::new(),
            models: HashMap::new(),
        };
        result
            .save()
            .context("Failed to save the newly initialized neuronav.")?;
        log::info!("Successfully initialized neuronav at '{path:?}'.");
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

        let models = Self::load_model_configs(path.as_path())?
            .into_iter()
            .map(|model| (model.name().to_owned(), model))
            .collect();
        Ok(Self {
            path,
            services,
            models,
        })
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path(self.path.as_path());
        let services: Vec<Service> = self.services.values().cloned().collect();
        let config_string =
            serde_json::to_string(&services).context("Failed to serialize config to JSON.")?;
        fs::write(&config_path, config_string)
            .with_context(|| format!("Failed to write config to '{config_path:?}'."))?;

        self.save_model_configs()?;
        Ok(())
    }

    pub fn model_dir<S: AsRef<str>>(&self, model_name: S) -> PathBuf {
        self.path.join(model_name.as_ref())
    }

    pub fn create_model(&mut self, model_name: impl AsRef<str>) -> Result<()> {
        let model_name = model_name.as_ref();
        if self.models.contains_key(model_name) {
            bail!("Model '{model_name}' already exists in neuronav.",);
        }
        let model_dir = self.model_dir(model_name);
        if model_dir.is_dir() {
            bail!("Directory for model '{model_name}' already exists in neuronav.",);
        }
        fs::create_dir_all(&model_dir)
            .with_context(|| format!("Failed to create directory for '{model_name}'."))?;
        self.save()?;
        Ok(())
    }

    pub fn create_service(
        &mut self,
        name: impl AsRef<str>,
        provider: ServiceProvider,
    ) -> Result<()> {
        let name = name.as_ref();
        if self.services.contains_key(name) {
            bail!("Service '{name}' already exists in neuronav.",);
        }
        let service = Service::new(name, provider);
        self.services.insert(name.to_owned(), service);
        self.save()?;
        Ok(())
    }

    pub fn add_existing_neuroscope_dir(
        &mut self,
        model_name: impl AsRef<str>,
        source_path: impl AsRef<Path>,
    ) -> Result<()> {
        let model_name = model_name.as_ref();
        let model_dir = self.model_dir(model_name);
        if !model_dir.is_dir() {
            bail!("Model '{model_name}' does not exist in neuronav.",);
        }
        let path = source_path.as_ref();
        if !path.is_dir() {
            bail!("Source path '{path:?}' is not a directory.",);
        }
        let model_config = self
            .models
            .get_mut(model_name)
            .with_context(|| format!("Model '{model_name}' does not exist in neuronav.",))?;
        if model_config.has_serice("neuroscope") {
            bail!("Model '{model_name}' already has neuroscope.",);
        }
        let target_path = model_dir.join("neuroscope");
        if target_path.is_dir() {
            bail!("Directory '{target_path:?}' already exists.",);
        }
        model_config.add_service("neuroscope");
        fs::create_dir_all(&target_path)
            .with_context(|| format!("Failed to create directory '{target_path:?}'.",))?;
        fs_extra::dir::copy(path, &target_path, &fs_extra::dir::CopyOptions::new())
            .with_context(|| format!("Failed to copy '{path:?}' to '{target_path:?}'.",))?;
        self.save()?;
        Ok(())
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
