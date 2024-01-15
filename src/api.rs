//! The API is the user-facing interface that allows the user to view and modify
//! state

use crate::{resource::lcd::LcdUserState, UserState};
use anyhow::Context;
use rocket::{routes, serde::json::Json, State};
use std::sync::Arc;

/// Launch the API
pub async fn start(user_state: Arc<UserState>) -> anyhow::Result<()> {
    rocket::build()
        .manage(user_state)
        .mount("/", routes![get_lcd, set_lcd])
        .launch()
        .await
        .context("Error starting API")?;
    Ok(())
}

/// Get current LCD settings
#[rocket::get("/lcd")]
async fn get_lcd(user_state: &State<Arc<UserState>>) -> Json<LcdUserState> {
    Json(*user_state.lcd.read().await)
}

/// Set LCD settings
#[rocket::put("/lcd", data = "<data>")]
async fn set_lcd(
    user_state: &State<Arc<UserState>>,
    data: Json<LcdUserState>,
) -> Json<LcdUserState> {
    *user_state.lcd.write().await = data.into_inner();
    Json(*user_state.lcd.read().await)
}
