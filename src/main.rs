mod config;
#[cfg_attr(not(target_arch = "arm"), path = "mock_display.rs")]
mod display;
mod state;
mod weather;

use crate::{config::Config, display::Display, state::UserState};
use anyhow::Context;
use log::{info, LevelFilter};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module(env!("CARGO_PKG_NAME"), LevelFilter::Debug)
        .parse_default_env()
        .init();

    // Spawn a background task to monitor/update hardware
    let config = Config::load()?;
    let mut display = Display::new(&config)?;
    let state = UserState::default();

    let should_run = Arc::new(AtomicBool::new(true));

    let r = should_run.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .context("Error setting Ctrl-C handler")?;

    info!("Starting main loop");
    while should_run.load(Ordering::SeqCst) {
        display.tick(&state)?;
        thread::sleep(Display::INTERVAL);
    }

    Ok(())
}
