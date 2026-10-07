# Goldfinger

> Do you expect me to talk?
>
> No Mr. Bond, I expect you to show me the current local weather!

This is a control system for a Raspberry Pi with an e-ink display. This was originally based on [Söze](https://github.com/LucasPickering/soze), but has since been simplified dramatically and rewritten in Rust, with different hardware.

## Software

The software is a single synchronous Rust program, which runs a main loop to update the display periodically. Background tasks use threads. It computes state based on settings and external state (e.g. time or weather) and updates the hardware accordingly over the SPI device. It's meant to be very simple.

## Hardware

- [Raspberry Pi Zero W](https://www.raspberrypi.org/products/pi-zero/)
- [Adafruit 2.13" Monochrome E-Ink Bonnet](https://www.adafruit.com/product/4687)

## Development

I haven't figured out to run this locally, it needs some hardware mocking. Usually it's easiest to just run it on the Pi.

### Prerequisites

- `brew install filosottile/musl-cross/musl-cross` (for deployment only)
  - https://github.com/FiloSottile/homebrew-musl-cross

### Pi Setup

From a fresh RPi OS installation, you'll need to enable both **SPI** and **GPIO via network** in `raspi-config`.

### Deployment

The executable is cross-compiled for the Raspberry Pi, then copied over with a script.

To run the program on the Pi with a live SSH session, run:

```sh
mise dev
```

To spawn the systemctl service and run it in the background:

```sh
mise deploy
```
