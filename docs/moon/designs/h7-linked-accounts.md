# H7 — Per-User Linked External Accounts: Design

> **Issue:** [#286](https://github.com/ACFHarbinger/Coding-Assistants/issues/286)
> · **Roadmap:** [`multi_human.md`](../roadmaps/multi_human.md) H7
> · **Status:** 🔬 Research / design-only · **Blocked on H2** (identity
> namespacing). No implementer assigned this round.
>
> **Purpose of this document:** Record the target architecture and provisional
> single-user approach so that (a) #284's account-connection surface can ship
> now under the `local` owner key, and (b) when H2 lands the migration plan is
> already written and agreed.

---

## 1. What this feature is

A user can *link* an external provider account (ChatGPT / Claude / Google /
DeepSeek / …) to their Hub identity. Linking associates a long-lived credential
reference with the user so that every provider call and quota read resolves
*that user's* token rather than a global env var.

The connection method is one of:

| `connection_kind` | Description |
|---|---|
| `oauth_device` | Hub walks the OAuth 2.0 device-authorization grant; the access/refresh tokens land in the P12 vault. |
| `vendor_cli_login` | The user already authenticated with the vendor's own CLI (e.g. `claude auth login`, `gemini auth`). Hub records the fact but does not own the token; the harness reads it from the CLI's own credential store. |

---

## 2. The `linked_account` record

```
linked_account {
    owner          TEXT NOT NULL,   -- H2 namespaced identity, e.g. "harbinger"
                                    -- (provisional: "local" until H2 lands)
    provider       TEXT NOT NULL,   -- "openai" | "anthropic" | "google" |
                                    -- "deepseek" | "meta" | …
    external_label TEXT,            -- display name / email from the provider
    connection_kind TEXT NOT NULL,  -- "oauth_device" | "vendor_cli_login"
    linked_at      INTEGER NOT NULL,-- Unix epoch seconds (UTC)
    token_ref      TEXT,            -- vault key in the P12 secret backend,
                                    -- NULL when connection_kind = vendor_cli_login
    PRIMARY KEY (owner, provider)
}
```

### What lives where

| Data | Location | Rationale |
|---|---|---|
| `linked_account` record | `hub.db` | Presence metadata — never a secret value. |
| Actual token / refresh token | P12 vault (`hub::secret`) | `SecretBackend` contract: value never in DB, IPC, logs, or diagnostics. |
| Token reference key (`token_ref`) | `hub.db` column | Only a key name, not the secret. Naming convention: `linked.{owner}.{provider}`. |

### Crossing IPC

Only these facts cross IPC (no value, ever):

```rust
pub struct LinkedAccountStatus {
    pub provider: String,
    pub external_label: Option<String>,
    pub connection_kind: String,
    pub is_linked: bool,
    pub linked_at: Option<i64>,  // epoch seconds
    pub source: String,          // "vault" | "vendor_cli"
}
```

---

## 3. Operations

| Operation | Rust surface (future) | Effect |
|---|---|---|
| **Link (OAuth device)** | `hub_link_account_oauth(provider)` | Runs device-auth flow; stores token in vault under `linked.{owner}.{provider}`; writes `linked_account` row. |
| **Link (vendor CLI)** | `hub_link_account_cli(provider, label)` | Writes row with `connection_kind = vendor_cli_login`, `token_ref = NULL`. |
| **Status** | `hub_list_linked_accounts()` | Returns `Vec<LinkedAccountStatus>` — no values. |
| **Unlink** | `hub_unlink_account(provider)` | Atomically deletes vault entry (if any) and `linked_account` row. |

Unlink is a two-phase operation:

1. Delete from P12 vault: `hub::secret::clear_secret(token_ref)`.
2. Delete `linked_account` row from `hub.db`.

If step 1 succeeds and step 2 fails, the orphaned vault entry is safe (the row
no longer references it); a subsequent re-link overwrites it. If step 1 fails,
abort — do not delete the row.

---

## 4. Provider resolution at call time

When a provider/quota adapter needs a credential it calls the resolver:

```
resolve_for_caller(owner, provider)
  1. Look up linked_account row for (owner, provider).
  2. If row exists and token_ref is set → hub::secret::resolve(token_ref).
  3. If row exists and token_ref is NULL → signal "vendor CLI owns auth".
  4. If no row → fall back to hub::secret::resolve(env_var_name)
     (existing behaviour — vault then std::env::var).
```

A single-user Hub following the provisional `local` key behaves identically to
the current env-var resolver because there is only one user and the fallback
chain is the same. No regression for existing deployments.

---

## 5. Provisional single-user storage (pre-H2)

Until H2 (identity namespacing) lands, `owner` is the hardcoded string
`"local"`. This means:

- `PRIMARY KEY ("local", provider)` — one linked account per provider,
  globally.
- The P12 vault key is `linked.local.{provider}`.
- The #284 Settings UI account-connection surface ships against this schema.

**The H2 migration contract:** H2 seeds today's roster as `harbinger/*` (or
whatever single-user identity name is chosen). At that point a one-time
migration rewrites `owner = "local"` → `owner = "<chosen_identity>"` on every
`linked_account` row, and renames every `linked.local.*` vault key to
`linked.<chosen_identity>.*`.

### Migration sketch (for H2 implementer)

```sql
-- Part of the H2 schema migration
UPDATE linked_account
SET owner = :new_owner
WHERE owner = 'local';
```

```rust
// Part of the H2 vault migration utility
for provider in list_providers_with_local_key() {
    let old_key = format!("linked.local.{provider}");
    let new_key = format!("linked.{new_owner}.{provider}");
    if let Ok(Some(secret)) = hub::secret::get_raw(&old_key) {
        hub::secret::set_secret(&new_key, secret.expose())?;
        hub::secret::clear_secret(&old_key)?;
    }
}
```

This migration is safe to run on a Hub with zero linked accounts (no-op) and is
idempotent: if `owner` is already the namespaced identity, the `WHERE` clause
matches nothing.

---

## 6. Multi-user behaviour (post-H2)

Once H2 lands:

- `owner` is the H2 namespaced identity (e.g. `"harbinger"`, `"alice"`).
- Two users can link the same provider to different accounts — their rows have
  different `owner` values and different vault keys, so they never collide.
- A provider call resolves the *caller's* linked account (looked up by the
  caller's H2 identity), not a global one.
- A Hub with one user behaves exactly as today (same query, one row).

The linked-account table is **never global** — there is no `owner = NULL` /
org-wide credential row. Shared credentials (e.g. a team API key) are an
explicit future feature (H8 or later) with its own approval gate.

---

## 7. SQLite schema placement

The `linked_account` table belongs in `hub.db` alongside `agents`, `messages`,
and `settings`. It is provisioned by the `HubStore` migration sequence (see
`crates/hub/src/store/`). This document does **not** add the migration — that
is implementation work gated on H2 timeline.

The table creation SQL when it is implemented:

```sql
CREATE TABLE IF NOT EXISTS linked_account (
    owner           TEXT    NOT NULL,
    provider        TEXT    NOT NULL,
    external_label  TEXT,
    connection_kind TEXT    NOT NULL CHECK (connection_kind IN ('oauth_device', 'vendor_cli_login')),
    linked_at       INTEGER NOT NULL,
    token_ref       TEXT,
    PRIMARY KEY (owner, provider)
) STRICT;
```

`STRICT` mode (SQLite ≥ 3.37, 2021-11-27) is used elsewhere in `hub.db`
where it is available; use `WITHOUT ROWID` only if benchmarking shows a benefit
(the table is expected to have O(10) rows per user).

---

## 8. Linkable providers — initial list

The account-connection surface in #284 should enumerate these providers. The
list is advisory; the implementation does not hard-code it (a `provider` is any
non-empty string ≤ 64 chars, validated by the Rust layer).

| Provider slug | Display name | Supported `connection_kind` | Notes |
|---|---|---|---|
| `openai` | OpenAI / ChatGPT | `oauth_device` | Device-auth endpoint available. |
| `anthropic` | Claude | `vendor_cli_login` | `claude auth login` manages the session; Hub records the fact. |
| `google` | Google / Gemini | `oauth_device` or `vendor_cli_login` | `agy auth` / `gemini auth` paths both viable. |
| `deepseek` | DeepSeek | `oauth_device` | API key flow; device-auth TBD — spike needed. |
| `meta` | Meta / Muse | `oauth_device` | Meta Model API key flow. |
| `xai` | xAI / Grok | `oauth_device` | API key flow. |
| `mistral` | Mistral | `oauth_device` | API key flow. |
| `perplexity` | Perplexity | `oauth_device` or `vendor_cli_login` | `pwm login` manages its own session (#277). |

**Spike required** before implementing any OAuth flow: confirm that the
provider exposes a device-authorization endpoint (RFC 8628) or a comparable
API-key provisioning API. A "uses vendor CLI native login" row is always safe
to record without a spike.

---

## 9. Security invariants (must survive into implementation)

1. **No credential value crosses IPC, ever.** `LinkedAccountStatus` carries
   only presence, label, kind, and timestamp. The pattern from `SecretStatus`
   (#282) applies here identically.
2. **`token_ref` is a vault key, not a secret.** Logging `token_ref` is safe;
   logging any value retrieved by it is not.
3. **Unlink is atomic at the application level.** Vault delete before row
   delete; abort on vault failure. An orphaned vault key (row gone, key
   present) is safe — it is unreachable and will be cleaned on re-link.
4. **No shared / org-level credential rows.** Every row has a non-null `owner`.
5. **`vendor_cli_login` rows have `token_ref = NULL`.** The Hub never reads,
   stores, or touches the vendor CLI's own token.
6. **Debug impls for any struct holding `token_ref` must redact or omit the
   field** (treat it as a key name that happens to point at a secret).

---

## 10. Open questions for H2 implementer

| # | Question | Why it matters |
|---|---|---|
| Q1 | What is the canonical string form of the H2 namespaced identity? (e.g. `"harbinger"`, `"harbinger/human"`, `"local/harbinger"`) | Determines the `owner` column value and the vault key prefix format. |
| Q2 | Is the identity stored in `hub.db` or resolved from a separate identity table? | Determines whether `linked_account.owner` is a FK or a denormalized string. |
| Q3 | Does H2 introduce a `users` / `identities` table? | If yes, add `FOREIGN KEY (owner) REFERENCES identities(id)` to `linked_account`. |
| Q4 | Is the migration a Rust code migration or a pure SQL migration? | Affects how the `linked.local.*` vault key rename is coordinated with the row update (needs to be transactional at the application level, not just the SQL level). |
| Q5 | What happens if a `vendor_cli_login` link exists for a provider whose CLI no longer has a session? | Policy question: surface as `is_linked = true, source = "vendor_cli"` with a "session may have expired" note, or attempt a liveness check (potentially slow)? |

---

## 11. What #284 may ship today (single-user)

The #284 Settings UI account-connection surface can ship against the provisional
`local` owner key **without any new Rust code in this round**. The UI panel
shows:

- A static list of linkable providers (see §8).
- Per row: connected / not-connected status, external account label if
  connected, **Connect** and **Disconnect** buttons.

The **backend Tauri commands** that #284 calls are future work (blocked on this
design being approved and on H2 timeline). For this round #284 may render the
panel in a read-only or mock state, with a clear `TODO` comment referencing
this design doc and issue #286.

If Harbinger wants to unblock #284's Connect/Disconnect buttons sooner:

1. Add the `linked_account` table to `hub.db` migrations now (with `owner =
   "local"` as the sentinel).
2. Implement the three Tauri commands (`hub_link_account_cli`,
   `hub_list_linked_accounts`, `hub_unlink_account`) scoped to `owner =
   "local"`.
3. Note in code and commit message: *"provisional local key; H2 migration
   rewrites owner"* (per issue #286 and #284's spec).

This is entirely safe — the H2 migration is a one-time UPDATE + vault rename,
not a destructive change.

---

## 12. Relationship to other issues

| Issue | Relationship |
|---|---|
| [#282](https://github.com/ACFHarbinger/Coding-Assistants/issues/282) | P12 vault. `token_ref` keys live here. H7 **requires** #282. |
| [#283](https://github.com/ACFHarbinger/Coding-Assistants/issues/283) | Credential set/clear Tauri commands. H7's `hub_link_account_oauth` reuses the same vault write path. |
| [#284](https://github.com/ACFHarbinger/Coding-Assistants/issues/284) | Settings UI that renders the account-connection surface backed by this design. |
| [#285](https://github.com/ACFHarbinger/Coding-Assistants/issues/285) | Field catalog. Provider `connection_kind` is orthogonal to per-field secrets; they share the vault but have different key namespaces (`linked.*` vs. env-var names). |
| [#287](https://github.com/ACFHarbinger/Coding-Assistants/issues/287) | Migrates quota/dispatch adapters onto the #282 resolver. H7's per-user resolution is a superset: resolve the caller's linked account first, then fall through to the #282 env resolver. |
| H2 (TBD issue) | Identity namespacing. Unblocks full multi-user implementation of H7. |

---

*Authored 2026-09-08 for the #279 credentials batch. To be folded into the H2
design when multi-human work is scheduled.*
