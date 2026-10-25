// SPDX-License-Identifier: Apache-2.0

//! MCP Server JSON-RPC 2.0 stdio & SSE transport implementation.

use axum::{
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::tools::{call_tool, list_tools};

/// Standing guidance returned to every MCP client in the `initialize`
/// result (MCP spec: `instructions` is surfaced to the model by
/// compliant hosts). Encodes the ownership doctrine from
/// `docs/kicad-workflows.md` in four rules an agent must never
/// violate, plus the gate order and a pointer to the full knowledge
/// base.
pub const SERVER_INSTRUCTIONS: &str = "Synth compiles .synth design sources into deterministic KiCad projects. Standing rules: (1) .synth sources are the only editable design files; generated .kicad_sch/.kicad_sym/.kicad_pro/.kicad_pcb and bom.csv are build artifacts — never hand-edit them, a re-export will silently destroy the edit. (2) Layout tuning lives in TWO sidecars with the same schema but different coordinate spaces, and they must not be mixed: <design>.schematic.layout.toml holds sheet millimetres (where a part sits on the page; written by synth_mutate_layout persist=true) and <design>.placement.layout.toml holds board millimetres (where a footprint sits on the PCB; written by synth_write_layout_override / synth_place_with_hints). Passing one where the other is expected silently misplaces parts. Tune visually through these files, never through schematic coordinates. (7) A part whose kicad_footprint names a footprint that is not installed is a broken reference, not a missing part: call synth_resolve_footprints to find the real one in the local KiCad library, and synth_export applies a confident, unambiguous match automatically. Never invent pin numbers to match a footprint — a ball-grid package uses designators like A22/AC13, and E-SYNTH-PIN-001 (blocking) is what catches fictional ones. Fix those from the datasheet. Before routing, call synth_place_with_hints with run_routing=false and inspect placement_quality.visual_review; if requires_revision is true, revise connector edge placement, orientation, breakout corridors, component grouping, or board compactness and rerun the review. This is a structural visual gate. (3) Schematic readability is reviewed on the rendered sheet, not inferred: call synth_render_schematic (or synth_review_schematic for a one-pass diagnostics+render packet) and actually look at the returned image. Fix what connectivity checks cannot — overlapping labels, confusing crossings, a component outside its group box, content past the page edge. Persist refinements with synth_mutate_layout persist=true (or synth_write_layout_override), never by editing schematic coordinates. Use synth_schematic_baseline to detect drift across revisions. (4) For difficult routing, inspect the unrouted net diagnostics and the report's `recommended_routing_order`; the external engine chooses its own net order, so treat that list as output feedback, not an input. Use `--place-route-iterate` (synth_export's `place_route_iterate`) to let the loop group the hardest nets' parts. (5) Part sourcing data (mpn/lcsc_pn) lives in registry part entries; the exporter stamps hidden MPN/LCSC fields onto every schematic instance — change parts at the source, never in BOM outputs. (6) A missing part is recoverable: search the registry, import or author a datasheet-backed Tier-2 part, confirm its pins and physical assets, then revalidate; never replace it with guessed pins or a different package silently. (8) Gate order before any handoff: synth_validate with zero blocking diagnostics, placement visual review, schematic render inspection, then synth_export, then 'kicad-cli sch erc' with zero errors. Export rejects unresolved compilation diagnostics; generated artifacts from failed attempts are not release evidence. Full review workflow, fabrication exports, Gerber review, panelization, and product-render guidance: docs/kicad-workflows.md.";

/// Promote any embedded `png_base64` payloads in a tool result into MCP
/// image content blocks, returning the pruned text value.
///
/// Recognises the shapes the schematic-review tools emit:
/// `synth_render_schematic`/`synth_review_schematic` put them under
/// `sheets[]` (and `render.sheets[]`), and `synth_schematic_baseline`
/// puts one at the top level for `inline_diff`. Promoting them means an
/// MCP host renders the sheet as an image instead of the model having to
/// decode base64 out of a JSON string.
fn extract_image_content(mut result: Value) -> (Value, Vec<String>) {
    fn take(map: &mut serde_json::Map<String, Value>, images: &mut Vec<String>) {
        if let Some(data) = map
            .remove("png_base64")
            .and_then(|v| v.as_str().map(str::to_string))
        {
            images.push(data);
        }
    }

    let mut images = Vec::new();

    if let Some(obj) = result.as_object_mut() {
        take(obj, &mut images);
    }
    if let Some(sheets) = result.get_mut("sheets").and_then(Value::as_array_mut) {
        for sheet in sheets {
            if let Some(obj) = sheet.as_object_mut() {
                take(obj, &mut images);
            }
        }
    }
    if let Some(sheets) = result
        .get_mut("render")
        .and_then(|r| r.get_mut("sheets"))
        .and_then(Value::as_array_mut)
    {
        for sheet in sheets {
            if let Some(obj) = sheet.as_object_mut() {
                take(obj, &mut images);
            }
        }
    }
    (result, images)
}

/// Handle an incoming MCP JSON-RPC request payload.
#[allow(clippy::needless_pass_by_value)]
pub fn handle_jsonrpc_request(req: Value, default_registry: Option<&Path>) -> Value {
    let jsonrpc = req["jsonrpc"].as_str().unwrap_or("2.0");
    let id = req["id"].clone();
    let method = req["method"].as_str().unwrap_or("");

    // JSON-RPC 2.0 Notification Rule: If `id` is null or missing, or if method is a notification,
    // NO response must be returned.
    if id.is_null() || method.starts_with("notifications/") {
        return Value::Null;
    }

    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match method {
        "initialize" => serde_json::json!({
            "jsonrpc": jsonrpc,
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "synth-mcp",
                    "version": "0.1.1"
                },
                "instructions": SERVER_INSTRUCTIONS
            }
        }),
        "tools/list" => {
            let tools = list_tools();
            serde_json::json!({
                "jsonrpc": jsonrpc,
                "id": id,
                "result": {
                    "tools": tools
                }
            })
        }
        "tools/call" => {
            let params = &req["params"];
            let name = params["name"].as_str().unwrap_or("");
            let arguments = params["arguments"].clone();

            match call_tool(name, &arguments, default_registry) {
                Ok(result_val) => {
                    let (text_val, images) = extract_image_content(result_val);
                    let mut content = vec![serde_json::json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&text_val).unwrap_or_default()
                    })];
                    for data in images {
                        content.push(serde_json::json!({
                            "type": "image",
                            "mimeType": "image/png",
                            "data": data
                        }));
                    }
                    serde_json::json!({
                        "jsonrpc": jsonrpc,
                        "id": id,
                        "result": { "content": content }
                    })
                }
                Err(err_msg) => serde_json::json!({
                    "jsonrpc": jsonrpc,
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": err_msg
                    }
                }),
            }
        }
        _ => serde_json::json!({
            "jsonrpc": jsonrpc,
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        }),
    }));

    match res {
        Ok(val) => val,
        Err(_) => serde_json::json!({
            "jsonrpc": jsonrpc,
            "id": id,
            "error": {
                "code": -32603,
                "message": "Internal MCP server panic caught"
            }
        }),
    }
}

/// Serialises everything written to stdout. A response can be larger than a pipe
/// buffer and so go out in several writes; a heartbeat line landing between them
/// would corrupt the message.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn write_message(message: &str) {
    let _guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(message.as_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

/// How often a running tool tells the client it is still working.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Sends an MCP progress notification at a fixed interval until dropped.
///
/// A tool such as `synth_export` can run for many minutes inside the router and
/// returns nothing until it is done. A client that times a call out after a
/// period without a response or progress, as Claude Code does after 30 minutes,
/// cannot tell that from a hang, and aborts a healthy export.
struct Heartbeat {
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl Heartbeat {
    fn start<F>(token: Value, interval: Duration, send: F) -> Self
    where
        F: Fn(String) + Send + 'static,
    {
        let stop = Arc::new((Mutex::new(false), Condvar::new()));
        let shared = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let (flag, wake) = &*shared;
            let mut beats: u64 = 0;
            loop {
                let guard = flag
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let (guard, _) = wake
                    .wait_timeout_while(guard, interval, |stopped| !*stopped)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if *guard {
                    break;
                }
                drop(guard);
                beats += 1;
                send(
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/progress",
                        "params": {
                            "progressToken": token,
                            "progress": beats,
                            "message": "synth is still working"
                        }
                    })
                    .to_string(),
                );
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Heartbeat {
    fn drop(&mut self) {
        let (flag, wake) = &*self.stop;
        *flag
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        wake.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The token the client asked progress to be reported under, or the request id
/// when it gave none: a client that never asked simply ignores the notification.
fn progress_token(req: &Value) -> Value {
    req["params"]["_meta"]["progressToken"]
        .as_str()
        .map(Value::from)
        .or_else(|| {
            req["params"]["_meta"]["progressToken"]
                .as_i64()
                .map(Value::from)
        })
        .unwrap_or_else(|| req["id"].clone())
}

/// Run MCP Server over standard I/O (stdin/stdout).
///
/// Uses plain newline-delimited JSON (one message per line), which is what
/// Antigravity, Claude Desktop, and Cursor all use for stdio MCP servers.
pub async fn run_stdio_server(default_registry: Option<PathBuf>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req_val: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("synth-mcp: json parse error: {e}");
                let err_resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Parse error: {e}") }
                });
                write_message(&serde_json::to_string(&err_resp).unwrap_or_default());
                continue;
            }
        };

        let heartbeat = (req_val["method"] == "tools/call").then(|| {
            Heartbeat::start(progress_token(&req_val), HEARTBEAT_INTERVAL, |message| {
                write_message(&message);
            })
        });
        let resp_val = handle_jsonrpc_request(req_val, default_registry.as_deref());
        drop(heartbeat);
        // Null = notification, no response.
        if resp_val.is_null() {
            continue;
        }
        write_message(&serde_json::to_string(&resp_val).unwrap_or_default());
    }
    Ok(())
}

/// Handle HTTP JSON-RPC requests for SSE/HTTP endpoint.
pub async fn handle_http_rpc(
    axum::extract::State(default_registry): axum::extract::State<Option<PathBuf>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let resp = handle_jsonrpc_request(payload, default_registry.as_deref());
    Json(resp)
}

/// Run MCP Server over HTTP (SSE / REST transport).
pub async fn run_sse_server(port: u16, default_registry: Option<PathBuf>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/rpc", post(handle_http_rpc))
        .route("/message", post(handle_http_rpc))
        .route("/sse", get(handle_http_rpc))
        .with_state(default_registry);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("Synth MCP Server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod heartbeat_tests {
    use super::*;

    #[test]
    fn a_running_call_sends_progress_until_it_returns() {
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&seen);
        let beat = Heartbeat::start(Value::from("t1"), Duration::from_millis(10), move |m| {
            sink.lock().unwrap().push(m);
        });
        std::thread::sleep(Duration::from_millis(80));
        drop(beat);
        let count = seen.lock().unwrap().len();
        assert!(count >= 2, "expected several beats, got {count}");
        let first: Value = serde_json::from_str(&seen.lock().unwrap()[0]).unwrap();
        assert_eq!(first["method"], "notifications/progress");
        assert_eq!(first["params"]["progressToken"], "t1");
        assert_eq!(first["params"]["progress"], 1);
        std::thread::sleep(Duration::from_millis(40));
        assert_eq!(
            seen.lock().unwrap().len(),
            count,
            "no beat after the call returned"
        );
    }

    #[test]
    fn the_token_is_the_one_the_client_gave_or_else_the_request_id() {
        let with = serde_json::json!({"id": 7, "params": {"_meta": {"progressToken": "abc"}}});
        assert_eq!(progress_token(&with), "abc");
        let numeric = serde_json::json!({"id": 7, "params": {"_meta": {"progressToken": 3}}});
        assert_eq!(progress_token(&numeric), 3);
        let without = serde_json::json!({"id": 7, "params": {}});
        assert_eq!(progress_token(&without), 7);
    }
}
