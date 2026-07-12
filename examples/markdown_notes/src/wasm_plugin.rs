use anyhow::Result;
use wasmtime::*;

pub struct PluginState {
    pub result_buffer: String,
}

pub struct WasmPlugin {
    engine: Engine,
    module: Module,
}

impl WasmPlugin {
    pub fn new(wasm_bytes: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;
        Ok(Self { engine, module })
    }

    pub fn get_name(&self) -> Result<String> {
        self.call_plugin_method("plugin_name")
    }

    pub fn render(&self) -> Result<String> {
        self.call_plugin_method("plugin_render")
    }

    fn call_plugin_method(&self, method: &str) -> Result<String> {
        let mut store = Store::new(
            &self.engine,
            PluginState {
                result_buffer: String::new(),
            },
        );

        let mut linker = Linker::new(&self.engine);

        // host_set_result(ptr: i32, len: i32)
        linker.func_wrap("env", "host_set_result", |mut caller: Caller<'_, PluginState>, ptr: i32, len: i32| {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut buffer = vec![0; len as usize];
            memory.read(&caller, ptr as usize, &mut buffer).unwrap();
            let result_str = String::from_utf8_lossy(&buffer).into_owned();
            caller.data_mut().result_buffer = result_str;
        })?;

        // host_log(ptr: i32, len: i32)
        linker.func_wrap("env", "host_log", |mut caller: Caller<'_, PluginState>, ptr: i32, len: i32| {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let mut buffer = vec![0; len as usize];
            memory.read(&caller, ptr as usize, &mut buffer).unwrap();
            let result_str = String::from_utf8_lossy(&buffer).into_owned();
            println!("[WASM LOG]: {}", result_str);
        })?;

        let instance = linker.instantiate(&mut store, &self.module)?;
        
        let func = instance.get_func(&mut store, method)
            .ok_or_else(|| anyhow::anyhow!("Method {} not found in plugin", method))?;
            
        let typed_func = func.typed::<(), ()>(&store)?;
        typed_func.call(&mut store, ())?;

        let result = store.data().result_buffer.clone();
        Ok(result)
    }
}
