//! Test LCD behavior without having to run the whole webserver

use goldfinger::{
    resource::{
        lcd::{Lcd, LcdConfig},
        Resource,
    },
    state::{LcdMode, LcdUserState},
};
use log::LevelFilter;
use std::{thread, time::Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("goldfinger", LevelFilter::Trace)
        .parse_default_env()
        .init();

    let mut lcd = Lcd::new(&LcdConfig {
        port: "/dev/spidev0.0".into(),
    })?;
    lcd.on_start()?;
    println!("Ctrl-c to exit...");
    loop {
        lcd.on_tick(LcdUserState {
            mode: LcdMode::Weather,
        })?;
        thread::sleep(Duration::from_secs(5));
    }
}
