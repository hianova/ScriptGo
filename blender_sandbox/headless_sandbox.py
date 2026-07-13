import bpy
import socket
import struct
import random
import threading
import time

# --- Configuration ---
RUST_IP = "127.0.0.1"
RUST_PORT = 8888
BLENDER_PORT = 8889
EPISODE_LENGTH = 50

# Global socket for sending and receiving
udp_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
udp_socket.bind(("0.0.0.0", BLENDER_PORT))
udp_socket.setblocking(False)

# Keep track of simulation frames
episode_frame_count = 0
simulation_running = True
target_object = None

def setup_scene():
    global target_object
    # Clear existing objects
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()
    
    # Create Ground (Passive Rigid Body)
    bpy.ops.mesh.primitive_plane_add(size=10, location=(0, 0, -2))
    ground = bpy.context.object
    bpy.ops.rigidbody.object_add()
    ground.rigid_body.type = 'PASSIVE'
    
    # Create target cube (Active Rigid Body)
    bpy.ops.mesh.primitive_cube_add(size=1, location=(0, 0, 5))
    target_object = bpy.context.object
    bpy.ops.rigidbody.object_add()
    target_object.rigid_body.mass = 1.0
    
    print("[Sandbox] Demo scene created with Ground and falling Cube.")

setup_scene()

def randomize_parameters():
    """
    Randomizes physical properties to train the AI under extreme conditions.
    """
    try:
        # Example: Modifying a GeometryNodes parameter or simple physics gravity
        modifier = bpy.context.object.modifiers.get("GeometryNodes")
        if modifier:
            # Randomize fake parameters (Density, Length)
            modifier["Socket_2"] = random.uniform(0.1, 10000.0)
            modifier["Socket_3"] = random.uniform(0.01, 50.0)
            
        # Alternatively randomize scene gravity as a physical mutation
        bpy.context.scene.gravity[2] = random.uniform(-20.0, -1.0)
        print(f"[Sandbox] Parameters mutated. Gravity Z: {bpy.context.scene.gravity[2]:.2f}")
    except Exception as e:
        print(f"[Sandbox] Parameter mutation error: {e}")

def udp_listener():
    """
    Runs in a background thread, listening for control commands from Rust.
    """
    print(f"[Sandbox] UDP Listener started on port {BLENDER_PORT}...")
    while simulation_running:
        try:
            data, addr = udp_socket.recvfrom(1024)
            if len(data) >= 12:
                # Expecting a struct of 3 floats representing Force (X, Y, Z)
                force = struct.unpack('<3f', data[:12])
                
                # We can't safely modify bpy context from a background thread directly,
                # so we store it globally and apply in the next frame hook.
                global last_received_force
                last_received_force = force
        except BlockingIOError:
            time.sleep(0.001)
        except Exception as e:
            print(f"UDP Recv Error: {e}")
            time.sleep(0.1)

last_received_force = (0.0, 0.0, 0.0)

# Start background listener thread
listener_thread = threading.Thread(target=udp_listener, daemon=True)
listener_thread.start()

def on_frame_change(scene):
    """
    Hook called every time the frame advances.
    """
    global episode_frame_count
    
    # Apply forces received from Rust (OODA 'Act' phase)
    if target_object and last_received_force != (0.0, 0.0, 0.0):
        # Override physics location manually for the demo
        target_object.location[0] += last_received_force[0]
        target_object.location[1] += last_received_force[1]
        target_object.location[2] += last_received_force[2]

    # Extract state (OODA 'Observe' phase)
    if target_object:
        loc = target_object.location
        # Pack state: Frame(uint32), X(float32), Y(float32), Z(float32)
        # 4 + 4 + 4 + 4 = 16 bytes
        payload = struct.pack('<I3f', scene.frame_current, loc.x, loc.y, loc.z)
        
        # Stream to Rust
        try:
            udp_socket.sendto(payload, (RUST_IP, RUST_PORT))
        except BlockingIOError:
            pass

    episode_frame_count += 1
    
    # Check if episode is complete
    if episode_frame_count >= EPISODE_LENGTH:
        print(f"[Sandbox] Episode complete. Resetting...")
        episode_frame_count = 0
        scene.frame_set(1)
        randomize_parameters()

# Register the handler
bpy.app.handlers.frame_change_post.clear()
bpy.app.handlers.frame_change_post.append(on_frame_change)

print("[Sandbox] Headless script loaded. Starting simulation loop...")

# In headless mode, Blender exits when the script ends.
# We must manually advance frames to trigger physics and the frame_change_post handler.
try:
    while simulation_running:
        current = bpy.context.scene.frame_current
        bpy.context.scene.frame_set(current + 1)
        # Sleep a tiny bit to simulate realtime ~60FPS
        time.sleep(1.0 / 60.0)
except KeyboardInterrupt:
    print("[Sandbox] Simulation stopped by user.")
    simulation_running = False
