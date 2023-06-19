use std::sync::Arc;

use crate::{
    data::{retrieve, NeuronViewerObject, NeuroscopePage},
    server, Neuronav,
};
use anyhow::{Context, Result};
use pyo3::prelude::*;
use tokio::{runtime::Runtime, sync::Mutex};

#[pyfunction]
fn debug() {
    println!("Hello from Rust!");
}

#[pyfunction]
fn scrape_layer_to_files(
    data_path: &str,
    model: &str,
    layer_index: u32,
    num_neurons: u32,
) -> PyResult<()> {
    Runtime::new()
        .context("Failed to start async runtime to scrape neuroscope.")?
        .block_on(async {
            println!("Scraping layer {layer_index} of model {model} to {data_path}.");
            retrieve::neuroscope::scrape_layer_to_files(data_path, model, layer_index, num_neurons)
                .await
                .context("Failed to scrape layer.")
        })?;
    Ok(())
}

#[pyclass(name = "NeuronViewerObject")]
struct PyNeuronViewerObject {
    object: NeuronViewerObject,
}

#[pymethods]
impl PyNeuronViewerObject {
    #[new]
    fn new(json: &str) -> PyResult<Self> {
        let json_value = serde_json::from_str(json).context("failed to parse json")?;
        Ok(PyNeuronViewerObject {
            object: NeuronViewerObject::from_json(&json_value)?,
        })
    }

    fn to_file(&self, path: &str) {
        self.object.to_file(path);
    }
}

#[pyclass(name = "NeuroscopePage")]
struct PyNeuroscopePage {
    object: NeuroscopePage,
}

#[pymethods]
impl PyNeuroscopePage {
    #[new]
    fn new(html: &str, layer_index: u32, neuron_index: u32) -> PyResult<Self> {
        Ok(PyNeuroscopePage {
            object: NeuroscopePage::from_html_str(html, layer_index, neuron_index)
                .with_context(|| format!("Failed to parse html of neuroscope page for neuron index {neuron_index} on layer {layer_index}."))?,
        })
    }

    fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.object).context("Failed to serialize neuroscope page.")
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.object)
    }

    fn to_file(&self, path: &str) -> Result<()> {
        self.object
            .to_file(path)
            .with_context(|| format!("Failed to write neuroscope page to file '{path:?}'."))
    }

    #[staticmethod]
    fn from_file(path: &str) -> Result<Self> {
        Ok(PyNeuroscopePage {
            object: NeuroscopePage::from_file(path)
                .with_context(|| format!("Failed to read neuroscope page from file '{path:?}'."))?,
        })
    }
}

#[pyclass(name = "Neuronav")]
pub struct PyNeuronav {
    neuronav: Arc<Mutex<Neuronav>>,
}

#[pymethods]
impl PyNeuronav {
    #[staticmethod]
    fn initialize(path: &str) -> PyResult<Self> {
        let neuronav = Arc::new(Mutex::new(Neuronav::initialize(path)?));
        Ok(PyNeuronav { neuronav })
    }

    #[staticmethod]
    fn from_dir(path: &str) -> PyResult<Self> {
        let neuronav = Arc::new(Mutex::new(Neuronav::from_dir(path)?));
        Ok(PyNeuronav { neuronav })
    }

    fn create_model(&mut self, model: &str) -> PyResult<()> {
        self.neuronav.blocking_lock().create_model(model)?;
        Ok(())
    }

    fn add_existing_neuroscope_dir(&mut self, model: &str, path: &str) -> PyResult<()> {
        self.neuronav
            .blocking_lock()
            .add_existing_neuroscope_dir(model, path)?;
        Ok(())
    }

    fn start_server(&self) -> PyResult<()> {
        server::start_server(Arc::clone(&self.neuronav))?;
        Ok(())
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn neuronav(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(debug, m)?)?;
    m.add_function(wrap_pyfunction!(scrape_layer_to_files, m)?)?;
    m.add_class::<PyNeuronViewerObject>()?;
    m.add_class::<PyNeuroscopePage>()?;
    m.add_class::<PyNeuronav>()?;
    Ok(())
}
