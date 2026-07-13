use std::net::UdpSocket;
use std::mem;
use vec101::{vec101_compute, vec101_context};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct BlenderState {
    frame: u32,
    x: f32,
    y: f32,
    z: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct BlenderForce {
    fx: f32,
    fy: f32,
    fz: f32,
}

pub fn main() -> std::io::Result<()> {
    println!("🚀 Starting Rust OODA Brain (ScriptGo Sandbox Bridge)...");
    
    // Bind the listener socket
    let socket = UdpSocket::bind("0.0.0.0:8888")?;
    socket.set_nonblocking(false)?;
    
    // Address of Blender to send forces back
    let blender_addr = "127.0.0.1:8889";
    
    println!("📡 Listening for Blender UDP stream on 0.0.0.0:8888...");
    println!("📡 Will send forces to Blender on {}", blender_addr);

    let mut buf = [0u8; 1024];

    loop {
        // Observe Phase
        let (amt, _src) = socket.recv_from(&mut buf)?;
        
        if amt >= mem::size_of::<BlenderState>() {
            let state: BlenderState = unsafe { 
                let mut s: BlenderState = mem::zeroed();
                std::ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    &mut s as *mut BlenderState as *mut u8,
                    mem::size_of::<BlenderState>()
                );
                s
            };

            // Orient & Decide Phase (using vec101 inference engine)
            let mut fx = 0.0;
            let fy = 0.0;
            let mut fz = 0.0;

            unsafe {
                // Initialize vec101 context with observed state
                let ctx: vec101_context = mem::zeroed();
                
                // We'll mock the inference execution since vec101_context internals are opaque.
                // In a real integration, state parameters are fed into the 1.58-bit neural block.
                vec101_compute(&ctx);
                
                // Extremely simple reflex algorithm to push the object back to origin
                // If it goes too far right (x > 0), push left (negative force)
                if state.x > 2.0 {
                    fx = -0.5;
                } else if state.x < -2.0 {
                    fx = 0.5;
                }
                
                // Dampen the gravity (if it falls too fast)
                if state.z < 0.0 {
                    fz = 0.8;
                }
            }

            // Act Phase
            let force = BlenderForce { fx, fy, fz };
            let force_bytes: [u8; mem::size_of::<BlenderForce>()] = unsafe { mem::transmute(force) };

            socket.send_to(&force_bytes, blender_addr)?;
            
            // Print occasionally to avoid spamming the console
            if state.frame.is_multiple_of(10) {
                println!("[OODA Loop] Frame: {:04} | Pos: ({:5.2}, {:5.2}, {:5.2}) | Action: Force({:5.2}, {:5.2}, {:5.2})",
                    state.frame, state.x, state.y, state.z, force.fx, force.fy, force.fz);
            }
        }
    }
}
