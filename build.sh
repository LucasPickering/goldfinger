#!/bin/sh

set -e

PI_HOST=pi@192.168.0.64
PROJECT_DIR=/home/pi/goldfinger
PI_TARGET=arm-unknown-linux-musleabihf
FILES="goldfinger.service config.json target/$PI_TARGET/release/goldfinger"

cargo build -v --release --target $PI_TARGET
rsync -r -vv $FILES $PI_HOST:$PROJECT_DIR

if [ "$1" = "--release" ]; then
    echo "Starting systemd service..."
    ssh $PI_HOST << EOF
        sudo systemctl link $PROJECT_DIR/goldfinger.service
        sudo systemctl enable goldfinger
        sudo systemctl restart goldfinger
EOF
else
    echo "Running in dev mode..."
    # Run the program directly for testing
    ssh $PI_HOST << EOF
        sudo systemctl stop goldfinger
        cd ./goldfinger
        RUST_BACKTRACE=1 sudo -E ./goldfinger
EOF
fi
