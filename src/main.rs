mod config;
mod display;
mod transit;
mod util;
mod weather;

use crate::{
    config::Config,
    display::{text, Display, FontSize},
    transit::Transit,
    weather::Weather,
};
use anyhow::Context;
use embedded_graphics::{
    geometry::AnchorX,
    prelude::{Dimensions, Point},
    text::Alignment,
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
    weather: Weather,
    transit: Transit,
}

impl Controller {
    /// Number of weather periods we can show at once
    const WEATHER_PERIODS: usize = 4;

    fn new() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let display = Display::new(&config)?;
        let weather = Weather::new(&config);
        let transit = Transit::new(&config);
        Ok(Self {
            display,
            weather,
            transit,
        })
    }

    fn tick(&mut self) -> anyhow::Result<()> {
        trace!("Running display tick");

        self.draw();

        // Redraw if anything changed
        self.display.draw()?;
        Ok(())
    }

    /// Draw screen contents to the buffer (but don't update the hardware)
    fn draw(&mut self) {
        // Weather
        if let Some(forecast) = self.weather.forecast() {
            let now = forecast.now();

            // Current temperature
            let temperature = format!("{}\n", now.temperature());
            let temperature_text = text(
                &temperature,
                (Display::LEFT, Display::TOP),
                FontSize::Large,
                Alignment::Left,
            );
            let temperature_right =
                temperature_text.bounding_box().anchor_x(AnchorX::Right);
            let mut next = self.display.draw_text(&temperature_text);
            next.y += 8; // Padding

            // Draw current PoP just to the right
            self.display.draw_text(&text(
                &now.prob_of_precip(),
                (temperature_right, Display::TOP),
                FontSize::Medium,
                Alignment::Left,
            ));

            // Show the next n periods
            for period in forecast.future_periods().take(Self::WEATHER_PERIODS)
            {
                next = self.display.draw_text(&text(
                    &format!(
                        "{} {:>4} {:>4}\n",
                        period.start_time().format("%_I%P"),
                        period.temperature(),
                        period.prob_of_precip(),
                    ),
                    next,
                    FontSize::Medium,
                    Alignment::Left,
                ));
            }
        }

        // Transit
        let predictions = self.transit.predictions();
        let mut next = Point::new(Display::RIGHT, Display::TOP);
        for line in predictions.lines {
            next = self.display.draw_text(&text(
                &format!(
                    "{}\n{}\n{}\n",
                    line.name, line.inbound, line.outbound
                ),
                next,
                FontSize::Medium,
                Alignment::Right,
            ));
            // The returned x is a bit shifted for some reason, so reset it
            next.x = Display::RIGHT;
            next.y += 8; // Padding between lines
        }
    }
}
