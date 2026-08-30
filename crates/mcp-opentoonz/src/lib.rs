//! Minimal MCP bridge for OpenToonz — **Tier 4, viability spike result.**
//!
//! Finding: the "one MCP server per running app" model does **not** fit
//! OpenToonz. It has no scripting API, no plugin IPC, and no Python/Lua.
//! Its `toonz_plugin` C++ SDK is for raster *image effects* only (tile
//! processing), not app automation. Mainline OpenToonz also has no
//! documented headless-render flag.
//!
//! So this bridge is deliberately small and does not talk to a running
//! instance:
//! - `scene_info` parses a `.tnz` file directly (it is XML) — reliable,
//!   needs no binary.
//! - `render` is a **best-effort passthrough** to `<opentoonz-bin> <args>`;
//!   whether your build renders headlessly is up to that build. It reports
//!   the exit status and captured output rather than pretending to know.
//!
//! If a future OpenToonz/Tahoma2D gains a real scripting or IPC surface,
//! replace this with a socket bridge like the others.

use mcp_core::app_link::{result_to_text, AppLink};
use mcp_core::{ServerInfo, ToolProvider, ToolResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

pub const APP_LABEL: &str = "OpenToonz";

/// Not a socket — `scene_info` reads the `.tnz` file, `render` shells out.
pub struct OpenToonzLink {
    pub bin: PathBuf,
    pub allow_render: bool,
}

impl AppLink for OpenToonzLink {
    fn request(&self, op: &str, args: &Value) -> Result<Value, String> {
        match op {
            "scene_info" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or("scene_info needs a `path`")?;
                let xml = std::fs::read_to_string(path)
                    .map_err(|e| format!("could not read {path}: {e}"))?;
                Ok(parse_tnz(&xml, path))
            }
            "render" => {
                if !self.allow_render {
                    return Err("render is disabled. Start the bridge with --allow-render (and note that mainline OpenToonz may have no headless render — this is a best-effort passthrough).".into());
                }
                let extra: Vec<String> = args
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let out = Command::new(&self.bin)
                    .args(&extra)
                    .output()
                    .map_err(|e| format!("could not run {}: {e}", self.bin.display()))?;
                Ok(json!({
                    "exit_ok": out.status.success(),
                    "exit_code": out.status.code(),
                    "stdout": String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    "stderr": String::from_utf8_lossy(&out.stderr).trim().to_string(),
                }))
            }
            other => Err(format!("unknown op {other}")),
        }
    }
}

/// Pull frame range, camera size and level references out of a `.tnz`
/// (Toonz scene XML). Best-effort string scanning — the format is stable
/// but not formally specified; missing fields come back absent, not an
/// error.
pub fn parse_tnz(xml: &str, path: &str) -> Value {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let open = xml.find(&format!("<{tag}"))?;
        let close = xml[open..].find('>')? + open;
        let seg = &xml[open..close];
        let key = format!("{name}=\"");
        let start = seg.find(&key)? + key.len();
        let end = seg[start..].find('"')? + start;
        Some(seg[start..end].to_string())
    };

    let levels: Vec<Value> = xml
        .match_indices("<level ")
        .map(|(i, _)| {
            let seg = &xml[i..xml[i..].find("/>").map(|e| i + e).unwrap_or(xml.len())];
            let get = |k: &str| {
                let key = format!("{k}=\"");
                let s = seg.find(&key)? + key.len();
                let e = seg[s..].find('"')? + s;
                Some(seg[s..e].to_string())
            };
            json!({ "name": get("name"), "type": get("type"), "path": get("scenePath").or_else(|| get("path")) })
        })
        .collect();

    json!({
        "path": path,
        "frame_count": attr("scene", "frameCount")
            .or_else(|| attr("frameCount", "value"))
            .and_then(|s| s.parse::<i64>().ok()),
        "camera_width": attr("camera", "width").or_else(|| attr("res", "x")),
        "camera_height": attr("camera", "height").or_else(|| attr("res", "y")),
        "level_count": levels.len(),
        "levels": levels,
        "note": "parsed from the .tnz XML directly; OpenToonz has no scripting API to query a live scene",
    })
}

pub struct OpenToonzProvider<L: AppLink> {
    link: L,
}

impl<L: AppLink> OpenToonzProvider<L> {
    pub fn new(link: L) -> Self {
        Self { link }
    }
}

impl<L: AppLink + 'static> ToolProvider for OpenToonzProvider<L> {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "coding-assistants-mcp-opentoonz".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "scene_info",
                "description": "Parse a .tnz (Toonz scene) file: frame count, camera resolution, and the levels it references. Reads the file directly — OpenToonz has no API to inspect a running scene.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "Absolute path to a .tnz file." } },
                    "required": ["path"],
                },
            }),
            json!({
                "name": "render",
                "description": "Best-effort passthrough: runs the OpenToonz binary with the given argv and returns its exit status + output. Mainline OpenToonz may have NO headless render mode — treat a nonzero exit as 'this build can't do it'. Gated behind --allow-render.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "args": { "type": "array", "items": { "type": "string" }, "description": "Raw argv passed to the OpenToonz binary." }
                    },
                    "required": ["args"],
                },
            }),
        ]
    }

    fn call(&self, name: &str, arguments: &Value) -> ToolResult {
        match self.link.request(name, arguments) {
            Ok(result) => ToolResult::Ok(result_to_text(&result)),
            Err(error) => ToolResult::Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TNZ: &str = r#"<?xml version="1.0"?>
<tnzscene>
  <scene frameCount="48">
    <camera width="1920" height="1080"/>
    <levels>
      <level name="bg" type="raster" scenePath="+drawings/bg.tlv"/>
      <level name="char" type="vector" scenePath="+drawings/char.pli"/>
    </levels>
  </scene>
</tnzscene>"#;

    #[test]
    fn parse_tnz_pulls_frames_camera_and_levels() {
        let v = parse_tnz(SAMPLE_TNZ, "/x/a.tnz");
        assert_eq!(v["frame_count"], 48);
        assert_eq!(v["camera_width"], "1920");
        assert_eq!(v["camera_height"], "1080");
        assert_eq!(v["level_count"], 2);
        assert_eq!(v["levels"][0]["name"], "bg");
        assert_eq!(v["levels"][1]["type"], "vector");
    }

    #[test]
    fn scene_info_reads_a_file_through_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.tnz");
        std::fs::write(&p, SAMPLE_TNZ).unwrap();
        let link = OpenToonzLink {
            bin: PathBuf::from("opentoonz"),
            allow_render: false,
        };
        let provider = OpenToonzProvider::new(link);
        let out = provider.call("scene_info", &json!({ "path": p.to_string_lossy() }));
        match out {
            ToolResult::Ok(text) => assert!(text.contains("\"frame_count\": 48")),
            ToolResult::Err(e) => panic!("expected ok: {e}"),
        }
    }

    #[test]
    fn render_is_refused_unless_allowed() {
        let link = OpenToonzLink {
            bin: PathBuf::from("opentoonz"),
            allow_render: false,
        };
        let provider = OpenToonzProvider::new(link);
        match provider.call("render", &json!({ "args": ["-render", "/x/a.tnz"] })) {
            ToolResult::Err(e) => assert!(e.contains("--allow-render")),
            ToolResult::Ok(_) => panic!("render should be refused"),
        }
    }

    #[test]
    fn tools_list_is_just_the_two() {
        let provider = OpenToonzProvider::new(OpenToonzLink {
            bin: PathBuf::from("x"),
            allow_render: true,
        });
        let names: Vec<_> = provider
            .tools()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(names, ["scene_info", "render"]);
    }
}
