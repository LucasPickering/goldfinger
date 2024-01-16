use anyhow::anyhow;
use rocket::form::{FromFormField, ValueField};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

/// 32-bit Red-Green-Blue color. Serializes/deserializes as HTML format
/// (#rrggbb), for API compatibility.
#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub const BLACK: Self = Self {
        red: 0,
        green: 0,
        blue: 0,
    };

    pub fn red(self) -> u8 {
        self.red
    }

    pub fn green(self) -> u8 {
        self.green
    }

    pub fn blue(self) -> u8 {
        self.blue
    }

    pub fn to_bytes(self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }
}

// This is lossy, since we throw away the first 8 bytes. Hope it wasn't RGBA!
impl From<u32> for Color {
    fn from(value: u32) -> Self {
        // Casting will truncate the 24 most significant bits
        let red = (value >> 16) as u8;
        let green = (value >> 8) as u8;
        let blue = value as u8;
        Self { red, green, blue }
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> Self {
        ((color.red as u32) << 16)
            | ((color.green as u32) << 8)
            | color.blue as u32
    }
}

// This is lossy, since we throw away the first 8 bytes. Hope it wasn't RGBA!
impl FromStr for Color {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 7 && s.starts_with('#') {
            let value = u32::from_str_radix(&s[1..], 16)?;
            Ok(value.into())
        } else {
            Err(anyhow!("Invalid color string: {}", s))
        }
    }
}
impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:0>2x}{:0>2x}{:0>2x}", self.red, self.green, self.blue)
    }
}

// These impls are needed for serde
impl TryFrom<String> for Color {
    type Error = <Color as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Color> for String {
    fn from(color: Color) -> Self {
        color.to_string()
    }
}

impl<'a> FromFormField<'a> for Color {
    fn from_value(field: ValueField<'a>) -> rocket::form::Result<'a, Self> {
        field.value.parse().map_err(|_error| todo!())
    }
}
