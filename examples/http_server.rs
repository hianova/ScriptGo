use arc_swap::ArcSwap;
use script_go::assembler::parse_asm;
use script_go::instruction::Instruction;
use script_go::vm::ScriptVm;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

struct HttpGateway {
    script: ArcSwap<Vec<Instruction>>,
}

impl HttpGateway {
    pub fn new(source: &str) -> Self {
        let code = parse_asm(source).unwrap();
        Self {
            script: ArcSwap::from_pointee(code),
        }
    }

    #[inline(always)]
    pub fn handle_request(&self) -> u32 {
        let code = self.script.load();
        let mut vm = ScriptVm::new();
        // In reality, map HTTP payload/headers to registers here
        let _ = vm.run(&code);
        vm.registers[1] // Return some calculated result
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let script = "LOADIMM 1 200\nLOADIMM 2 55\nADD 1 1 2\nHALT";
    let gateway = Arc::new(HttpGateway::new(script));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("🚀 ScriptGo E2E HTTP Server listening on http://127.0.0.1:8080");
    println!("🧪 Run: wrk -t12 -c400 -d10s http://127.0.0.1:8080/");

    loop {
        let (mut socket, _) = listener.accept().await?;
        let gw = gateway.clone();

        // Fully embedded In-Process routing. ZERO CGI overhead.
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            if let Ok(n) = socket.read(&mut buf).await {
                if n > 0 {
                    let result = gw.handle_request();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                        result.to_string().len(),
                        result
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }
            }
        });
    }
}
