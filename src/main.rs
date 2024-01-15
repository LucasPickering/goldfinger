mod api;
mod resource;
mod util;

use crate::resource::{
    lcd::{Lcd, LcdUserState},
    Resource,
};
use clap::Parser;
use log::LevelFilter;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// Path to the LCD serial port. If not specified, a mock will be used
    #[clap(long)]
    lcd: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .parse_default_env()
        .init();
    let args = Args::parse();

    // TODO persist
    let user_state = Arc::new(UserState::default());

    // Spawn a background task monitor/update hardware
    {
        let lcd = Lcd::new(args.lcd.as_deref())?;
        let user_state = Arc::clone(&user_state);
        tokio::spawn(async move { lcd.run(&user_state.lcd).await });
    }

    // Primary task will run the API
    api::start(user_state).await
}

/// TODO move somewhere
#[derive(Default)]
struct UserState {
    lcd: RwLock<LcdUserState>,
}
