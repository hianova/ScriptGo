#!/bin/bash
echo "==========================================="
echo " Starting Blender Sandbox OODA Architecture"
echo "==========================================="

# Path to the Blender executable (macOS specific typical path)
BLENDER_EXEC="/Applications/Blender.app/Contents/MacOS/Blender"

if [ ! -f "$BLENDER_EXEC" ]; then
    # Fallback to checking if blender is in PATH
    if command -v blender >/dev/null 2>&1; then
        BLENDER_EXEC="blender"
    else
        echo "⚠️ Blender not found. Please ensure Blender is installed."
        echo "Will skip starting Blender and only start the Rust Brain."
        BLENDER_EXEC=""
    fi
fi

echo "1. Building and starting Rust Brain..."
cargo run --release --bin brain &
RUST_PID=$!

sleep 2

if [ -n "$BLENDER_EXEC" ]; then
    echo "2. Starting Blender Headless Sandbox..."
    $BLENDER_EXEC --background --python blender_sandbox/headless_sandbox.py &
    BLENDER_PID=$!
fi

echo "Running simulation for 10 seconds..."
sleep 10

echo "3. Cleaning up processes..."
kill $RUST_PID
if [ -n "$BLENDER_PID" ]; then
    kill $BLENDER_PID
fi
echo "Done."
