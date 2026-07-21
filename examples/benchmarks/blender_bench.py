import time
import sys

def run_simulation(num_objects=100000, steps=600):
    positions_y = [float(i % 100) + 10.0 for i in range(num_objects)]
    velocities_y = [0.0] * num_objects
    
    dt = 1.0 / 60.0
    gravity = 9.8
    bounce = -0.8
    
    for frame in range(steps):
        for i in range(num_objects):
            vy = velocities_y[i]
            py = positions_y[i]
            
            vy -= gravity * dt
            py += vy * dt
            
            if py < 0.0:
                py = 0.0
                vy *= bounce
                
            velocities_y[i] = vy
            positions_y[i] = py
            
    print(f"{positions_y[1]:.2f}")
    
if __name__ == "__main__":
    run_simulation(100000, 600)
