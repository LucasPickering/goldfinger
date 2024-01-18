//! The API is the user-facing interface that allows the user to view and modify
//! state

use crate::state::{LcdUserState, UserStateManager};
use rocket::{
    form::Form, fs::FileServer, response::Redirect, routes, serde::json::Json,
    Build, Rocket, State,
};
use rocket_dyn_templates::Template;
use std::sync::Arc;

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
async fn index(user_state: &State<Arc<UserStateManager>>) -> Template {
    Template::render("index", *user_state.read().await)
}

/// Get current LCD settings via JSON
#[rocket::get("/lcd", format = "json")]
async fn get_lcd_json(
    user_state: &State<Arc<UserStateManager>>,
) -> Json<LcdUserState> {
    Json(*user_state.read().await)
}

/// Set LCD settings via JSON
#[rocket::post("/lcd", format = "json", data = "<data>")]
async fn set_lcd_json(
    user_state: &State<Arc<UserStateManager>>,
    data: Json<LcdUserState>,
) -> Json<LcdUserState> {
    user_state.set(data.into_inner()).await.unwrap(); // TODO remove unwrap
    Json(*user_state.read().await)
}

/// Set LCD settings via HTML form
#[rocket::post("/lcd", format = "form", data = "<data>")]
async fn set_lcd_form(
    user_state: &State<Arc<UserStateManager>>,
    data: Form<LcdUserState>,
) -> Redirect {
    user_state.set(data.into_inner()).await.unwrap(); // TODO remove unwrap
    Redirect::to("/")
}
