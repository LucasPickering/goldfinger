#!/bin/sh

set -ex

PI_HOST=pi@192.168.0.64
PROJECT_DIR=/home/pi/goldfinger
PI_TARGET=arm-unknown-linux-musleabihf
FILES="goldfinger.service Rocket.toml static templates target/$PI_TARGET/release/goldfinger"

cargo build -v --release --target $PI_TARGET
rsync -r -vv $FILES $PI_HOST:$PROJECT_DIR
ssh $PI_HOST << EOF
    sudo systemctl link $PROJECT_DIR/goldfinger.service
    sudo systemctl enable goldfinger
    sudo systemctl restart goldfinger
EOF
