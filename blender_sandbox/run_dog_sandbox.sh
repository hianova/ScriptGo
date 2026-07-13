#!/bin/bash
echo "=========================================================="
echo " Starting RobotGo & Blender Quadruped Dog Simulation"
echo "=========================================================="

# Path to the Blender executable (macOS specific typical path)
BLENDER_EXEC="/Applications/Blender.app/Contents/MacOS/Blender"

if [ ! -f "$BLENDER_EXEC" ]; then
    if command -v blender >/dev/null 2>&1; then
        BLENDER_EXEC="blender"
    else
        echo "⚠️ Blender not found. Please ensure Blender is installed."
        BLENDER_EXEC=""
    fi
fi

echo "1. Building and starting RobotGo Dog Trainer..."
cd /Users/kuangtalin/Documents/RobotGo
cargo build --release --bin dog_trainer
./target/release/dog_trainer > trainer.log 2>&1 &
ROBOTGO_PID=$!

sleep 2

if [ -n "$BLENDER_EXEC" ]; then
    echo "2. Starting Blender Headless Sandbox with Quadruped Dog..."
    cd /Users/kuangtalin/Documents/ScriptGo
    $BLENDER_EXEC --background --python blender_sandbox/dog_sandbox.py > blender.log 2>&1 &
    BLENDER_PID=$!
fi

echo "Running simulation for 8 seconds..."
sleep 8

echo "3. Cleaning up processes..."
kill $ROBOTGO_PID
if [ -n "$BLENDER_PID" ]; then
    kill $BLENDER_PID
fi
echo "Done."
