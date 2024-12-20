use crate::config::Config;
use anyhow::{anyhow, Context};
use embedded_graphics::{
    geometry::Point,
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    text::{Baseline, Text},
    Drawable,
};
use embedded_vintage_fonts::{FONT_12X16, FONT_24X32};
use linux_embedded_hal::{
    spidev::{SpiModeFlags, SpidevOptions},
    sysfs_gpio::Direction,
    Delay, SpidevDevice, SysfsPin,
};
use log::{info, trace};
use ssd1680::{
    color::Color,
    driver::{DisplayError, Ssd1680},
    graphics::{Display as _, Display2in13, DisplayRotation},
};
use std::{mem, time::Duration};

const PIN_BUSY: u64 = 17; // GPIO/BCM 17, pin 11
const PIN_DC: u64 = 22; // GPIO/BCM 22, pin 15
const PIN_RESET: u64 = 27; // GPIO/BCM 27, pin 13

/// Manage text state calculation and hardware communication
pub struct Display {
    // Hardware state
    device: Ssd1680<SpidevDevice, SysfsPin, SysfsPin, SysfsPin>,
    display: Display2in13,

    // Logical state
    /// The text currently on the screen
    text_buffer: Vec<TextItem>,
    /// The text to be written to the screen soon™. Empty except during a write
    /// tick
    next_text_buffer: Vec<TextItem>,
}

impl Display {
    pub const INTERVAL: Duration = Duration::from_millis(1000);

    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let mut spi =
            SpidevDevice::open(&config.display_port).context("SPI device")?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(1_000_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options).context("SPI configuration")?;

        let reset = init_pin(PIN_RESET, Direction::Out)
            .context("Initializing pin Reset")?;
        let dc =
            init_pin(PIN_DC, Direction::Out).context("Initializing pin D/C")?;
        let busy = init_pin(PIN_BUSY, Direction::In)
            .context("Initializing pin Busy")?;

        let controller = Ssd1680::new(spi, busy, dc, reset, &mut Delay)
            .map_err(map_error)?;
        info!("Display controller initialized");

        let mut display = Display2in13::bw();
        display.set_rotation(DisplayRotation::Rotate90);

        Ok(Self {
            device: controller,
            display,
            text_buffer: Vec::new(),
            next_text_buffer: Vec::new(),
        })
    }

    /// Add text to the buffer, to be written later. Return the dimensions of
    /// the text
    pub fn add_text(
        &mut self,
        text: String,
        (x, y): (i32, i32),
        font_size: FontSize,
    ) -> (i32, i32) {
        let (char_width, char_height) = font_size.char_dimensions();
        let width =
            text.lines().map(str::len).max().unwrap() as i32 * char_width;
        let height = text.lines().count() as i32 * char_height;

        self.next_text_buffer.push(TextItem {
            text,
            location: Point::new(x, y),
            font_size,
        });
        (width, height)
    }

    /// Draw current text buffer to the screen, if it's changed
    pub fn draw(&mut self) -> anyhow::Result<()> {
        // If anything changed, update the screen
        if self.draw_text()? {
            trace!("Sending frame to display");
            self.device
                .update_bw_frame(self.display.buffer())
                .map_err(map_error)?;
            trace!("Updating display");
            self.device.display_frame(&mut Delay).map_err(map_error)?;
            trace!("Done updating display");
        }
        Ok(())
    }

    /// If text has changed, flush all text from the buffer and write to the
    /// screen. If nothing changed, do nothing. Return whether or not the text
    /// changed.
    fn draw_text(&mut self) -> anyhow::Result<bool> {
        if self.next_text_buffer != self.text_buffer {
            trace!(
                "Text changed: old={:?}; new={:?}",
                self.text_buffer,
                self.next_text_buffer
            );
            self.text_buffer = mem::take(&mut self.next_text_buffer);

            self.display.clear_buffer(Color::White);
            for text_item in &self.text_buffer {
                let style = match text_item.font_size {
                    FontSize::Medium => {
                        MonoTextStyle::new(&FONT_12X16, BinaryColor::Off)
                    }
                    FontSize::Large => {
                        MonoTextStyle::new(&FONT_24X32, BinaryColor::Off)
                    }
                };
                Text::with_baseline(
                    &text_item.text,
                    text_item.location,
                    style,
                    Baseline::Top,
                )
                .draw(&mut self.display)
                .map_err(map_error)?;
            }

            Ok(true)
        } else {
            self.next_text_buffer.clear();
            Ok(false)
        }
    }
}

/// Available font sizes
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FontSize {
    Medium,
    Large,
}

impl FontSize {
    fn char_dimensions(&self) -> (i32, i32) {
        match self {
            FontSize::Medium => (12, 16),
            FontSize::Large => (24, 32),
        }
    }
}

#[derive(Debug, PartialEq)]
struct TextItem {
    text: String,
    location: Point,
    font_size: FontSize,
}

/// Initialize a GPIO pin
fn init_pin(pin_num: u64, direction: Direction) -> anyhow::Result<SysfsPin> {
    let pin = SysfsPin::new(pin_num);
    pin.export().context("Error exporting pin")?;
    while !pin.is_exported() {}
    pin.set_direction(direction)
        .context("Error setting pin direction")?;
    if matches!(direction, Direction::Out) {
        pin.set_value(1).context("Error enabling pin")?;
    }
    Ok(pin)
}

/// The error type from the driver doesn't implement Error so we have to map
/// manually
fn map_error(error: DisplayError) -> anyhow::Error {
    anyhow!("{error:?}")
}
