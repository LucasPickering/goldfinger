#!/bin/sh
# Run just the LCD code, without the webserver

set -ex

PI_HOST=pi@192.168.0.64
PROJECT_DIR=/home/pi/goldfinger
PI_TARGET=arm-unknown-linux-musleabihf
FILES="target/$PI_TARGET/release/examples/lcd"

cargo build -v --release --target $PI_TARGET --example lcd
rsync -r -vv $FILES $PI_HOST:$PROJECT_DIR
ssh $PI_HOST sudo $PROJECT_DIR/lcd
