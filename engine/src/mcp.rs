// SPDX-License-Identifier: AGPL-3.0-or-later
// Minimal MCP server over stdio (JSON-RPC 2.0): initialize, tools/list, tools/call.
// Gate rule: no tool parameter can mark a run proven; statuses come from the validator only.
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

fn handle_call(params: Value) -> Value {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let text = match name {
        "validate" => {
            let r = args.get("ref_dir").and_then(|v| v.as_str());
            let o = args.get("other_dir").and_then(|v| v.as_str());
            match (r, o) {
                (Some(a), Some(b)) => match bylazora::validator::compare_outputs(
                    &PathBuf::from(a), &PathBuf::from(b)) {
                    Ok(errs) if errs.is_empty() => "IDENTICAL".to_string(),
                    Ok(errs) => format!("MISMATCH\n{}", errs.join("\n")),
                    Err(e) => format!("ERROR: {e}"),
                },
                _ => "error: ref_dir and other_dir required".to_string(),
            }
        }
        "bench" => {
            let i = args.get("input_dir").and_then(|v| v.as_str());
            let o = args.get("output_dir").and_then(|v| v.as_str());
            let backend = args.get("backend").and_then(|v| v.as_str()).unwrap_or("cpu");
            match (i, o) {
                (Some(a), Some(b)) => {
                    let r = match backend {
                        "gpu" => bylazora::gpu::run(&PathBuf::from(a), &PathBuf::from(b), 0),
                        "cuda" => bylazora::cuda::run(&PathBuf::from(a), &PathBuf::from(b), 0),
                        _ => bylazora::cpu::run(&PathBuf::from(a), &PathBuf::from(b)),
                    };
                    match r { Ok(()) => "bench complete".to_string(), Err(e) => format!("ERROR: {e}") }
                }
                _ => "error: input_dir and output_dir required".to_string(),
            }
        }
        _ => "error: unknown tool".to_string(),
    };
    json!({ "content": [{ "type": "text", "text": text }] })
}

pub fn run() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let req: Value = match serde_json::from_str(&line) { Ok(v) => v, Err(_) => continue };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let response = match method {
            "initialize" => json!({ "jsonrpc": "2.0", "id": id, "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "Bylazora", "version": "0.4.1" } } }),
            "tools/list" => json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": [
                { "name": "validate", "description": "Byte-compare outputs; the validator is the only authority that marks a run proven.",
                  "inputSchema": { "type": "object", "properties": { "ref_dir": { "type": "string" }, "other_dir": { "type": "string" } }, "required": ["ref_dir", "other_dir"] } },
                { "name": "bench", "description": "Run the CPU or vendor-neutral GPU tier and write byte-exact outputs.",
                  "inputSchema": { "type": "object", "properties": { "input_dir": { "type": "string" }, "output_dir": { "type": "string" }, "backend": { "type": "string" } }, "required": ["input_dir", "output_dir"] } }
            ] } }),
            "tools/call" => {
                let result = handle_call(req.get("params").cloned().unwrap_or(Value::Null));
                json!({ "jsonrpc": "2.0", "id": id, "result": result })
            }
            "notifications/initialized" => continue,
            _ => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "method not found" } }),
        };
        writeln!(stdout, "{}", response)?;
        stdout.flush()?;
    }
    Ok(())
}
