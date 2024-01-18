use crate::{
    resource::Resource,
    state::{LcdMode, LcdUserState},
    util::Color,
};
use anyhow::{anyhow, bail, Context};
use chrono::Local;
use log::{info, trace};
use serde::Deserialize;
use serialport::SerialPort;
use std::io::Write;

/// Width of the LCD, in characters
const LCD_WIDTH: usize = 20;
/// Height of the LCD in lines
const LCD_HEIGHT: usize = 4;

// Convenient consts for building up jumbo characters
const HBR: u8 = CustomCharacter::HalfBottomRight.tag();
const HBL: u8 = CustomCharacter::HalfBottomLeft.tag();
const BOT: u8 = CustomCharacter::Bottom.tag();
const FBR: u8 = CustomCharacter::FullBottomRight.tag();
const FBL: u8 = CustomCharacter::FullBottomLeft.tag();
const FUL: u8 = 0xff; // Solid block (built-in to the LCD)
const EMT: u8 = b' '; // Empty

/// LCD resource, to manage state calculation and hardware communication
pub struct Lcd {
    color: Color,
    text: LcdText,
    serial: LcdSerialPort,
}

/// Serial port config, to be loaded from Rocket config
#[derive(Deserialize)]
pub struct LcdConfig {
    #[serde(rename = "lcd_baud_rate")]
    baud_rate: u32,
    #[serde(rename = "lcd_port")]
    port: String,
}

impl Lcd {
    pub fn new(config: &LcdConfig) -> anyhow::Result<Self> {
        let serial = serialport::new(&config.port, config.baud_rate)
            .open()
            .with_context(|| {
                format!("Error connecting to LCD at {}", config.port)
            })?;

        Ok(Self {
            color: Color::default(),
            text: LcdText::default(),
            serial: LcdSerialPort(serial),
        })
    }

    /// Set background color. Does nothing if the color hasn't changed.
    fn set_color(&mut self, color: Color) -> anyhow::Result<()> {
        if self.color != color {
            self.color = color;
            self.serial.command(LcdCommand::SetColor { color })?;
        }
        Ok(())
    }

    /// Set LCD text. This will diff against current text and only send what's
    /// changed. The LCD is slow to update, so without diffing it will never
    /// settle. Input text is assumed to be ASCII, and have exactly [LCD_HEIGHT]
    /// lines.
    fn set_text(&mut self, text: LcdText) -> anyhow::Result<()> {
        // A list of groups that need their text updated. Each group will end
        // either at a non-diff byte, or end of line
        let mut diff_groups: Vec<TextGroup> = Vec::new();
        let mut current_diff_group: Option<TextGroup> = None;
        // Helper to terminate a group. This can't capture current_diff_group,
        // because we need &mut to it below
        let mut finish_group = |diff_group: &mut Option<TextGroup>| {
            // Move the current group out of the option, if present
            if let Some(diff_group) = diff_group.take() {
                diff_groups.push(diff_group);
            }
        };

        // Figure out what's changed. We're going to make a very bold assumption
        // that the input text is ASCII. If not, shit's fucked. Top-left to
        // bottom-right, line by line.
        for y in 0..LCD_HEIGHT {
            for x in 0..LCD_WIDTH {
                let old_byte = self.text.get(x, y);
                let new_byte = text.get(x, y);

                // Save the new byte, then check if it's a diff
                if old_byte == new_byte {
                    finish_group(&mut current_diff_group);
                } else {
                    self.text.set(x, y, new_byte);
                    match &mut current_diff_group {
                        // Start a new diff group
                        None => current_diff_group = Some(TextGroup::new(x, y)),
                        // Extend the current group
                        Some(diff_group) => {
                            diff_group.extend();
                        }
                    }
                }
            }

            // Always finish a group at the end of a line
            finish_group(&mut current_diff_group);
        }

        // Update the diffed sections
        for group in diff_groups {
            // LCD positions are 1-indexed!
            self.serial.write_at(
                group.x as u8 + 1,
                group.y as u8 + 1,
                self.text.get_slice(&group),
            )?;
        }

        Ok(())
    }
}

impl Resource for Lcd {
    fn name(&self) -> &str {
        "LCD"
    }

    fn on_start(&mut self) -> anyhow::Result<()> {
        self.serial.command(LcdCommand::Clear)?; // Reset text state
        self.serial.command(LcdCommand::BacklightOn)?;

        // Generally this init only needs to be done once ever, but it's safer
        // to do it on every startup
        self.serial.command(LcdCommand::SetSize {
            width: LCD_WIDTH as u8,
            height: LCD_HEIGHT as u8,
        })?;
        self.serial
            .command(LcdCommand::SetContrast { contrast: 255 })?;
        self.serial
            .command(LcdCommand::SetBrightness { brightness: 255 })?;
        for &character in CustomCharacter::ALL {
            self.serial.command(LcdCommand::SaveCustomCharacter {
                bank: 0,
                character,
            })?;
        }
        self.serial
            .command(LcdCommand::LoadCharacterBank { bank: 0 })
    }

    fn on_tick(&mut self, user_state: &LcdUserState) -> anyhow::Result<()> {
        match user_state.mode {
            LcdMode::Off => {
                self.serial.command(LcdCommand::BacklightOff)?;
                self.serial.command(LcdCommand::Clear)?;
                // Reset internal buffer so we reprint everything when switching
                // out of this mode
                self.text.clear();
            }
            LcdMode::Clock => {
                self.set_color(user_state.color)?;
                self.set_text(get_clock_text()?)?;
            }
        }
        Ok(())
    }
}

/// Blanket Drop impls aren't possible so we need this on the implementor :(
impl Drop for Lcd {
    fn drop(&mut self) {
        info!("Closing resource {}", self.name());
    }
}

/// Wrapper around a serial port, which abstracts some of the LCD-specific
/// logic.
struct LcdSerialPort(Box<dyn SerialPort>);

impl LcdSerialPort {
    /// Convert a command to bytes and send it over the port
    fn command(&mut self, command: LcdCommand) -> anyhow::Result<()> {
        trace!("Sending LCD command: {command:x?}");
        self.0.write(&command.into_bytes()).with_context(|| {
            format!("Error sending LCD command {command:x?}")
        })?;
        Ok(())
    }

    /// Move the cursor to the given position, then write some text
    fn write_at(&mut self, x: u8, y: u8, text: &[u8]) -> anyhow::Result<()> {
        self.command(LcdCommand::CursorPos { x, y })?;
        trace!("Writing text {text:?}");
        // Non-command bytes are interpreted as text
        // https://learn.adafruit.com/usb-plus-serial-backpack/sending-text
        self.0.write(text).with_context(|| {
            format!("Error writing LCD text at ({x}, {y}) {text:x?}")
        })?;
        Ok(())
    }
}

/// Text on the LCD. Use bytes instead of str/char because those allow unicode,
/// which we don't support. Every character is one byte in this world. The LCD
/// has autowrap enabled by default, so we can just shit out a stream of bytes
/// and it will figure it out.
#[derive(Copy, Clone, Debug)]
struct LcdText {
    lines: [[u8; LCD_WIDTH]; LCD_HEIGHT],
}

impl LcdText {
    fn get(&self, x: usize, y: usize) -> u8 {
        self.lines[y][x]
    }

    fn set(&mut self, x: usize, y: usize, value: u8) {
        self.lines[y][x] = value;
    }

    fn clear(&mut self) {
        self.lines = [[b' '; LCD_WIDTH]; LCD_HEIGHT];
    }

    /// Get a slice into a single line in the text. This assumes the
    /// position/length are valid!
    fn get_slice(&self, group: &TextGroup) -> &[u8] {
        &self.lines[group.y][group.x..group.x + group.length]
    }
}

impl Default for LcdText {
    fn default() -> Self {
        Self {
            // Default to whitespace
            lines: [[b' '; LCD_WIDTH]; LCD_HEIGHT],
        }
    }
}

/// Non-text commands for the LCD. There are more command types than this, but
/// we don't use them. Copy them in as needed.
/// https://learn.adafruit.com/usb-plus-serial-backpack/command-reference
#[derive(Copy, Clone, Debug)]
enum LcdCommand {
    Clear,
    BacklightOn,
    BacklightOff,
    SetSize {
        width: u8,
        height: u8,
    },
    SetBrightness {
        brightness: u8,
    },
    SetContrast {
        contrast: u8,
    },
    SetColor {
        color: Color,
    },
    /// Move the cursor to start editing at this position. Positions are
    /// 1-based, not 0-based.
    CursorPos {
        x: u8,
        y: u8,
    },
    /// Initialize a custom character, to be used later. The character will be
    /// accessed by its tag (from [CustomCharacter::tag])
    SaveCustomCharacter {
        bank: u8,
        character: CustomCharacter,
    },
    LoadCharacterBank {
        bank: u8,
    },
}

impl LcdCommand {
    const COMMAND_BYTE: u8 = 0xFE;

    fn into_bytes(self) -> Vec<u8> {
        // Every command status with a sentinel byte, followed by a tag byte
        let mut buffer = vec![Self::COMMAND_BYTE, self.tag()];

        // These writes are all infallible because they're into a buffer
        match self {
            Self::Clear => {}
            // Second param is the number of minutes to stay on, but it doesn't
            // actually get used, so we don't parameterize it
            Self::BacklightOn => buffer.write_all(&[0]).unwrap(),
            Self::BacklightOff => {}
            Self::SetSize { width, height } => {
                buffer.write_all(&[width, height]).unwrap()
            }
            Self::SetBrightness { brightness } => {
                buffer.write_all(&[brightness]).unwrap()
            }
            Self::SetContrast { contrast } => {
                buffer.write_all(&[contrast]).unwrap()
            }
            Self::SetColor { color } => {
                buffer.write_all(&color.to_bytes()).unwrap()
            }
            Self::CursorPos { x, y } => buffer.write_all(&[x, y]).unwrap(),
            Self::SaveCustomCharacter { bank, character } => {
                buffer.write_all(&[0xC1, bank, character.tag()]).unwrap();
                buffer.write_all(&character.pixels()).unwrap();
            }
            Self::LoadCharacterBank { bank } => {
                buffer.write_all(&[bank]).unwrap()
            }
        }

        buffer
    }

    /// Get the one-byte indentifier for a command type
    fn tag(&self) -> u8 {
        match self {
            Self::Clear => 0x58,
            Self::BacklightOn => 0x42,
            Self::BacklightOff => 0x46,
            Self::SetSize { .. } => 0xD1,
            Self::SetBrightness { .. } => 0x98,
            Self::SetContrast { .. } => 0x91,
            Self::SetColor { .. } => 0xD0,
            Self::CursorPos { .. } => 0x47,
            Self::SaveCustomCharacter { .. } => 0xC1,
            Self::LoadCharacterBank { .. } => 0xC0,
        }
    }
}

/// Custom characters that are used to create jumbo (multi-line) characters.
/// Each of these is defined as a 5x8 grid of pixels.
#[derive(Copy, Clone, Debug)]
enum CustomCharacter {
    HalfBottomRight,
    HalfBottomLeft,
    Bottom,
    FullBottomRight,
    FullBottomLeft,
}

impl CustomCharacter {
    const ALL: &'static [Self] = &[
        Self::HalfBottomRight,
        Self::HalfBottomLeft,
        Self::Bottom,
        Self::FullBottomLeft,
        Self::FullBottomRight,
    ];

    /// The byte representing this character when writing text. Also, the index
    /// of the character in the character bank
    const fn tag(self) -> u8 {
        match self {
            Self::HalfBottomRight => 0x00,
            Self::HalfBottomLeft => 0x01,
            Self::Bottom => 0x02,
            Self::FullBottomRight => 0x03,
            Self::FullBottomLeft => 0x04,
        }
    }

    /// Get the pixel grid that defines this character. Each character is 8
    /// lines of 5 pixels each, where each pixel can be on or off. These need
    /// to be loaded into the LCD at boot.
    fn pixels(self) -> [u8; 8] {
        match self {
            Self::HalfBottomRight => [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00011, 0b01111, 0b01111,
                0b11111,
            ],
            Self::HalfBottomLeft => [
                0b00000, 0b00000, 0b00000, 0b00000, 0b11000, 0b11110, 0b11110,
                0b11111,
            ],
            Self::Bottom => [
                0b00000, 0b00000, 0b00000, 0b00000, 0b11111, 0b11111, 0b11111,
                0b11111,
            ],
            Self::FullBottomRight => [
                0b11111, 0b11111, 0b11111, 0b11111, 0b11111, 0b01111, 0b01111,
                0b00011,
            ],
            Self::FullBottomLeft => [
                0b11111, 0b11111, 0b11111, 0b11111, 0b11111, 0b11110, 0b11110,
                0b11000,
            ],
        }
    }
}

/// A group of contiguous characters in a text block. Used to select blocks of
/// text for diff-only updating. This intentionally does *not* implement Copy,
/// to prevent logic bugs when mutating collections/options of this.
///
/// Groups *cannot* span multiple lines.
#[derive(Debug)]
struct TextGroup {
    x: usize,
    y: usize,
    length: usize,
}

impl TextGroup {
    fn new(x: usize, y: usize) -> Self {
        Self { x, y, length: 1 }
    }

    fn extend(&mut self) {
        self.length += 1;
    }
}

/// Format the current date+time as LCD text
fn get_clock_text() -> anyhow::Result<LcdText> {
    let now = Local::now();
    let mut text = LcdText::default();

    // First line is date and seconds. Month is abbreviated to prevent overflow
    let date = now.format("%A, %b %-d");
    let seconds = now.format("%S");
    // This should always be 20 chars
    text.lines[0] = format!("{date:<18}{seconds}")
        .into_bytes()
        .try_into()
        .map_err(|bytes| {
            anyhow!(
                "First clock line has incorrect length. \
                Expected {LCD_WIDTH} bytes, received {bytes:?}",
            )
        })?;

    // Next three lines are the time, in jumbo text
    // https://docs.rs/chrono/latest/chrono/format/strftime/index.html
    let time = now.format("%_I:%M").to_string();
    // Skip the first character to force some left padding
    let [_, line1, line2, line3] = &mut text.lines;
    let x_offset = 1;
    write_jumbo_text(
        [
            &mut line1[x_offset..],
            &mut line2[x_offset..],
            &mut line3[x_offset..],
        ],
        &time,
    )?;

    Ok(text)
}

/// Render a string as jumbo text. Jumbo characters are 3 lines tall, so we
/// need a 3-line slice.
fn write_jumbo_text(
    line_buffer: [&mut [u8]; 3],
    text: &str,
) -> anyhow::Result<()> {
    let mut x = 0; // Gets bumped up once per char
    for c in text.as_bytes() {
        // For each line, copy the bytes into our text array
        let jumbo_bytes = get_jumbo_character(*c)?;
        // Typically all lines will be the same length, but just be safe
        let x_len = jumbo_bytes
            .iter()
            .map(|line| line.len())
            .max()
            .unwrap_or_default();
        for y in 0..3 {
            let jumbo_bytes_line = jumbo_bytes[y];
            line_buffer[y][x..x + jumbo_bytes_line.len()]
                .copy_from_slice(jumbo_bytes_line);
        }
        // Adjust for this char's width, plus a space
        x += x_len + 1;
    }
    Ok(())
}

/// Get the bytes for a single jumbo character. Each character is exactly 3
/// bytes tall, but they have varying widths.
fn get_jumbo_character(character: u8) -> anyhow::Result<[&'static [u8]; 3]> {
    // We know how many lines we're modifying, but we *don't* know how many
    // chars per line we're writing, so those have to be slices
    let bytes: [&[u8]; 3] = match character {
        b'0' => [&[HBR, BOT, HBL], &[FUL, EMT, FUL], &[FBR, BOT, FBL]],
        b'1' => [&[BOT, HBL, EMT], &[EMT, FUL, EMT], &[BOT, FUL, BOT]],
        b'2' => [&[HBR, BOT, HBL], &[HBR, BOT, FBL], &[FBR, BOT, BOT]],
        b'3' => [&[HBR, BOT, HBL], &[EMT, BOT, FUL], &[BOT, BOT, FBL]],
        b'4' => [&[BOT, EMT, BOT], &[FBR, BOT, FUL], &[EMT, EMT, FUL]],
        b'5' => [&[BOT, BOT, BOT], &[FUL, BOT, HBL], &[BOT, BOT, FBL]],
        b'6' => [&[HBR, BOT, HBL], &[FUL, BOT, HBL], &[FBR, BOT, FBL]],
        b'7' => [&[BOT, BOT, BOT], &[EMT, HBR, FBL], &[EMT, FUL, EMT]],
        b'8' => [&[HBR, BOT, HBL], &[FUL, BOT, FUL], &[FBR, BOT, FBL]],
        b'9' => [&[HBR, BOT, HBL], &[FBR, BOT, FUL], &[EMT, EMT, FUL]],
        b' ' => [&[EMT, EMT, EMT], &[EMT, EMT, EMT], &[EMT, EMT, EMT]],
        b':' => [&[FUL], &[EMT], &[FUL]],
        _ => {
            bail!("Cannot convert character `{character}` to jumbo text");
        }
    };
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::lcd::{write_jumbo_text, LCD_WIDTH};

    #[test]
    fn test_jumbo_text() {
        let mut jumbo = [[b' '; LCD_WIDTH]; 3];
        let [line0, line1, line2] = &mut jumbo;
        write_jumbo_text([line0, line1, line2], "10:23").unwrap();
        assert_eq!(
            jumbo,
            [
                [
                    BOT, HBL, EMT, EMT, // 1
                    HBR, BOT, HBL, EMT, // 0
                    FUL, EMT, // :
                    HBR, BOT, HBL, EMT, // 2
                    HBR, BOT, HBL, // 3
                    EMT, EMT, EMT
                ],
                [
                    EMT, FUL, EMT, EMT, // 1
                    FUL, EMT, FUL, EMT, // 0
                    EMT, EMT, // :
                    HBR, BOT, FBL, EMT, // 2
                    EMT, BOT, FUL, // 3
                    EMT, EMT, EMT
                ],
                [
                    BOT, FUL, BOT, EMT, // 1
                    FBR, BOT, FBL, EMT, // 0
                    FUL, EMT, // :
                    FBR, BOT, BOT, EMT, // 2
                    BOT, BOT, FBL, // 3
                    EMT, EMT, EMT
                ]
            ]
        );
    }
}
