use anyhow::Context;
use log::info;
use serde::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub display_port: String,
}

impl Config {
    const PATH: &'static str = "./config.json";

    /// TODO
    pub fn load() -> anyhow::Result<Self> {
        info!("Loading config from `{}`", Self::PATH);
        let file = File::open(Self::PATH)?;
        serde_json::from_reader(file)
            .context(format!("Error parsing config file {}", Self::PATH))
    }
}
