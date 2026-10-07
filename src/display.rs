use crate::config::Config;
use anyhow::{Context, anyhow};
use display_interface::DisplayError;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    Drawable,
    geometry::Point,
    text::{Alignment, Baseline, LineHeight, TextStyleBuilder},
};
use linux_embedded_hal::{
    Delay, SpidevDevice, SysfsPin,
    spidev::{SpiModeFlags, SpidevOptions},
    sysfs_gpio::Direction,
};
use log::{error, info};
use u8g2_fonts::{U8g2TextStyle, fonts};
use weact_studio_epd::{
    Color, WeActStudio213BlackWhiteDriver,
    graphics::{Display213BlackWhite, DisplayRotation},
};

const PIN_BUSY: u64 = 17; // GPIO/BCM 17, pin 11
const PIN_DC: u64 = 22; // GPIO/BCM 22, pin 15
const PIN_RESET: u64 = 27; // GPIO/BCM 27, pin 13

type Text<'a> = embedded_graphics::text::Text<'a, U8g2TextStyle<Color>>;

/// Manage text state calculation and hardware communication
pub struct Display {
    // Hardware state
    device: WeActStudio213BlackWhiteDriver<
        SPIInterface<SpidevDevice, SysfsPin>,
        SysfsPin,
        SysfsPin,
        Delay,
    >,
    display: Display213BlackWhite,

    // Logical state
    /// The text currently on the screen
    text_buffer: Vec<u8>,
}

impl Display {
    /// X coordinate of the left edge of the screen
    pub const LEFT: i32 = 0;
    /// Y coordinate of the top edge of the screen. The first 6 rows of the
    /// buffer are not visible
    pub const TOP: i32 = 6;

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
        let spi_interface = SPIInterface::new(spi, dc);

        let mut controller = WeActStudio213BlackWhiteDriver::new(
            spi_interface,
            busy,
            reset,
            Delay,
        );
        controller.init().map_err(map_error)?;
        info!("Display controller initialized");

        let mut display = Display213BlackWhite::new();
        display.set_rotation(DisplayRotation::Rotate90);

        Ok(Self {
            device: controller,
            display,
            text_buffer: Vec::new(),
        })
    }

    /// Draw some text to the screen buffer
    pub fn draw_text(&mut self, text: &Text) -> Point {
        text.draw(&mut self.display).expect("Infallible")
    }

    /// Draw current text buffer to the screen, if it's changed
    pub fn draw(&mut self) -> anyhow::Result<()> {
        // If anything changed, update the screen
        if self.display.buffer() != self.text_buffer {
            info!("Updating display");
            self.device.full_update(&self.display).map_err(map_error)?;
            // Store this buffer so we can check if it's changed later
            self.text_buffer = self.display.buffer().to_owned();
        }

        // After attempting a draw, clear no matter what so the next frame is
        // from a clean slate
        self.display.clear(Color::White);
        Ok(())
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        // Clear the screen on shutdown. A fast refresh leaves ghost text
        // behind, so do a full one
        info!("Clearing display for shutdown");

        self.display.clear(Color::White);
        let result = self.device.full_update(&self.display);
        if let Err(error) = result {
            error!("Failed to clear display on shutdown: {error:?}")
        }
        info!("Done clearing display");
    }
}

/// Available font sizes
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FontSize {
    Medium,
    Large,
}

impl FontSize {
    pub fn font(&self) -> U8g2TextStyle<Color> {
        match self {
            FontSize::Medium => U8g2TextStyle::new(
                fonts::u8g2_font_spleen12x24_me,
                Color::Black,
            ),
            FontSize::Large => U8g2TextStyle::new(
                fonts::u8g2_font_spleen32x64_me,
                Color::Black,
            ),
        }
    }

    /// Line height (in pixels) to get compact text
    fn line_height(&self) -> u32 {
        match self {
            FontSize::Medium => 19,
            FontSize::Large => 40,
        }
    }
}

/// Build a text object
pub fn text(
    text: &str,
    position: impl Into<Point>,
    font_size: FontSize,
    alignment: Alignment,
) -> Text<'_> {
    let character_style = font_size.font();
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Top)
        .alignment(alignment)
        .line_height(LineHeight::Pixels(font_size.line_height()))
        .build();
    Text::with_text_style(text, position.into(), character_style, text_style)
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
