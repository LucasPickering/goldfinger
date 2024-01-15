//! The API is the user-facing interface that allows the user to view and modify
//! state

use crate::state::LcdUserState;
use anyhow::Context;
use rocket::{routes, serde::json::Json};

pub async fn start() -> anyhow::Result<()> {
    rocket::build()
        .mount("/", routes![get_lcd, set_lcd])
        .launch()
        .await
        .context("Error starting API")?;
    Ok(())
}

/// Get current LCD settings
#[rocket::get("/lcd")]
async fn get_lcd() -> Json<LcdUserState> {
    todo!()
}

/// Set LCD settings
#[rocket::put("/lcd", data = "<data>")]
async fn set_lcd(data: Json<LcdUserState>) -> Json<LcdUserState> {
    todo!()
}
