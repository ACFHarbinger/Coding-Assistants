use super::*;
use std::collections::HashSet;

#[test]
fn catalog_ids_are_unique_and_valid() {
    let mut seen_ids = HashSet::new();
    for spec in CATALOG {
        assert!(
            !spec.id.trim().is_empty(),
            "field id cannot be empty: {:?}",
            spec
        );
        assert!(
            seen_ids.insert(spec.id),
            "duplicate field id in CATALOG: {}",
            spec.id
        );
        assert!(
            crate::secret::validate_key(spec.id).is_ok(),
            "field id {} fails validate_key",
            spec.id
        );
        assert!(
            spec.id.starts_with(spec.owner_kind.as_str()),
            "field id {} should start with owner_kind prefix {}",
            spec.id,
            spec.owner_kind.as_str()
        );
    }
}

#[test]
fn catalog_fields_have_non_empty_metadata() {
    for spec in CATALOG {
        assert!(
            !spec.display_name.trim().is_empty(),
            "display_name is empty for {}",
            spec.id
        );
        assert!(
            !spec.owner_key.trim().is_empty(),
            "owner_key is empty for {}",
            spec.id
        );
    }
}

#[test]
fn catalog_env_vars_are_consistent_and_valid() {
    for spec in CATALOG {
        if let Some(env_var) = spec.env_var {
            assert!(
                !env_var.trim().is_empty(),
                "env_var cannot be empty for {}",
                spec.id
            );
            assert!(
                crate::secret::validate_key(env_var).is_ok(),
                "env_var {} for {} fails validate_key",
                env_var,
                spec.id
            );
            assert!(
                env_var
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                "env_var {} for {} must be uppercase alphanumeric or underscore",
                env_var,
                spec.id
            );
        }
    }
}

#[test]
fn catalog_accessors_find_expected_fields() {
    let deepseek_key = field("provider.deepseek.api_key").expect("deepseek api_key must exist");
    assert_eq!(deepseek_key.owner_kind, OwnerKind::Provider);
    assert_eq!(deepseek_key.owner_key, "deepseek");
    assert_eq!(deepseek_key.env_var, Some("DEEPSEEK_API_KEY"));
    assert!(deepseek_key.secret);
    assert_eq!(deepseek_key.scope, Scope::Global);
    assert_eq!(deepseek_key.vault_key(), "DEEPSEEK_API_KEY");

    assert!(field("non_existent_field_id").is_none());

    let deepseek_fields = fields_for(OwnerKind::Provider, "deepseek");
    assert_eq!(deepseek_fields.len(), 3);
    assert!(deepseek_fields
        .iter()
        .any(|f| f.id == "provider.deepseek.api_key"));
    assert!(deepseek_fields
        .iter()
        .any(|f| f.id == "provider.deepseek.base_url"));
    assert!(deepseek_fields
        .iter()
        .any(|f| f.id == "provider.deepseek.model"));

    let providers = fields_by_owner_kind(OwnerKind::Provider);
    assert!(!providers.is_empty());
    assert!(providers
        .iter()
        .all(|f| f.owner_kind == OwnerKind::Provider));

    let secrets = secret_fields();
    assert!(!secrets.is_empty());
    assert!(secrets.iter().all(|f| f.secret));
    assert!(secrets.iter().any(|f| f.id == "provider.deepseek.api_key"));
    assert!(secrets.iter().any(|f| f.id == "harness.cursor.login_token"));
    assert!(secrets.iter().any(|f| f.id == "mcp.perplexity.api_key"));

    let by_env = field_by_env_var("DEEPSEEK_API_KEY").expect("should find by env var");
    assert_eq!(by_env.id, "provider.deepseek.api_key");
    assert!(field_by_env_var("NON_EXISTENT_VAR").is_none());
}

#[test]
fn vault_key_fallback_works_when_no_env_var() {
    let muse_model = field("harness.muse.model").expect("muse model field exists");
    assert!(muse_model.env_var.is_none());
    assert_eq!(muse_model.vault_key(), "harness.muse.model");
}

#[test]
fn external_mcp_catalog_consistency() {
    // Cross-check with hub::mcp::external::CATALOG
    for ext_server in crate::mcp::external::CATALOG {
        match ext_server.auth {
            crate::mcp::external::AuthKind::ApiKey { env_var } => {
                let found = field_by_env_var(env_var).unwrap_or_else(|| {
                    panic!(
                        "external MCP server {} env_var {} missing in CATALOG",
                        ext_server.key, env_var
                    )
                });
                assert_eq!(found.owner_kind, OwnerKind::Mcp);
                assert_eq!(found.owner_key, ext_server.key);
                assert!(found.secret);
            }
            crate::mcp::external::AuthKind::SessionLogin { .. } => {
                let mcp_fields = fields_for(OwnerKind::Mcp, ext_server.key);
                assert!(
                    !mcp_fields.is_empty(),
                    "session login MCP server {} missing in CATALOG",
                    ext_server.key
                );
                assert!(mcp_fields.iter().any(|f| !f.secret));
            }
            crate::mcp::external::AuthKind::None => {}
        }
    }
}

#[test]
fn field_spec_serde_camel_case_serialization() {
    let spec = field("provider.deepseek.api_key").expect("deepseek field exists");
    let json_val = serde_json::to_value(spec).expect("serialize field spec");

    assert_eq!(json_val["id"], "provider.deepseek.api_key");
    assert_eq!(json_val["displayName"], "DeepSeek API Key");
    assert_eq!(json_val["ownerKind"], "provider");
    assert_eq!(json_val["ownerKey"], "deepseek");
    assert_eq!(json_val["envVar"], "DEEPSEEK_API_KEY");
    assert_eq!(json_val["secret"], true);
    assert_eq!(json_val["scope"], "global");
    assert!(json_val["docsUrl"].is_string());
    assert!(json_val["notes"].is_string());
}
