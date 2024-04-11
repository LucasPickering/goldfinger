use crate::{config::Config, state::UserState, weather::Weather};
use anyhow::Context;
use chrono::Local;
use embedded_graphics::{
    drawable::Drawable,
    fonts::{Font12x16, Font24x32, Font6x8, Text},
    geometry::Point,
    text_style,
};
use linux_embedded_hal::{
    spidev::{SpiModeFlags, SpidevOptions},
    sysfs_gpio::Direction,
    Delay, Pin, Spidev,
};
use log::{debug, info, trace};
use ssd1680::{
    color::{Black, White},
    driver::Ssd1680,
    graphics::{Display as _, Display2in13, DisplayRotation},
};
use std::time::Duration;

const PIN_CS: u64 = 8; // GPIO/BCM 8, pin 24
const PIN_BUSY: u64 = 17; // GPIO/BCM 17, pin 11
const PIN_DC: u64 = 22; // GPIO/BCM 22, pin 15
const PIN_RESET: u64 = 27; // GPIO/BCM 27, pin 13

/// Manage state calculation and hardware communication
pub struct Display {
    // Hardware state
    spi: Spidev,
    controller: Ssd1680<Spidev, Pin, Pin, Pin, Pin>,
    display: Display2in13,

    // Logical state
    /// The text currently on the screen
    text_buffer: Vec<TextItem>,
    weather: Weather,
}

impl Display {
    pub const INTERVAL: Duration = Duration::from_millis(1000);

    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let mut spi =
            Spidev::open(&config.display_port).context("SPI device")?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(1_000_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options).context("SPI configuration")?;

        let cs = init_pin(PIN_CS, Direction::Out).context("Pin CS")?;
        let reset = init_pin(PIN_RESET, Direction::Out).context("Pin Reset")?;
        let dc = init_pin(PIN_DC, Direction::Out).context("Pin D/C")?;
        let busy = init_pin(PIN_BUSY, Direction::In).context("Pin Busy")?;

        let controller =
            Ssd1680::new(&mut spi, cs, busy, dc, reset, &mut Delay)?;
        info!("Display controller initialized");

        let mut display = Display2in13::bw();
        display.set_rotation(DisplayRotation::Rotate90);

        Ok(Self {
            spi,
            controller,
            display,
            text_buffer: Vec::new(),
            weather: Weather::default(),
        })
    }

    pub fn tick(&mut self, state: &UserState) -> anyhow::Result<()> {
        debug!("Updating display");

        let mut text_buffer = Vec::new();

        // Clock
        // https://docs.rs/chrono/latest/chrono/format/strftime/index.html
        let now = Local::now();
        add_text(
            &mut text_buffer,
            now.format("%_I:%M").to_string(),
            (132, 0),
            FontSize::Large,
        );

        // Weather
        if let Some(forecast) = self.weather.forecast() {
            let periods = &forecast.properties.periods;
            let period = if state.weather_period < periods.len() {
                &periods[state.weather_period]
            } else {
                &periods[0]
            };

            add_text(
                &mut text_buffer,
                format!("{}°", period.temperature),
                (0, 0),
                FontSize::Large,
            );

            add_text(
                &mut text_buffer,
                format!(
                    "{}\n{}, {}%",
                    period.name.clone(),
                    period.short_forecast,
                    period.precipitation()
                ),
                (0, 36),
                FontSize::Medium,
            );
        }

        // If anything changed, update the screen
        if self.draw_text(text_buffer)? {
            trace!("Sending frame to display");
            self.controller
                .update_bw_frame(&mut self.spi, self.display.buffer())?;
            trace!("Updating display");
            self.controller.display_frame(&mut self.spi, &mut Delay)?;
            trace!("Done updating display");
        }
        Ok(())
    }

    /// If text has changed, flush all text from the buffer and write to the
    /// screen. If nothing changed, do nothing. Return whether or not the text
    /// changed.
    fn draw_text(&mut self, buffer: Vec<TextItem>) -> anyhow::Result<bool> {
        if buffer != self.text_buffer {
            trace!(
                "Text changed: old={:?}; new={:?}",
                self.text_buffer,
                buffer
            );
            self.text_buffer = buffer;

            for text_item in &self.text_buffer {
                let text = Text::new(&text_item.text, text_item.location);
                match text_item.font_size {
                    // The Font trait isn't object safe so we need static
                    // dispatch here, which is annoying
                    FontSize::Small => text
                        .into_styled(text_style!(
                            font = Font6x8,
                            text_color = Black,
                            background_color = White,
                        ))
                        .draw(&mut self.display),
                    FontSize::Medium => text
                        .into_styled(text_style!(
                            font = Font12x16,
                            text_color = Black,
                            background_color = White,
                        ))
                        .draw(&mut self.display),
                    FontSize::Large => text
                        .into_styled(text_style!(
                            font = Font24x32,
                            text_color = Black,
                            background_color = White,
                        ))
                        .draw(&mut self.display),
                }
                .context("Drawing text")?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Proxy for font sizes, because the ones from embedded_graphics aren't
/// object-safe
#[derive(Copy, Clone, Debug, PartialEq)]
enum FontSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, PartialEq)]
struct TextItem {
    text: String,
    location: Point,
    font_size: FontSize,
}

/// Initialize a GPIO pin
fn init_pin(pin_num: u64, direction: Direction) -> anyhow::Result<Pin> {
    let pin = Pin::new(pin_num);
    pin.export()?;
    while !pin.is_exported() {}
    pin.set_direction(direction)?;
    if matches!(direction, Direction::Out) {
        pin.set_value(1)?;
    }
    Ok(pin)
}

/// Add text to the buffer, to be written later
fn add_text(
    buffer: &mut Vec<TextItem>,
    text: String,
    (x, y): (i32, i32),
    font_size: FontSize,
) {
    buffer.push(TextItem {
        text,
        location: Point::new(x, y),
        font_size,
    })
}
