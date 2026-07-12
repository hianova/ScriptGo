#[link(wasm_import_module = "env")]
extern "C" {
    fn host_set_result(ptr: *const u8, len: usize);
    fn host_log(ptr: *const u8, len: usize);
}

#[no_mangle]
pub extern "C" fn plugin_name() {
    let name = "To-Do List (Wasm)";
    unsafe {
        host_set_result(name.as_ptr(), name.len());
    }
}

#[no_mangle]
pub extern "C" fn plugin_render() {
    let msg = "Rendering plugin UI...";
    unsafe {
        host_log(msg.as_ptr(), msg.len());
    }

    let html = r#"
        <div style="padding: 30px; font-family: sans-serif; color: #fff;">
            <h1 style="font-size: 28px; margin-bottom: 20px;">✅ Wasm To-Do List</h1>
            <p style="color: #aaa; margin-bottom: 30px;">This entire UI is generated dynamically from a Wasm sandbox.</p>
            
            <div style="background: rgba(255,255,255,0.05); padding: 20px; border-radius: 10px;">
                <input type="text" placeholder="Add a new task..." style="width: 70%; padding: 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.2); background: rgba(0,0,0,0.3); color: white; margin-right: 10px;">
                <button style="padding: 10px 20px; background: #007aff; border: none; border-radius: 5px; color: white; cursor: pointer; font-weight: bold;">Add Task</button>
                
                <ul style="list-style: none; padding: 0; margin-top: 20px;">
                    <li style="padding: 15px; border-bottom: 1px solid rgba(255,255,255,0.1); display: flex; align-items: center;">
                        <input type="checkbox" style="margin-right: 15px; transform: scale(1.5);"> Build Wasm Platform
                    </li>
                    <li style="padding: 15px; border-bottom: 1px solid rgba(255,255,255,0.1); display: flex; align-items: center;">
                        <input type="checkbox" checked style="margin-right: 15px; transform: scale(1.5);"> Dominate Cross-Platform
                    </li>
                </ul>
            </div>
        </div>
    "#;
    
    unsafe {
        host_set_result(html.as_ptr(), html.len());
    }
}
