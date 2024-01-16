mod api;
mod resource;
mod util;

use crate::resource::{
    lcd::{Lcd, LcdConfig, LcdUserState},
    Resource,
};
use anyhow::Context;
use log::LevelFilter;
use rocket_dyn_templates::Template;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .parse_default_env()
        .init();

    // TODO persist
    let user_state = Arc::new(RwLock::new(LcdUserState::default()));

    // Set up main Rocket instance
    let rocket = rocket::build()
        .manage(Arc::clone(&user_state))
        .attach(Template::fairing());
    let rocket = api::mount_routes(rocket);
    let lcd_config: LcdConfig = rocket.figment().extract()?;

    // Spawn a background task to monitor/update hardware
    let lcd = Lcd::new(&lcd_config)?;
    lcd.spawn_task(Arc::clone(&user_state));

    // Primary task will run the API
    rocket.launch().await.context("Error starting API")?;

    Ok(())
}
