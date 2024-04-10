use anyhow::Context;
use goldfinger::{
    api,
    resource::{
        lcd::{Lcd, LcdConfig},
        Resource,
    },
    state::UserStateManager,
};
use log::LevelFilter;
use rocket_dyn_templates::Template;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .parse_default_env()
        .init();

    let user_state = Arc::new(UserStateManager::load().await);

    // Set up main Rocket instance
    let rocket = rocket::build()
        .manage(Arc::clone(&user_state))
        .attach(Template::fairing());
    let rocket = api::mount_routes(rocket);
    let lcd_config: LcdConfig = rocket.figment().extract()?;

    // Spawn a background task to monitor/update hardware
    let lcd = Lcd::new(&lcd_config)?;
    let join_handle = lcd.spawn_task(user_state);

    // Primary task will run the API
    rocket.launch().await.context("Error starting API")?;

    join_handle.abort();

    Ok(())
}
