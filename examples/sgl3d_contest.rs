use rkyv::{Archive, Deserialize, Serialize};
use rkyv::rancor::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write, Read, BufWriter};
use std::time::Instant;
use std::path::Path;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub struct Face {
    pub v1: u32,
    pub v2: u32,
    pub v3: u32,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub struct Model3D {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
}

fn generate_huge_obj(path: &str, num_vertices: usize, num_faces: usize) {
    if Path::new(path).exists() {
        return;
    }
    println!("Generating huge .obj file ({} vertices, {} faces)...", num_vertices, num_faces);
    let file = File::create(path).unwrap();
    let mut writer = BufWriter::new(file);
    
    // Generate vertices
    for i in 0..num_vertices {
        let x = (i as f32) * 0.1;
        let y = (i as f32) * 0.2;
        let z = (i as f32) * 0.3;
        writeln!(writer, "v {} {} {}", x, y, z).unwrap();
    }
    
    // Generate faces
    for i in 0..num_faces {
        let v1 = (i % num_vertices) + 1;
        let v2 = ((i + 1) % num_vertices) + 1;
        let v3 = ((i + 2) % num_vertices) + 1;
        writeln!(writer, "f {} {} {}", v1, v2, v3).unwrap();
    }
    writer.flush().unwrap();
}

fn generate_sgl3d_from_obj(obj_path: &str, sgl3d_path: &str) {
    if Path::new(sgl3d_path).exists() {
        return;
    }
    println!("Converting .obj to .sgl3d binary format...");
    let model = parse_obj_naive(obj_path);
    
    let bytes = rkyv::to_bytes::<Error>(&model).unwrap();
    let mut file = File::create(sgl3d_path).unwrap();
    file.write_all(&bytes).unwrap();
}

fn parse_obj_naive(path: &str) -> Model3D {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    
    for line in reader.lines() {
        let line = line.unwrap();
        if line.starts_with("v ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 4 {
                vertices.push(Vertex {
                    x: parts[1].parse().unwrap(),
                    y: parts[2].parse().unwrap(),
                    z: parts[3].parse().unwrap(),
                });
            }
        } else if line.starts_with("f ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 4 {
                faces.push(Face {
                    v1: parts[1].parse().unwrap(),
                    v2: parts[2].parse().unwrap(),
                    v3: parts[3].parse().unwrap(),
                });
            }
        }
    }
    
    Model3D { vertices, faces }
}

fn parse_sgl3d_zerocopy(path: &str) {
    let mut file = File::open(path).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap(); // In a real mmap, this is 0-copy, but even reading into vec is fast.
    
    // Zero-copy access
    let start_check = Instant::now();
    let archived = unsafe { rkyv::access_unchecked::<<Model3D as rkyv::Archive>::Archived>(&bytes) };
    
    // Prove we can read it instantly
    let v_count = archived.vertices.len();
    let f_count = archived.faces.len();
    
    // Simulate ScriptGo physics processing (shift all X by 1.0)
    // Archived types are read-only, so for physics, ScriptGo would mutate a zero-copy mapped buffer 
    // or just iterate and apply it to a hardware display buffer.
    // For this benchmark, we just read them.
    let mut checksum = 0.0;
    for i in 0..1000.min(v_count) {
        checksum += archived.vertices[i].x.to_native();
    }
    
    let access_time = start_check.elapsed();
    println!("✅ SGL3D Zero-Copy access ready in: {:?}", access_time);
    println!("   Found {} vertices and {} faces (checksum: {})", v_count, f_count, checksum);
}

fn main() {
    println!("🪐 ScriptGo 3D: SGL3D (Zero-Copy) vs OBJ Parser 🪐\n");
    
    let obj_path = "/tmp/huge_model.obj";
    let sgl3d_path = "/tmp/huge_model.sgl3d";
    
    let v_count = 1_000_000;
    let f_count = 2_000_000;
    
    generate_huge_obj(obj_path, v_count, f_count);
    generate_sgl3d_from_obj(obj_path, sgl3d_path);
    
    println!("\n--------------------------------------------------");
    println!("Starting Naive .obj parsing (1M vertices, 2M faces)...");
    let start_obj = Instant::now();
    let model = parse_obj_naive(obj_path);
    let duration_obj = start_obj.elapsed();
    println!("✅ OBJ parsing completed in: {:?}", duration_obj);
    println!("   Loaded {} vertices, {} faces", model.vertices.len(), model.faces.len());
    
    println!("\n--------------------------------------------------");
    println!("Starting SGL3D Zero-Copy parsing...");
    let start_sgl = Instant::now();
    parse_sgl3d_zerocopy(sgl3d_path);
    let duration_sgl = start_sgl.elapsed();
    
    println!("\n--------------------------------------------------");
    let speedup = duration_obj.as_secs_f64() / duration_sgl.as_secs_f64();
    println!("🏆 SGL3D Format is {:.2}x FASTER than .obj parsing!", speedup);
}
