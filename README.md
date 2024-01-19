# Goldfinger

_No Mr. Bond, I expect you to show the current time and/or weather forecast._

API and control system for a Raspberry Pi with character LCD. This is based loosely on [Söze](https://github.com/lucasPickering/soze), but is simplified and rewritten in Rust.

## Software

The software is a single Rust program, which includes a web server with both HTML and JSON interfaces for reading and modifying state. It computes state based on settings and external state (e.g. time or weather) and updates the hardware accordingly over the serial device.

This can be run off-device without opening a serial port by simply running it without the `--lcd` parameter.

## Hardware

- [Raspberry Pi Zero W](https://www.raspberrypi.org/products/pi-zero/)
- [Adafruit RGB Backlight 20x4 Character LCD](https://www.winstar.com.tw/products/character-lcd-display-module/wh2004a.html)
  - [Datasheet](https://www.digikey.com.mx/htmldatasheets/production/1848324/0/0/1/wh2004a-cfh-jt-specification.html)
- [Adafruit LCD Backpack](https://www.adafruit.com/product/781)

The RPi is connected to the LCD backpack with a 3-pin connector that carries 5V, GND, and TX. The RPi sends commands and data to the backpack via UART. The UART settings are:

- 9600 baud rate
- 8 bits
- 1 stop bit
- No parity

Note that the backpack is meant to take 5V input on the data line, but the RPi only puts out 3.3V on its GPIO pins. Fortunately, 3.3V is high enough for the backpack to recognize as logical high, so no level converter is needed.

### Pin Layout

Specified pin numbers use the **hardware** pin numbering system.

| Purpose | Pin # |
| ------- | ----- |
| LCD 5V  | 4     |
| LCD GND | 6     |
| LCD TX  | 8     |

## Development

### Prerequisites

- `brew install socat`
- `brew install filosottile/musl-cross/musl-cross --build-from-source --without-x86_64 --without-aarch64 --with-arm-hf` (for deployment only)
  - https://github.com/FiloSottile/homebrew-musl-cross

To run this locally, you need to run a mock serial port in a separate window _before_ starting the server. This gives the LCD handler something to connect to.

```sh
# In terminal 1
./mock_serial.sh
# In terminal 2
cargo run
```

### Deployment

The executable is cross-compiled for the Raspberry Pi, then copied over with a script. Make sure you installed the correct linker in the prerequisites.

After any changes, deploy with:

```sh
./release.sh
```
