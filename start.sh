#!/bin/bash

# Check if cargo exists
if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: Cargo not found! Install rust first!"
    exit 1
fi

# Try different methods to check and kill process on port 3000
if command -v netstat >/dev/null 2>&1; then
    echo "Checking for processes on port 3000..."
    PORT_PID=$(netstat -tulpn 2>/dev/null | grep ":3000" | awk '{print $7}' | cut -d'/' -f1)
elif command -v ss >/dev/null 2>&1; then
    PORT_PID=$(ss -tulpn | grep ":3000" | awk '{print $7}' | cut -d',' -f2 | cut -d'=' -f2)
elif command -v fuser >/dev/null 2>&1; then
    PORT_PID=$(fuser 3000/tcp 2>/dev/null)
fi

if [ ! -z "$PORT_PID" ] && [ "$PORT_PID" != "" ]; then
    echo "Found process running on port 3000 (PID: $PORT_PID)"
    kill -15 "$PORT_PID" 2>/dev/null
    sleep 1
    if kill -0 "$PORT_PID" 2>/dev/null; then
        echo "Process didn't terminate gracefully, forcing..."
        kill -9 "$PORT_PID" 2>/dev/null
    fi
    echo "Port 3000 is now free"
fi

# Run project with error checking
echo "Starting server..."

# Handle Ctrl+C gracefully
cleanup() {
    echo -e "\nReceived shutdown signal. Stopping server..."
    exit 0
}
trap cleanup INT TERM

# Run with any provided arguments
if cargo run "$@"; then
    echo "Server stopped successfully!"
else
    echo "FATAL ERROR: Server failed to start!"
    exit 1
fi