use super::*;
use crate::mcp::creative;
use serde_json::{json, Value};
use tempfile::tempdir;

fn entry(key: &str) -> McpServerEntry {
    entry_for(server(key).expect(key))
}

fn read_mcp(ws: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(ws.join(".mcp.json")).unwrap()).unwrap()
}

#[test]
fn catalog_keys_are_unique_and_disjoint_from_creative() {
    let mut seen = BTreeSet::new();
    for s in CATALOG {
        assert!(seen.insert(s.key), "duplicate external key {}", s.key);
    }
    for c in creative::CATALOG {
        assert!(
            !seen.contains(c.key),
            "external key {} collides with a creative-tool key; the two \
             registries must own disjoint namespaces",
            c.key
        );
    }
}

#[test]
fn catalog_launchers_match_the_spiked_packages() {
    let api = server("perplexity").unwrap();
    assert_eq!(api.command, "npx");
    assert_eq!(api.default_args, &["-y", "@perplexity-ai/mcp-server"]);
    assert!(matches!(
        api.auth,
        AuthKind::ApiKey {
            env_var: "PERPLEXITY_API_KEY"
        }
    ));
    assert!(api.notes.contains("PERPLEXITY_API_KEY"));
    assert!(api.auth_token_relpath.is_none());

    let web = server("perplexity-web").unwrap();
    assert_eq!(web.command, "pwm-mcp");
    assert!(web.default_args.is_empty());
    assert!(matches!(
        web.auth,
        AuthKind::SessionLogin {
            setup_cmd: "pwm login"
        }
    ));
    assert!(web.notes.contains("Quota-limited"));
    assert!(web.notes.contains("30"));
    assert_eq!(
        web.auth_token_relpath,
        Some(".config/perplexity-web-mcp/token")
    );
    assert_eq!(web.launcher_aliases, &["pwm", "uvx"]);
    let probes: Vec<_> = web.launcher_probe_names().collect();
    assert_eq!(probes, ["pwm-mcp", "pwm", "uvx"]);
    assert!(api.launcher_aliases.is_empty());
}

#[test]
fn auth_configured_reports_presence_and_unknown() {
    assert_eq!(AuthKind::None.configured(), Some(true));
    assert_eq!(AuthKind::SessionLogin { setup_cmd: "x" }.configured(), None);
    // A definitionally-unset var — no `set_var` here: mutating the
    // environment races every other test thread's `getenv`.
    assert_eq!(
        AuthKind::ApiKey {
            env_var: "CA_EXTERNAL_MCP_DEFINITELY_UNSET",
        }
        .configured(),
        Some(false)
    );
    // The present-and-non-empty branch: `PATH` is always set.
    assert_eq!(
        AuthKind::ApiKey { env_var: "PATH" }.configured(),
        Some(true)
    );
}

#[test]
fn session_token_probe_is_existence_only() {
    let home = tempdir().unwrap();
    let rel = ".config/perplexity-web-mcp/token";
    assert!(
        !token_file_exists_in(home.path(), rel),
        "missing token file must report absent"
    );
    let path = home.path().join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"not-a-real-token\n").unwrap();
    assert!(token_file_exists_in(home.path(), rel));
    // The helper never returns the bytes; this assertion is the contract.
    let _ = std::fs::metadata(&path).unwrap();
}

#[test]
fn enabled_set_round_trips_and_drops_unknown_keys() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let ws = dir.path().join("proj");
    assert!(enabled_keys(&store, &ws).is_empty());

    let mut keys: BTreeSet<String> = BTreeSet::new();
    keys.insert("perplexity".to_string());
    keys.insert("not-a-real-server".to_string());
    set_enabled_keys(&store, &ws, &keys).unwrap();
    assert_eq!(
        enabled_keys(&store, &ws),
        ["perplexity".to_string()].into_iter().collect()
    );
}

#[test]
fn external_state_file_uses_its_own_suffix() {
    let dir = tempdir().unwrap();
    let store = HubStore::open(dir.path()).unwrap();
    let ws = dir.path().join("proj");
    let keys: BTreeSet<String> = ["perplexity".to_string()].into_iter().collect();
    set_enabled_keys(&store, &ws, &keys).unwrap();
    assert!(state_path(&store, &ws)
        .to_string_lossy()
        .ends_with(".external.json"));
}

#[test]
fn apply_adds_then_removes_only_its_own_keys() {
    let ws = tempdir().unwrap();
    let ws = ws.path();
    apply_to_workspace(ws, &[entry("perplexity")]).unwrap();
    let v = read_mcp(ws);
    assert_eq!(
        v["mcpServers"]["perplexity"]["command"], "npx",
        "PATH launchers render as a bare command, not an absolute path"
    );
    assert_eq!(
        v["mcpServers"]["perplexity"]["args"],
        json!(["-y", "@perplexity-ai/mcp-server"])
    );
    assert!(
        v["mcpServers"]["perplexity"].get("env").is_none(),
        "the registry must never write PERPLEXITY_API_KEY into a workspace file"
    );

    apply_to_workspace(ws, &[]).unwrap();
    let v = read_mcp(ws);
    assert!(v["mcpServers"]["perplexity"].is_null());
}

/// #276 / #277 RFR: enabling then disabling each catalog key adds then
/// removes exactly that entry and leaves a hand-added server alone.
#[test]
fn apply_enables_then_disables_each_perplexity_server_and_spares_hand_added() {
    for key in ["perplexity", "perplexity-web"] {
        let tmp = tempdir().unwrap();
        let ws = tmp.path();
        apply_to_workspace(ws, &[entry(key)]).unwrap();

        let mcp = ws.join(".mcp.json");
        let mut with_user: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
        with_user["mcpServers"]["user-fs"] = json!({ "command": "npx", "args": ["-y", "@mcp/fs"] });
        std::fs::write(&mcp, serde_json::to_string_pretty(&with_user).unwrap()).unwrap();

        let v = read_mcp(ws);
        assert!(v["mcpServers"][key].is_object(), "{key} must be present");
        assert_eq!(v["mcpServers"][key]["command"], entry(key).command);

        apply_to_workspace(ws, &[]).unwrap();
        let v = read_mcp(ws);
        assert!(
            v["mcpServers"][key].is_null(),
            "disabled {key} must be removed"
        );
        assert_eq!(
            v["mcpServers"]["user-fs"]["command"], "npx",
            "a hand-added server must survive disabling {key}"
        );
    }
}

/// The load-bearing contract for #276/#277: an `external` apply must
/// not disturb a `creative` registration and vice-versa, in either
/// order.
#[test]
fn external_and_creative_registries_compose_without_clobbering() {
    let ws = tempdir().unwrap();
    let ws = ws.path();
    let blender = creative::entry_for(
        creative::tool("coding-assistants-mcp-blender").unwrap(),
        Path::new("/opt/blender-bridge"),
    );

    // creative first, then external.
    creative::apply_to_workspace(ws, std::slice::from_ref(&blender)).unwrap();
    apply_to_workspace(ws, &[entry("perplexity")]).unwrap();
    let mcp_json = ws.join(".mcp.json");
    let v = read_mcp(ws);
    assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_object());
    assert!(v["mcpServers"]["perplexity"].is_object());

    // Disable external — blender must survive.
    apply_to_workspace(ws, &[]).unwrap();
    let v = read_mcp(ws);
    assert!(
        v["mcpServers"]["coding-assistants-mcp-blender"].is_object(),
        "creative entry must survive an external teardown"
    );
    assert!(v["mcpServers"]["perplexity"].is_null());

    // Re-enable external, then disable creative — external survives.
    apply_to_workspace(ws, &[entry("perplexity")]).unwrap();
    creative::apply_to_workspace(ws, &[]).unwrap();
    let v = serde_json::from_str::<Value>(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
    assert!(
        v["mcpServers"]["perplexity"].is_object(),
        "external entry must survive a creative teardown"
    );
    assert!(v["mcpServers"]["coding-assistants-mcp-blender"].is_null());
}

#[test]
fn apply_rejects_a_relative_workspace() {
    assert!(apply_to_workspace(Path::new("rel/path"), &[]).is_err());
}

#[test]
fn apply_is_idempotent() {
    let ws = tempdir().unwrap();
    apply_to_workspace(ws.path(), &[entry("perplexity")]).unwrap();
    let first = std::fs::read_to_string(ws.path().join(".mcp.json")).unwrap();
    let written = apply_to_workspace(ws.path(), &[entry("perplexity")]).unwrap();
    assert!(written.is_empty(), "second identical apply writes nothing");
    let second = std::fs::read_to_string(ws.path().join(".mcp.json")).unwrap();
    assert_eq!(first, second);
}

#[test]
fn web_entry_is_bare_pwm_mcp_with_no_args() {
    let e = entry("perplexity-web");
    assert_eq!(e.command, "pwm-mcp");
    assert!(e.args.is_empty());
}
