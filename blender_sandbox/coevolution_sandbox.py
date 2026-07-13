import bpy
import socket
import struct
import json
import os
import math
import time
import sys

# --- Configuration ---
RUST_IP = "127.0.0.1"
RUST_PORT = 8888
BLENDER_PORT = 8889
EPISODE_LENGTH = 100

GENOME_PATH = "/Users/kuangtalin/Documents/RobotGo/genome.json"

# Load Genome
if os.path.exists(GENOME_PATH):
    with open(GENOME_PATH, 'r') as f:
        genome = json.load(f)
else:
    genome = {
        "body_length": 1.5,
        "body_width": 1.2,
        "body_height": 0.5,
        "num_legs": 4,
        "leg_length": 0.8,
        "has_wheels": False,
        "gait_speed": 0.15,
        "gait_amp": 25.0
    }

print(f"[Sandbox] Loaded genome: {genome}")

# Global socket for sending and receiving
udp_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
udp_socket.bind(("0.0.0.0", BLENDER_PORT))
udp_socket.setblocking(False)

episode_frame_count = 0
simulation_running = True
body_obj = None
limbs = []

def setup_scene():
    global body_obj, limbs
    # Clear existing objects
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()
    
    # 1. Create Starting Ground Platform
    bpy.ops.mesh.primitive_plane_add(size=4, location=(0, 0, 0))
    start_ground = bpy.context.object
    bpy.ops.rigidbody.object_add()
    start_ground.rigid_body.type = 'PASSIVE'
    
    # 2. Create Swamp Area (slippery, low friction) along X path from 4m to 12m
    bpy.ops.mesh.primitive_plane_add(size=8, location=(8, 0, -0.05))
    swamp = bpy.context.object
    bpy.ops.rigidbody.object_add()
    swamp.rigid_body.type = 'PASSIVE'
    swamp.rigid_body.friction = 0.01  # Very slippery swamp!
    
    # 3. Create High Walls along X path (e.g. at X=14m, height 1.2m)
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(14, 0, 0.6))
    wall = bpy.context.object
    wall.scale = (0.2, 3.0, 1.2)
    bpy.ops.rigidbody.object_add()
    wall.rigid_body.type = 'PASSIVE'

    # 4. Create procedural Robot Torso
    bpy.ops.mesh.primitive_cube_add(
        size=1.0,
        location=(0, 0, 1.5)
    )
    body_obj = bpy.context.object
    body_obj.name = "RobotBody"
    body_obj.scale = (genome["body_length"], genome["body_width"], genome["body_height"])
    bpy.ops.rigidbody.object_add()
    body_obj.rigid_body.mass = 10.0
    
    # 5. Create procedural Limbs (Wheels or Legs)
    limbs = []
    num_legs = int(genome["num_legs"])
    leg_len = float(genome["leg_length"])
    has_wheels = bool(genome["has_wheels"])
    
    if has_wheels:
        # Spawn 4 wheels on sides
        wheel_offsets = [
            (genome["body_length"]/2.0, genome["body_width"]/2.0, -0.2),
            (genome["body_length"]/2.0, -genome["body_width"]/2.0, -0.2),
            (-genome["body_length"]/2.0, genome["body_width"]/2.0, -0.2),
            (-genome["body_length"]/2.0, -genome["body_width"]/2.0, -0.2)
        ]
        for i, offset in enumerate(wheel_offsets):
            bpy.ops.mesh.primitive_cylinder_add(radius=0.3, depth=0.2, location=offset)
            wheel = bpy.context.object
            wheel.name = f"Wheel_{i}"
            wheel.parent = body_obj
            limbs.append(wheel)
    elif num_legs > 0:
        # Spawn legs based on num_legs (2, 4, 6, 8, etc.)
        # Place them symmetrically along the length of the torso
        for i in range(num_legs):
            # Calculate offset X and Y
            is_left = (i % 2 == 0)
            offset_y = (genome["body_width"]/2.0) if is_left else (-genome["body_width"]/2.0)
            
            # Distribute along X axis
            row = i // 2
            num_rows = math.ceil(num_legs / 2.0)
            if num_rows > 1:
                fraction = row / (num_rows - 1)
                offset_x = (fraction - 0.5) * genome["body_length"]
            else:
                offset_x = 0.0
                
            leg_loc = (offset_x, offset_y, -0.3)
            bpy.ops.mesh.primitive_cylinder_add(radius=0.1, depth=leg_len, location=leg_loc)
            leg = bpy.context.object
            leg.name = f"Leg_{i}"
            leg.parent = body_obj
            limbs.append(leg)
            
    print(f"[Sandbox] Procedural robot setup completed. Limbs count: {len(limbs)}")

setup_scene()

last_received_joint_commands = [50.0] * 16  # Pre-allocate commands buffer

def udp_listener():
    print(f"[Sandbox] Coevolution UDP Listener started on port {BLENDER_PORT}...")
    while simulation_running:
        try:
            data, addr = udp_socket.recvfrom(1024)
            # Expecting up to 16 float32s representing limb commands
            num_floats = len(data) // 4
            if num_floats > 0:
                commands = struct.unpack(f'<{num_floats}f', data[:num_floats * 4])
                global last_received_joint_commands
                last_received_joint_commands[:num_floats] = list(commands)
        except BlockingIOError:
            time.sleep(0.001)
        except Exception as e:
            print(f"UDP Recv Error: {e}")
            time.sleep(0.1)

# Start background listener thread
import threading
listener_thread = threading.Thread(target=udp_listener, daemon=True)
listener_thread.start()

def on_frame_change(scene):
    global episode_frame_count, simulation_running
    
    has_wheels = bool(genome["has_wheels"])
    num_legs = int(genome["num_legs"])
    leg_len = float(genome["leg_length"])
    
    # 1. Apply movements based on physical morphology (OODA 'Act')
    if has_wheels:
        # Under wheels, if speed > 0, we apply forward displacement.
        # But wait! Swamp friction is 0.01!
        # So wheels will slide and slip heavily in the swamp (from X=4m to X=12m)!
        # They will only walk well on high friction start ground.
        # This forces the GA to select legs instead of wheels for the swamp!
        loc = body_obj.location
        in_swamp = (loc.x >= 4.0 and loc.x <= 12.0)
        eff_speed = 0.02 if in_swamp else 0.15 # Massive slip in swamp!
        
        # Apply force/speed
        body_obj.location[0] += eff_speed
    elif num_legs > 0:
        # Legs: they step over swamp and walls!
        # Scale legs scale[2] based on commands from Rust
        for i, leg in enumerate(limbs):
            if i < len(last_received_joint_commands):
                ext = last_received_joint_commands[i]
                ext_norm = (ext / 100.0)
                leg.scale[2] = max(0.1, min(2.0, ext_norm))
                
                # Physical lift step logic:
                # If legs are long enough, they can step over the high walls (1.2m at X=14m)
                # If leg_length is small (e.g. < 0.6), the robot torso height will be low and it will crash into the wall and get stuck!
                # If leg_length is high (e.g. >= 1.0), it can step right over the wall!
                if ext > 50.0:
                    body_obj.location[0] += 0.06
                    
                    # Extra vertical lift if legs are long
                    if leg_len > 1.0:
                        body_obj.location[2] = max(body_obj.location[2], leg_len + 0.5)

    # 2. Extract Body state & Pitch (OODA 'Observe')
    if body_obj:
        loc = body_obj.location
        rot = body_obj.rotation_euler
        pitch_deg = math.degrees(rot[1])
        roll_deg = math.degrees(rot[0])
        
        # Send state to Rust
        # Pack state: Frame(uint32), X(float32), Y(float32), Z(float32), Pitch(float32), Roll(float32), HeartbeatMask(uint8)
        payload = struct.pack('<I5fB', scene.frame_current, loc.x, loc.y, loc.z, pitch_deg, roll_deg, 0b1111)
        try:
            udp_socket.sendto(payload, (RUST_IP, RUST_PORT))
        except BlockingIOError:
            pass
            
    episode_frame_count += 1
    
    # 3. Episode complete -> exit Blender headlessly!
    if episode_frame_count >= EPISODE_LENGTH:
        print("[Sandbox] Episode complete. Exiting...")
        simulation_running = False
        bpy.ops.wm.quit_blender()

# Register the handler
bpy.app.handlers.frame_change_post.clear()
bpy.app.handlers.frame_change_post.append(on_frame_change)

print("[Sandbox] Headless dog sandbox simulation loop starting...")

try:
    while simulation_running:
        current = bpy.context.scene.frame_current
        bpy.context.scene.frame_set(current + 1)
        time.sleep(1.0 / 60.0)
except KeyboardInterrupt:
    print("[Sandbox] Quadruped simulation stopped.")
    simulation_running = False
