#!/bin/bash
set -e

# Start Xvfb
Xvfb :99 -screen 0 1280x720x24 &

# Wait for Xvfb to be ready
timeout=20
while ! xdpyinfo -display :99 >/dev/null 2>&1; do
    let timeout--
    if [ $timeout -le 0 ]; then
        echo "Timeout waiting for Xvfb"
        exit 1
    fi
    sleep 0.5
done

echo "Xvfb is ready on DISPLAY :99"

# Start the agent
exec /usr/local/bin/server "$@"
