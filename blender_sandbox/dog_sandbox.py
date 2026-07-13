import bpy
import socket
import struct
import random
import threading
import time
import math

# --- Configuration ---
RUST_IP = "127.0.0.1"
RUST_PORT = 8888
BLENDER_PORT = 8889
EPISODE_LENGTH = 120

# Global socket for sending and receiving
udp_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
udp_socket.bind(("0.0.0.0", BLENDER_PORT))
udp_socket.setblocking(False)

# Keep track of simulation frames
episode_frame_count = 0
simulation_running = True

body_obj = None
legs = []

def setup_scene():
    global body_obj, legs
    # Clear existing objects
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()
    
    # 1. Create Ground Plane as start
    bpy.ops.mesh.primitive_plane_add(size=4, location=(0, 0, 0))
    ground = bpy.context.object
    bpy.ops.rigidbody.object_add()
    ground.rigid_body.type = 'PASSIVE'
    
    # 2. Generate random uneven terrain stepping stones (梅花樁) along X path
    random.seed(42)  # Deterministic seed for reproducible testing
    for x in range(2, 25, 2):
        for y in [-0.8, 0.0, 0.8]:
            h = random.uniform(-0.15, 0.15)
            bpy.ops.mesh.primitive_cube_add(size=0.8, location=(x, y, h))
            stone = bpy.context.object
            bpy.ops.rigidbody.object_add()
            stone.rigid_body.type = 'PASSIVE'
            
    # Create Body (Active Rigid Body)
    bpy.ops.mesh.primitive_cube_add(size=1.5, location=(0, 0, 2))
    body_obj = bpy.context.object
    body_obj.name = "DogBody"
    bpy.ops.rigidbody.object_add()
    body_obj.rigid_body.mass = 10.0
    
    # Create 4 legs
    # Front-Left, Front-Right, Back-Left, Back-Right
    leg_offsets = [
        (0.8, 0.8, -0.5),   # FL (0)
        (0.8, -0.8, -0.5),  # FR (1)
        (-0.8, 0.8, -0.5),  # BL (2)
        (-0.8, -0.8, -0.5)  # BR (3) - Right Rear leg
    ]
    
    legs = []
    for i, offset in enumerate(leg_offsets):
        bpy.ops.mesh.primitive_cylinder_add(radius=0.15, depth=0.8, location=offset)
        leg = bpy.context.object
        leg.name = f"Leg_{i}"
        leg.parent = body_obj
        legs.append(leg)
        
    print("[Sandbox] Quadruped Dog Scene setup complete in Blender.")

setup_scene()

last_received_joint_commands = [50.0, 50.0, 50.0, 50.0]  # Extension for each leg (10..90 cm)

def udp_listener():
    print(f"[Sandbox] Dog UDP Listener started on port {BLENDER_PORT}...")
    while simulation_running:
        try:
            data, addr = udp_socket.recvfrom(1024)
            if len(data) >= 16:
                # Expecting 4 float32s representing leg extension heights
                commands = struct.unpack('<4f', data[:16])
                global last_received_joint_commands
                last_received_joint_commands = list(commands)
        except BlockingIOError:
            time.sleep(0.001)
        except Exception as e:
            print(f"UDP Recv Error: {e}")
            time.sleep(0.1)

# Start background listener thread
listener_thread = threading.Thread(target=udp_listener, daemon=True)
listener_thread.start()

def on_frame_change(scene):
    global episode_frame_count
    
    # --- Benchmark 2: Spawn payload at frame 40 ---
    if scene.frame_current == 40 and not bpy.data.objects.get("Payload"):
        loc = body_obj.location
        # Left-Front shoulder location (offset +X, +Y)
        bpy.ops.mesh.primitive_cube_add(size=0.3, location=(loc.x + 0.5, loc.y + 0.5, loc.z + 0.4))
        payload = bpy.context.object
        payload.name = "Payload"
        bpy.ops.rigidbody.object_add()
        payload.rigid_body.mass = 3.0 # 30% of body mass (10.0)
        
        # Fixed constraint to torso
        bpy.ops.object.empty_add(type='PLAIN_AXES', location=loc)
        joint = bpy.context.object
        joint.name = "PayloadJoint"
        bpy.ops.rigidbody.constraint_add()
        joint.rigid_body_constraint.type = 'FIXED'
        joint.rigid_body_constraint.object1 = body_obj
        joint.rigid_body_constraint.object2 = payload
        print("[Sandbox] ⚠️ Asymmetric payload (3.0 kg) attached to Left-Front shoulder!")

    # --- Benchmark 3: Leg 3 (Right-Rear) Motor fails at frame 70 ---
    heartbeat_mask = 0b1111  # All legs healthy (bit mask FL:0, FR:1, BL:2, BR:3)
    if scene.frame_current >= 70:
        # Force Right-Rear motor collapse
        last_received_joint_commands[3] = 10.0
        heartbeat_mask = 0b0111  # Leg 3 (Right-Rear) heartbeat lost!
        # Mute Right-Rear motor in Blender context
        if bpy.data.objects.get("Leg_3"):
            bpy.data.objects["Leg_3"].scale[2] = 0.1

    # 1. Apply joint commands to legs (OODA 'Act')
    for i, leg in enumerate(legs):
        if i == 3 and scene.frame_current >= 70:
            continue # Motor failed
            
        ext = last_received_joint_commands[i]
        ext_norm = (ext / 100.0)
        leg.scale[2] = max(0.1, min(1.5, ext_norm))
        
        # Physical push logic
        if ext > 50.0:
            body_obj.location[0] += 0.05

    # 2. Extract Body state (OODA 'Observe')
    if body_obj:
        loc = body_obj.location
        rot = body_obj.rotation_euler
        pitch_deg = math.degrees(rot[1])
        roll_deg = math.degrees(rot[0])
        
        # Pack state: Frame(uint32), X(float32), Y(float32), Z(float32), Pitch(float32), Roll(float32), Heartbeat(uint8)
        # 4 + 20 + 1 = 25 bytes
        payload = struct.pack('<I5fB', scene.frame_current, loc.x, loc.y, loc.z, pitch_deg, roll_deg, heartbeat_mask)
        
        # Stream to Rust
        try:
            udp_socket.sendto(payload, (RUST_IP, RUST_PORT))
        except BlockingIOError:
            pass
            
    episode_frame_count += 1
    
    # Reset episode / Exit Blender
    if episode_frame_count >= EPISODE_LENGTH:
        print("[Sandbox] Episode complete. Exiting...")
        global simulation_running
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
