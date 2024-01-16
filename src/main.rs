mod api;
mod resource;
mod util;

use crate::resource::{
    lcd::{Lcd, LcdUserState},
    Resource,
};
use anyhow::Context;
use log::LevelFilter;
use rocket_dyn_templates::Template;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Additional fields to load from Rocket configuration
#[derive(Deserialize)]
struct Config {
    lcd_serial_port: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .parse_default_env()
        .init();

    // TODO persist
    let user_state = Arc::new(UserState::default());

    // Set up main Rocket instance
    let rocket = rocket::build()
        .manage(Arc::clone(&user_state))
        .attach(Template::fairing());
    let rocket = api::mount_routes(rocket);
    let config: Config = rocket.figment().extract()?;

    // Spawn a background task to monitor/update hardware
    let lcd = Lcd::new(config.lcd_serial_port.as_deref())?;
    tokio::spawn(async move { lcd.run(&user_state.lcd).await });

    // Primary task will run the API
    rocket.launch().await.context("Error starting API")?;
    Ok(())
}

/// TODO move somewhere
#[derive(Default)]
struct UserState {
    lcd: RwLock<LcdUserState>,
}
