mod config;
mod display;
mod state;
mod weather;

use crate::{
    config::Config,
    display::{text, Display, FontSize},
    state::{Mode, UserState},
    weather::Weather,
};
use anyhow::Context;
use embedded_graphics::{
    geometry::AnchorX, prelude::Dimensions, text::Alignment,
};
use log::{info, trace, warn, LevelFilter};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

/// Frequence to recalcuate display contents
const INTERVAL: Duration = Duration::from_millis(1000);

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module(env!("CARGO_PKG_NAME"), LevelFilter::Debug)
        .parse_default_env()
        .init();

    let mut controller = Controller::new()?;
    let should_run = Arc::new(AtomicBool::new(true));

    let r = should_run.clone();
    ctrlc::set_handler(move || {
        warn!("Exiting process");
        r.store(false, Ordering::SeqCst);
    })
    .context("Error setting Ctrl-C handler")?;

    info!("Starting main loop");
    while should_run.load(Ordering::SeqCst) {
        controller.tick()?;
        thread::sleep(INTERVAL);
    }

    Ok(())
}

/// Main controller class
struct Controller {
    display: Display,
    state: UserState,
    weather: Weather,
}

impl Controller {
    /// Number of weather periods we can show at once
    const WEATHER_PERIODS: usize = 4;

    fn new() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let display = Display::new(&config)?;
        let state = UserState::default();
        let weather = Weather::new(&config);
        Ok(Self {
            display,
            state,
            weather,
        })
    }

    fn tick(&mut self) -> anyhow::Result<()> {
        trace!("Running display tick");

        match self.state.mode {
            Mode::Weather => self.draw_weather()?,
        }

        // Redraw if anything changed
        self.display.draw()?;
        Ok(())
    }

    /// Draw screen contents for weather mode
    fn draw_weather(&mut self) -> anyhow::Result<()> {
        // Weather
        if let Some(forecast) = self.weather.forecast() {
            // Now
            let now = forecast.now();
            let temperature = now.temperature();
            let temperature_text =
                text(&temperature, (0, 0), FontSize::Large, Alignment::Left);
            let temperature_right =
                temperature_text.bounding_box().anchor_x(AnchorX::Right);
            let mut next = self.display.draw_text(&temperature_text)?;
            // Draw time and current PoP just to the right
            let right_next = self.display.draw_text(&text(
                "TODO",
                (0, temperature_right),
                FontSize::Medium,
                Alignment::Left,
            ))?;
            self.display.draw_text(&text(
                &now.prob_of_precip(),
                right_next,
                FontSize::Medium,
                Alignment::Left,
            ))?;

            next.y += 8; // Padding

            // Show the next n periods
            for period in forecast
                .future_periods()
                .skip(self.state.weather_period)
                .take(Self::WEATHER_PERIODS)
            {
                next = self.display.draw_text(&text(
                    &format!(
                        "{} {:>4} {:>4}",
                        period.start_time().format("%_I%P"),
                        period.temperature(),
                        period.prob_of_precip(),
                    ),
                    next,
                    FontSize::Medium,
                    Alignment::Left,
                ))?;
            }
        }

        Ok(())
    }
}
