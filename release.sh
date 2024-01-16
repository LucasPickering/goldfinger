#!/bin/sh

set -ex

PI_HOST=pi@192.168.0.64
PI_TARGET=arm-unknown-linux-musleabihf
FILES="Rocket.toml static templates target/$PI_TARGET/release/goldfinger"

cargo build -v --release --target $PI_TARGET
rsync -r $FILES $PI_HOST:/home/pi/goldfinger/
# TODO copy systemd file
# TODO restart system service
