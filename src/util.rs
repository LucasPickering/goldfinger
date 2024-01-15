use anyhow::anyhow;
use log::info;
use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::{
    fmt::Display,
    io::{Read, Write},
    str::FromStr,
    time::Duration,
};

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

/// An implementation of [serialport::SerialPort] that doesn't actually connect
/// to a serial port.
#[derive(Clone, Debug)]
pub struct MockSerialPort {
    baud_rate: u32,
    timeout: Duration,
    data_bits: DataBits,
    flow_control: FlowControl,
    stop_bits: StopBits,
    parity: Parity,
}

impl Default for MockSerialPort {
    fn default() -> Self {
        Self {
            baud_rate: 0,
            timeout: Duration::default(),
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            stop_bits: StopBits::One,
            parity: Parity::None,
        }
    }
}

impl Read for MockSerialPort {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl Write for MockSerialPort {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SerialPort for MockSerialPort {
    fn name(&self) -> Option<String> {
        Some("Mock Serial Port".into())
    }

    fn baud_rate(&self) -> serialport::Result<u32> {
        Ok(self.baud_rate)
    }

    fn data_bits(&self) -> serialport::Result<DataBits> {
        Ok(self.data_bits)
    }

    fn flow_control(&self) -> serialport::Result<FlowControl> {
        Ok(self.flow_control)
    }

    fn parity(&self) -> serialport::Result<Parity> {
        Ok(self.parity)
    }

    fn stop_bits(&self) -> serialport::Result<StopBits> {
        Ok(self.stop_bits)
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn set_baud_rate(&mut self, baud_rate: u32) -> serialport::Result<()> {
        self.baud_rate = baud_rate;
        Ok(())
    }

    fn set_data_bits(
        &mut self,
        data_bits: serialport::DataBits,
    ) -> serialport::Result<()> {
        self.data_bits = data_bits;
        Ok(())
    }

    fn set_flow_control(
        &mut self,
        flow_control: serialport::FlowControl,
    ) -> serialport::Result<()> {
        self.flow_control = flow_control;
        Ok(())
    }

    fn set_parity(
        &mut self,
        parity: serialport::Parity,
    ) -> serialport::Result<()> {
        self.parity = parity;
        Ok(())
    }

    fn set_stop_bits(
        &mut self,
        stop_bits: serialport::StopBits,
    ) -> serialport::Result<()> {
        self.stop_bits = stop_bits;
        Ok(())
    }

    fn set_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> serialport::Result<()> {
        self.timeout = timeout;
        Ok(())
    }

    fn write_request_to_send(
        &mut self,
        _level: bool,
    ) -> serialport::Result<()> {
        Ok(())
    }

    fn write_data_terminal_ready(
        &mut self,
        _level: bool,
    ) -> serialport::Result<()> {
        Ok(())
    }

    fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }

    fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }

    fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }

    fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }

    fn bytes_to_read(&self) -> serialport::Result<u32> {
        Ok(0)
    }

    fn bytes_to_write(&self) -> serialport::Result<u32> {
        Ok(0)
    }

    fn clear(
        &self,
        _buffer_to_clear: serialport::ClearBuffer,
    ) -> serialport::Result<()> {
        Ok(())
    }

    fn try_clone(&self) -> serialport::Result<Box<dyn SerialPort>> {
        Ok(Box::new(self.clone()))
    }

    fn set_break(&self) -> serialport::Result<()> {
        Ok(())
    }

    fn clear_break(&self) -> serialport::Result<()> {
        Ok(())
    }
}

impl Drop for MockSerialPort {
    fn drop(&mut self) {
        info!("Closing serial port")
    }
}
