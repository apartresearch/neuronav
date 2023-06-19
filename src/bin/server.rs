use std::sync::Arc;

use anyhow::{Context, Result};
use neuronav::{server, Neuronav};
use tokio::sync::Mutex;

pub fn main() -> Result<()> {
    let neuronav = Neuronav::from_dir("data").context("Failed to load neuronav.")?;
    server::start_server(Arc::new(Mutex::new(neuronav))).context("Failed to start server.")
}
