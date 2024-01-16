//! The API is the user-facing interface that allows the user to view and modify
//! state

use crate::resource::lcd::LcdUserState;
use log::info;
use rocket::{
    form::Form, fs::FileServer, response::Redirect, routes, serde::json::Json,
    Build, Rocket, State,
};
use rocket_dyn_templates::{context, Template};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Attach all route handles to the given Rocket instance
pub fn mount_routes(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount(
            "/",
            routes![index, get_lcd_json, set_lcd_json, set_lcd_form],
        )
        .mount("/static", FileServer::from("./static"))
}

#[rocket::get("/")]
fn index() -> Template {
    Template::render("index", context! {})
}

/// Get current LCD settings via JSON
#[rocket::get("/lcd", format = "json")]
async fn get_lcd_json(
    user_state: &State<Arc<RwLock<LcdUserState>>>,
) -> Json<LcdUserState> {
    Json(*user_state.read().await)
}

/// Set LCD settings via JSON
#[rocket::post("/lcd", format = "json", data = "<data>")]
async fn set_lcd_json(
    user_state: &State<Arc<RwLock<LcdUserState>>>,
    data: Json<LcdUserState>,
) -> Json<LcdUserState> {
    info!("Updating LCD state: {:?}", &data.0);
    *user_state.write().await = data.into_inner();
    Json(*user_state.read().await)
}

/// Set LCD settings via HTML form
#[rocket::post("/lcd", format = "form", data = "<data>")]
async fn set_lcd_form(
    user_state: &State<Arc<RwLock<LcdUserState>>>,
    data: Form<LcdUserState>,
) -> Redirect {
    // TODO de-dupe code
    info!("Updating LCD state: {:?}", *data);
    *user_state.write().await = data.into_inner();
    Redirect::to("/")
}
