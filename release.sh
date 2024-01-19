#!/bin/sh

set -ex

PI_HOST=pi@192.168.0.64
GOLDFINGER_DIR=/home/pi/goldfinger
PI_TARGET=arm-unknown-linux-musleabihf
FILES="goldfinger.service Rocket.toml static templates target/$PI_TARGET/release/goldfinger"

cargo build -v --release --target $PI_TARGET
rsync -r -vv $FILES $PI_HOST:$GOLDFINGER_DIR
ssh $PI_HOST << EOF
    sudo systemctl link $GOLDFINGER_DIR/goldfinger.service
    sudo systemctl enable goldfinger
    sudo systemctl restart goldfinger
EOF
