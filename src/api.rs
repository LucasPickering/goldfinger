//! The API is the user-facing interface that allows the user to view and modify
//! state

use crate::{resource::lcd::LcdUserState, UserState};
use anyhow::Context;
use log::info;
use rocket::{form::Form, fs::FileServer, routes, serde::json::Json, State};
use rocket_dyn_templates::{context, Template};
use std::sync::Arc;

/// Launch the API
pub async fn start(user_state: Arc<UserState>) -> anyhow::Result<()> {
    rocket::build()
        .manage(user_state)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![index, get_lcd_json, set_lcd_json, set_lcd_form],
        )
        .mount("/static", FileServer::from("./static"))
        .launch()
        .await
        .context("Error starting API")?;
    Ok(())
}

#[rocket::get("/")]
fn index() -> Template {
    Template::render("index", context! {})
}

/// Get current LCD settings via JSON
#[rocket::get("/lcd", format = "json")]
async fn get_lcd_json(
    user_state: &State<Arc<UserState>>,
) -> Json<LcdUserState> {
    Json(*user_state.lcd.read().await)
}

/// Set LCD settings via JSON
#[rocket::post("/lcd", format = "json", data = "<data>")]
async fn set_lcd_json(
    user_state: &State<Arc<UserState>>,
    data: Json<LcdUserState>,
) -> Json<LcdUserState> {
    info!("Updating LCD state: {:?}", &data.0);
    *user_state.lcd.write().await = data.into_inner();
    Json(*user_state.lcd.read().await)
}

/// Set LCD settings via HTML form
#[rocket::post("/lcd", format = "form", data = "<data>")]
async fn set_lcd_form(
    user_state: &State<Arc<UserState>>,
    data: Form<LcdUserState>,
) {
    // TODO de-dupe code
    info!("Updating LCD state: {:?}", *data);
    *user_state.lcd.write().await = data.into_inner();
}
