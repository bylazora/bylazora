// SPDX-License-Identifier: AGPL-3.0-or-later
// Minimal MCP server over stdio (JSON-RPC 2.0): initialize, tools/list, tools/call.
// Gate rule: no tool parameter can mark a run proven; statuses come from the validator only.
//
// Nothing but JSON-RPC messages may reach stdout while this is running: every
// engine-tier progress line (cpu.rs, gpu.rs, cuda.rs, key.rs) goes to stderr
// for exactly this reason.
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

const KNOWN_BACKENDS: &[&str] = &["cpu", "gpu", "cuda"];

/// A tool's result and whether it represents a tool-level failure. MCP marks
/// failures with "isError": true in the result envelope, distinct from a
/// JSON-RPC protocol error; a validate MISMATCH is not itself a failure to
/// invoke the tool, but a rejected argument or a bench error is.
struct ToolResult {
    text: String,
    is_error: bool,
}

impl ToolResult {
    fn ok(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }
    fn err(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }
    fn into_json(self) -> Value {
        json!({ "content": [{ "type": "text", "text": self.text }], "isError": self.is_error })
    }
}

fn handle_call(params: &Value) -> ToolResult {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    match name {
        "validate" => {
            let r = args.get("ref_dir").and_then(|v| v.as_str());
            let o = args.get("other_dir").and_then(|v| v.as_str());
            match (r, o) {
                (Some(a), Some(b)) => {
                    match bylazora::validator::compare_outputs(&PathBuf::from(a), &PathBuf::from(b))
                    {
                        Ok(errs) if errs.is_empty() => ToolResult::ok("IDENTICAL"),
                        Ok(errs) => ToolResult::ok(format!("MISMATCH\n{}", errs.join("\n"))),
                        Err(e) => ToolResult::err(format!("ERROR: {e}")),
                    }
                }
                _ => ToolResult::err("error: ref_dir and other_dir required"),
            }
        }
        "bench" => {
            let i = args.get("input_dir").and_then(|v| v.as_str());
            let o = args.get("output_dir").and_then(|v| v.as_str());
            let backend = args
                .get("backend")
                .and_then(|v| v.as_str())
                .unwrap_or("cpu");
            if !KNOWN_BACKENDS.contains(&backend) {
                return ToolResult::err(format!(
                    "unknown backend: {backend} (expected one of {})",
                    KNOWN_BACKENDS.join(", ")
                ));
            }
            match (i, o) {
                (Some(a), Some(b)) => {
                    let r = match backend {
                        "gpu" => bylazora::gpu::run(&PathBuf::from(a), &PathBuf::from(b), 0),
                        #[cfg(feature = "cuda")]
                        "cuda" => bylazora::cuda::run(&PathBuf::from(a), &PathBuf::from(b), 0),
                        #[cfg(not(feature = "cuda"))]
                        "cuda" => Err(anyhow::anyhow!(
                            "cuda backend not available: this binary was built without --features cuda (requires the CUDA toolkit)"
                        )),
                        _ => bylazora::cpu::run(&PathBuf::from(a), &PathBuf::from(b)),
                    };
                    match r {
                        Ok(()) => ToolResult::ok("bench complete"),
                        Err(e) => ToolResult::err(format!("ERROR: {e}")),
                    }
                }
                _ => ToolResult::err("error: input_dir and output_dir required"),
            }
        }
        "copybook_parse" => {
            let src = args.get("src").and_then(|v| v.as_str());
            match src {
                Some(path) => match std::fs::read_to_string(path) {
                    Ok(text) => match bylazora::copybook::parse_copybook(&text) {
                        Ok(fields) => {
                            ToolResult::ok(bylazora::copybook::field_schema(&fields).to_string())
                        }
                        Err(e) => ToolResult::err(format!("ERROR: {e}")),
                    },
                    Err(e) => ToolResult::err(format!("ERROR: cannot read {path}: {e}")),
                },
                None => ToolResult::err("error: src required"),
            }
        }
        "migrate_verify" => {
            let job = args.get("job_dir").and_then(|v| v.as_str());
            match job {
                Some(dir) => match bylazora::migrate::cmd_verify(&PathBuf::from(dir)) {
                    Ok(()) => ToolResult::ok("IDENTICAL"),
                    Err(e) => ToolResult::err(e),
                },
                None => ToolResult::err("error: job_dir required"),
            }
        }
        "migrate_status" => {
            let job = args.get("job_dir").and_then(|v| v.as_str());
            match job {
                Some(dir) => match bylazora::migrate::cmd_status(&PathBuf::from(dir)) {
                    Ok(s) => ToolResult::ok(&s),
                    Err(e) => ToolResult::err(e),
                },
                None => ToolResult::err("error: job_dir required"),
            }
        }
        other => ToolResult::err(format!("error: unknown tool: {other}")),
    }
}

const TOOLS: &str = r#"[
    { "name": "validate", "description": "Byte-compare outputs; the validator is the only authority that marks a run proven.",
      "inputSchema": { "type": "object", "properties": { "ref_dir": { "type": "string" }, "other_dir": { "type": "string" } }, "required": ["ref_dir", "other_dir"] } },
    { "name": "bench", "description": "Run a tier and write byte-exact outputs.",
      "inputSchema": { "type": "object", "properties": { "input_dir": { "type": "string" }, "output_dir": { "type": "string" }, "backend": { "type": "string", "enum": ["cpu", "gpu", "cuda"], "description": "cpu (default), gpu (vendor-neutral wgpu, any adapter), or cuda (NVIDIA; needs a build with --features cuda)" } }, "required": ["input_dir", "output_dir"] } },
    { "name": "copybook_parse", "description": "Parse a COBOL copybook into its field schema.",
      "inputSchema": { "type": "object", "properties": { "src": { "type": "string" } }, "required": ["src"] } },
    { "name": "migrate_verify", "description": "Build, run and gate one migration workspace against its sealed reference. Returns the verdict record.", "inputSchema": { "type": "object", "properties": { "job_dir": { "type": "string" } }, "required": ["job_dir"] } },
    { "name": "migrate_status", "description": "Report a migration workspace's contract and its latest verdict.", "inputSchema": { "type": "object", "properties": { "job_dir": { "type": "string" } }, "required": ["job_dir"] } }
]"#;

/// Handle one JSON-RPC request. Returns None for a notification (a request
/// with no "id"): JSON-RPC forbids replying to notifications, and a client
/// that receives one anyway logs a protocol violation.
fn dispatch(req: &Value) -> Option<Value> {
    let id = req.get("id")?.clone();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    Some(match method {
        "initialize" => json!({ "jsonrpc": "2.0", "id": id, "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "Bylazora", "version": env!("CARGO_PKG_VERSION") } } }),
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        "tools/list" => {
            let tools: Value = serde_json::from_str(TOOLS).expect("TOOLS is valid JSON");
            json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tools } })
        }
        "tools/call" => {
            let result = handle_call(&req.get("params").cloned().unwrap_or(Value::Null));
            json!({ "jsonrpc": "2.0", "id": id, "result": result.into_json() })
        }
        _ => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "method not found" } })
        }
    })
}

pub fn run() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(response) = dispatch(&req) {
            writeln!(stdout, "{}", response)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_gets_no_reply() {
        let req = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(dispatch(&req).is_none());
    }

    #[test]
    fn ping_replies_with_empty_result() {
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
        let resp = dispatch(&req).unwrap();
        assert_eq!(resp["result"], json!({}));
    }

    #[test]
    fn initialize_reports_the_crate_version() {
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" });
        let resp = dispatch(&req).unwrap();
        assert_eq!(
            resp["result"]["serverInfo"]["version"],
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn unknown_method_gets_a_jsonrpc_error_not_a_panic() {
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "nonsense" });
        let resp = dispatch(&req).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn bench_rejects_unknown_backend_instead_of_running_cpu() {
        let params = json!({ "name": "bench", "arguments": { "input_dir": "x", "output_dir": "y", "backend": "quantum" } });
        let result = handle_call(&params);
        assert!(result.is_error);
        assert!(result.text.contains("unknown backend"), "{}", result.text);
    }

    #[test]
    fn bench_missing_args_is_an_error_result() {
        let params = json!({ "name": "bench", "arguments": {} });
        let result = handle_call(&params);
        assert!(result.is_error);
    }

    #[test]
    fn validate_missing_args_is_an_error_result() {
        let params = json!({ "name": "validate", "arguments": {} });
        let result = handle_call(&params);
        assert!(result.is_error);
    }

    #[test]
    fn unknown_tool_is_an_error_result() {
        let params = json!({ "name": "nonexistent" });
        let result = handle_call(&params);
        assert!(result.is_error);
    }

    #[test]
    fn tools_list_names_all_five_tools() {
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        let resp = dispatch(&req).unwrap();
        let names: Vec<&str> = resp["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "validate",
                "bench",
                "copybook_parse",
                "migrate_verify",
                "migrate_status"
            ]
        );
    }

    #[test]
    fn migrate_status_on_a_missing_workspace_is_an_error_result() {
        let params = json!({ "name": "migrate_status", "arguments": { "job_dir": "/tmp/definitely-not-a-job" } });
        let result = handle_call(&params);
        assert!(result.is_error, "{}", result.text);
        assert!(result.text.contains("cannot read"), "{}", result.text);
    }
}
