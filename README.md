# Goldfinger

_No Mr. Bond, I expect you to show the current time and/or weather forecast._

API and control system for a Raspberry Pi with an e-ink display. This is based loosely on [Söze](https://github.com/lucasPickering/soze), but is simplified and rewritten in Rust, with different hardware.

## Software

The software is a single Rust program, which includes a web server with both HTML and JSON interfaces for reading and modifying state. It computes state based on settings and external state (e.g. time or weather) and updates the hardware accordingly over the SPI device.

## Hardware

- [Raspberry Pi Zero W](https://www.raspberrypi.org/products/pi-zero/)
- [Adafruit 2.13" Monochrome E-Ink Bonnet](https://www.adafruit.com/product/4687)

## Development

I haven't figured out to run this locally, it needs some hardware mocking. Usually it's easiest to just run it on the Pi.

### Prerequisites

- `brew install filosottile/musl-cross/musl-cross --build-from-source --without-x86_64 --without-aarch64 --with-arm-hf` (for deployment only)
  - https://github.com/FiloSottile/homebrew-musl-cross

### Deployment

The executable is cross-compiled for the Raspberry Pi, then copied over with a script. Make sure you installed the correct linker in the prerequisites.

After any changes, deploy with:

```sh
./release.sh
```
