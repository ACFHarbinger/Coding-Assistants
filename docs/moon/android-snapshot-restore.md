# S13 Android download-only snapshot restore

The companion restores a **verified** replica snapshot for monitoring.
It does not talk to Drive, Firebase, Supabase, OneDrive, or Dropbox, and
it does not mutate the desktop `DriveClient` protocol.

## Layout (app-private home)

Prefer `Context.getExternalFilesDir(null)`, else `filesDir`.

| Path | Role |
| --- | --- |
| `keys/cloud-sync.key` | 32 raw bytes, owner-copied. Never uploaded. |
| `sync/inbox/<64-hex>` | Owner-copied CAS1 objects (hashed BlobIds only). |
| `sync/staging/restore/` | Verified plaintext. The only write target for replica files. |
| `sync/last-verified.json` | Schema + manifest blob id after a successful restore. |

Non-hex inbox names are skipped. `sync/lock` is never created.

## CAS1

Same as desktop S2: magic `CAS1`, version `1`, kind byte (blob=0,
manifest=1, snapshot=2), 12-byte nonce, IETF ChaCha20-Poly1305, AAD =
first 6 header bytes. Wrong key, truncate, or tamper fails closed.

## Restore rules

- Exactly one Manifest in the inbox.
- Each entry decrypts, matches `kind` + SHA-256 hex + size.
- Relative paths only; `..`, absolute, `keys/`, and `sync/lock` are refused.
- `hub.db` may land in staging; the live home `hub.db` is never replaced.
- No journal merge, no rebase, no Hub lock, no upload.

## Owner copy (debug)

```
adb push cloud-sync.key \
  /sdcard/Android/data/com.codingassistants.remotelauncher/files/keys/cloud-sync.key
adb push <64-hex> \
  /sdcard/Android/data/com.codingassistants.remotelauncher/files/sync/inbox/<64-hex>
```

Then **Restore snapshot** on the connection or dashboard screen.
