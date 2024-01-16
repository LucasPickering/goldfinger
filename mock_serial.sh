#!/bin/sh

socat pty,link=/tmp/goldfinger_tty stdout | xxd
