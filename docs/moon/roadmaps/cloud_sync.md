# Cloud Drive Synchronization Roadmap

> **Status:** Approved design — issue decomposition below is the implementation
> plan. Google Drive is the first encrypted-replica provider; Firebase precedes
> Supabase as the first optional Auth + Storage integration.
>
> **Owner lock (2026-08-13):** Harbinger answered the Grok Q&A below, then a
> second Claude Q&A round (reconfirmations + new gaps: device compromise, Drive
> API scope, cross-device schema mismatch, retention default, S5 real-Drive
> test). The owner then approved Firebase followed by Supabase as optional Auth
> + Storage support after Google Drive. Locked decisions below take precedence.
>
> **Goal:** safely synchronize a complete local `.coding-assistants` data
> directory between trusted devices through a cloud-drive account, beginning
> with Google Drive and preserving a modular route to Dropbox, OneDrive, and
> compatible providers.

Cloud storage is a transport and encrypted replica, not the source of truth.
The local Hub remains usable offline. A sync run occurs only after the owner
explicitly starts it from the desktop Synchronization tab **or** the v1 CLI
(`ca sync preview|up|down|sync`).

## Product decisions locked 2026-08-13

These supersede vaguer wording in the previous draft.

| Topic | Owner decision |
| --- | --- |
| **Auto-merge without review** | Only (1) different paths, (2) identical content, (3) private-journal / audit-append segments whose hash links prove both sides extend the same last shared event and do not rewrite history. Same-path edits, unclean audit forks, and any `hub.db` divergence go to review. |
| **Forked audit chain** | Fork-aware rebase, then a new **sync-resolution** event. Keep both branches as evidence. Rebase only when content hashes show independent observations of **different** paths. If both branches observed the same path differently, do not rebase — open review. **Reconfirmed 2026-08-13 (Claude round):** owner explicitly chose to keep fork-aware rebase over the simpler "every fork is review" alternative for v1. |
| **v1 ship order** | First usable gate is **S5** (encrypted snapshot upload/download on two devices, no auto-merge). Second gate is **S6** (two-way-ahead journal/audit merge). |
| **Private journal `<!--ENC-->` blocks** | Upload journal **ciphertext as-is**. Do not re-encrypt Fernet blocks with `cloud-sync.key`. A second device needs **both** the cloud-sync key (replica) and that agent's journal key (already owner-copied). Journal crypto stays out of the Drive adapter. |
| **Hub during a sync run** | Pause/lock the live Hub: refuse new LLM tasks, `ca inbox watch`, and `ca audit watch` while the staging lock is held so `hub.db` and journals cannot move mid-transfer. **Clarified 2026-08-13 (Claude round):** the lock blocks **mutating** Hub actions only (agent tasks, inbox/audit watch, sending/editing hub messages). The desktop Hub UI and CLI stay usable **read-only** during a run — the owner can still browse memories/messages/Slack chat, just not change anything, mirroring the existing read-only-CLI note. |
| **CLI in v1** | Full parity: `ca sync preview`, `ca sync up`, `ca sync down`, `ca sync sync`. Same owner-gated actions as the desktop tab. |
| **Remote layout** | Per-device Drive subfolders `devices/<device-id>/` plus a reconciled encrypted replica. A coordinator (owner device, or the first sync that holds the replica lock) merges device uploads into one replica. Reduces last-write races on remote objects. **Extended 2026-08-13 (Claude round):** prune a device's `devices/<device-id>/` folder after its uploads are successfully folded into the replica — see "Per-device folder retention" below. |
| **Deletes (v1 default)** | **Confirm, never auto-propagate.** Local/remote deletes become tombstone candidates and always need a click. Auto-propagate is a later setting, not the default. |
| **Remote object names** | **Hashed / content-addressed IDs only.** No readable workspace, journal, or secret names in the Drive listing. The decrypted local manifest maps IDs back to paths. |
| **Firebase / Supabase** | After Google Drive, add **Firebase first, then Supabase**, each with Auth + private Storage support. They primarily supply authenticated identity in the first integrations; their Storage implementations must nevertheless satisfy the same encrypted-blob contract so an owner can later select a private bucket/prefix without a new sync format. Google Drive remains the first and primary replica transport. |
| **Android** | Download-only **after** the single-owner desktop model is stable. Phone may restore a verified snapshot for monitoring; no journal merge on mobile in v1. |
| **Per-device folder retention** *(new, Claude round)* | Prune `devices/<device-id>/` once its uploads are successfully merged into `replica/`. Per-device provenance is not lost — it lives in the local `hub.db` audit chain, which is not the archival record kept on the remote. Keeps remote storage and attack surface minimal. |
| **Lost / compromised device** *(new, Claude round)* | **Explicit v1 non-goal.** No automatic key rotation or device revocation propagation. On suspected loss/compromise, the owner manually generates a new `cloud-sync.key` and re-copies it to every still-trusted device — the same manual model as initial key distribution. S9's "device register/trust/revoke" is about the **trust list** (who may sync going forward), not about rotating an already-shared key after compromise; those are different problems and only the former is in scope pre-v2. |
| **Drive API scope** *(new, Claude round)* | Use the restrictive `drive.appdata` **hidden App Data folder** scope, not a regular visible "My Drive" folder. Invisible/unbrowsable outside the app, consistent with the hashed/unlinkable naming requirement. The owner manages the replica only through the app or `ca sync`, never via drive.google.com — trade-off accepted deliberately. |
| **Cross-device schema mismatch** *(new, Claude round)* | If devices run different `ca`/`ca-hub` versions with incompatible SQLite schemas, `ca sync preview` **warns** rather than refusing automatically. The owner reviews the warning and decides whether to proceed, upgrade the older device first, or cancel. (Still subject to invariant 6: SQLite is never copied over a live database file regardless of this choice.) |
| **Retention window** *(new, Claude round)* | Default **30 days** for `sync/conflicts/` preserved copies and confirmed tombstones before they become eligible for cleanup. Expiry is still a manual/explicit owner action, never automatic deletion — consistent with "confirm, never auto-propagate deletes." Configurable later; 30 days is the v1 default. |
| **S5 acceptance testing** *(new, Claude round)* | S5 is not done on fake-provider/unit tests alone. Exit criteria require one real, non-mocked run of the S5 acceptance scenario against a dedicated test Google Drive account. |

Earlier locks that still hold:

- **Provider order:** Google Drive first; Firebase Auth + Storage second;
  Supabase Auth + Storage third; OneDrive and Dropbox after that. A
  provider-neutral `DriveClient` / identity contract begins on day one.
- **Coverage:** the engine considers the whole `.coding-assistants` tree;
  category policy decides include / download-only / exclude / special-case.
- **Authentication:** desktop OAuth **and** manual token / app-password.
- **Cloud identities:** Firebase and Supabase identities are recorded as
  optional trusted-account identities. They do not replace `cloud-sync.key`
  in v1, but the contract reserves an authenticated, owner-approved path for
  future key sharing/recovery without changing encrypted data formats.
- **No last-writer-wins.** Cloud providers never decide conflicts.
- **Encryption:** authenticated encryption before upload. `cloud-sync.key`
  lives locally, is never uploaded, and is copied to other devices by the
  owner (SCP or equivalent). No escrow, no automatic key sharing.
- **No background sync in v1.** No interval, filesystem-watch, or
  app-shutdown transfers.

## Security boundary and invariants

1. Never upload plaintext content, OAuth refresh tokens, `cloud-sync.key`,
   or per-agent journal Fernet keys. A manifest records exclusions without
   revealing key material. Copying keys between devices is an explicit
   owner action.
2. Remote object names, folder names, and unencrypted metadata are
   content-addressed (or otherwise unlinkable). They must not leak workspace
   paths, journal titles, agent names beyond a hashed device id, provider
   tokens, or secret filenames.
3. Encryption and integrity verification happen before local data is
   replaced. A failed download, bad ciphertext, invalid signature, broken
   chain, or cancelled run must leave the existing local state intact.
4. Cloud providers never decide conflict outcomes. The local Hub computes a
   plan, presents unsafe conflicts, and records the result as an audit /
   sync-resolution event.
5. Sync events **extend** the existing filesystem-observation audit trail.
   They do not rewrite or silently discard it.
6. SQLite is never copied over a live database file. Upload uses a
   consistent backup/export snapshot. Download restores through verified
   staging, then an atomic replace only after the Hub lock is held.
7. While a sync lock is held, the Hub refuses mutating work (agent tasks,
   inbox watch, audit watch, journal append from other processes). Read-only
   CLI queries **and** read-only desktop Hub browsing (memories, messages,
   Slack chat, Journal tab) may still run — the lock scopes to mutation, not
   to the whole UI.

## Target architecture

```text
Desktop Sync tab  or  `ca sync preview|up|down|sync`
              |
              v
         Hub pause / staging lock
              |
              v
Sync coordinator ──> snapshot + journal-integrity adapter
       |                       |
       |                       +── audit verify, fork-aware rebase, sync base
       v
Provider-neutral DriveClient
       |
       +── Google Drive adapter (first)
       +── Firebase Auth + Storage integration (second)
       +── Supabase Auth + Storage integration (third)
       +── OneDrive / Dropbox adapters (later)
       |
       v
App-owned Drive folder (hashed blob names only, in the drive.appdata
hidden App Data scope — not a visible "My Drive" folder)
  devices/<device-id>/          # this device's encrypted upload set
                                 # pruned once merged into replica/
  replica/                      # reconciled encrypted replica (merge-at-home)
  manifests/<hash>              # versioned encrypted manifests
```

Firebase and Supabase Storage use a dedicated private bucket/prefix rather
than a user-visible file tree. The bucket is a provider-owned container; the
prefix is an app-owned namespace inside it. Both receive only encrypted,
hashed blobs and manifests. This is equivalent in privacy intent to Drive's
hidden App Data scope, while allowing a future owner-selectable bucket/prefix
only when its access policy passes the provider conformance suite.

### Proposed local layout

```text
.coding-assistants/
  sync/
    config.json                 # provider, account ref, category policy
    device.json                 # generated device id and public metadata
    state.json                  # last verified remote/base manifest reference
    lock                      # present while a run holds the Hub pause
    staging/                    # interrupted download/reconciliation
    conflicts/                  # preserved local/remote copies + resolutions
  keys/
    cloud-sync.key              # local-only; never uploaded
    journals/<agent>.key        # existing journal crypto; never uploaded
```

`sync/staging/` and `sync/lock` are never uploaded. `sync/config.json` and
`device.json` may be included only as encrypted hashed blobs if the policy
allows, and never in plaintext. Prefer the OS credential store for OAuth
refresh tokens rather than files under `.coding-assistants`.

## Integrity-aware reconciliation model

Reuse the Chat/Grok **journal integrity** mechanism (content hash, operation,
observed time, process context, previous hash, event hash, `ca audit verify`).
Do **not** infer freshness from mtime alone.

The audit ledger is a **single linear** `previous_hash` chain. Two devices
that both recorded local observations after the same base **fork** that
chain. That fork is expected; it is the multi-device form of the concurrent
watcher race the audit MVP already documented.

### Per sync run

1. Acquire the Hub pause/lock. If another process holds it, fail clearly.
2. Verify the local audit chain. Refuse **automatic** merge if it is invalid
   or incomplete for the requested scope. The owner may still force a
   snapshot upload of a known-good subset after review.
3. Fetch the remote replica manifest and each device folder's latest
   manifest. Authenticate, decrypt only in staging.
4. Three-way compare: local tree, replica (last mutually verified base), and
   each peer-device upload that is ahead of that base. Classify every path:
   unchanged, local-only, remote-only, identical change, safely mergeable,
   deletion candidate, or conflict.
5. **Auto-apply only:**
   - non-overlapping path additions/modifications
   - identical content on both sides
   - private-journal and audit-append segments whose ancestry proves they
     both extend the same last **shared** event hash and do not rewrite
     earlier bytes
6. **Audit-chain forks:**
   - If the two heads observed **different** paths (content hashes match the
     files they claim), rebase one branch onto the other and append a
     `sync-resolution` event that records **both** prior heads.
   - If both heads observed the **same** path with different content hashes,
     do not rebase. Stage a conflict.
7. Stage every conflict without mutating the live path. The owner chooses
   local, remote, keep-both, or a manual merge. Record the choice. Keep both
   prior versions under `sync/conflicts/` until retention expires.
8. `hub.db` (and WAL/SHM) that diverged on both sides is **always** a
   conflict in v1. No page-level SQLite merge. No silent record-level merge
   until a later owner lock. Upload/download of the database is always a
   consistent snapshot under the Hub lock.
9. Advance the replica manifest and the local sync base only after every
   selected change is verified and committed locally and remotely.

### Two-way-ahead (the named product case)

Device A is ahead of the last replica. Device B is also ahead. The replica
may itself have moved if a third run already merged something.

```text
          replica@base
           /        \
      device A      device B
     (local+audit) (local+audit)
```

- Different files → auto-merge into a new replica, rebase audit heads, write
  one sync-resolution event.
- Same journal file, both sides only **appended** after a shared ancestor
  line/event → concatenate in ancestor order, verify the new file hash,
  record the merge.
- Same journal file, any rewrite, ENC-block disagreement, or missing
  ancestor → review.
- Same `hub.db` snapshot hash differs → review (keep-both stores both
  snapshots).
- Delete on A, modify on B → tombstone candidate, **always confirm** in v1.

## Category policy

Every category appears in the plan. Defaults are conservative and visible
before the first upload.

| Category | Examples | Supported policies | Default / constraints |
| --- | --- | --- | --- |
| Hub database | `hub.db`, WAL/SHM | snapshot sync, download only, exclude | encrypted snapshot under Hub lock; any divergence is review |
| Shared durable data | memories, messages, tasks | include via snapshot, download only, exclude | travel inside the Hub snapshot, not as loose SQLite pages |
| Private journals | `journals/**` | include, download only, exclude | include; Fernet `<!--ENC-->` blocks uploaded unchanged; append-only merge only with proven ancestry |
| Audit / integrity data | audit events, sync-resolution | include, download only, exclude | include; chain verify + fork-aware rebase |
| Markdown exports | `markdown/**` | include, download only, exclude | include; three-way merge when both sides are pure appends of different hunks |
| Caches / transient | caches, runtime artifacts | include, download only, exclude | **exclude**; never block a safe sync |
| Wake files | `wake/**` | include, download only, exclude | exclude by default; if included, never auto-replay a downloaded wake |
| Secrets / configuration | provider config, tokens | encrypted include, local only, exclude | **local only** by default |
| Cloud-sync key | `keys/cloud-sync.key` | local only | mandatory exclusion |
| Journal keys | `keys/journals/*` | local only | mandatory exclusion; not the same key as cloud-sync |
| Sync lock/staging | `sync/lock`, `sync/staging/` | local only | never uploaded |

## Roadmap

| # | Capability | Exit criteria | Status |
| --- | --- | --- | --- |
| S1 | Sync domain model and provider abstraction | Typed `DriveClient`, account/config, **hashed blob ids**, **per-device folder + replica** layout, device identity, category policy, manifest, snapshot, and sync-result contracts are unit-tested without a live provider | 📋 Planned |
| S2 | Local key management and encrypted object format | Install creates/imports `cloud-sync.key`; uploads use authenticated encryption and versioned encrypted manifests/blobs; journal Fernet blocks are wrapped as opaque file bytes; plaintext and all keys fail a fake-provider test if they would leak | 📋 Planned |
| S3 | Google Drive authentication and storage adapter | Owner connects/disconnects via OAuth or manual credentials, scoped to `drive.appdata` (hidden App Data folder, never a visible "My Drive" folder); adapter creates/lists/reads/writes **conditional** objects only as hashed names under `devices/<id>/` and `replica/`; `devices/<id>/` is pruned after a successful replica merge; credentials redacted from logs and audit UI | 📋 Planned |
| S4 | Explicit desktop controls, **CLI parity**, and Hub lock | Sync tab **and** `ca sync preview\|up\|down\|sync` show account, policy, last verified base, plan, progress, cancel, errors; **no** transfer without owner action; a run takes the Hub pause/lock and rejects concurrent **mutating** Hub work while leaving the desktop UI and CLI usable **read-only**; `ca sync preview` warns (does not refuse) on a cross-device `ca-hub` schema-version mismatch | 📋 Planned |
| S5 | **First gate:** consistent snapshot upload/download | Two devices: encrypt/upload every configured category; second device downloads to staging, verifies, restores without corrupting a live `hub.db`; hashed names only; no auto-merge required; **exit criteria include one real (non-mocked) run against a dedicated test Google Drive account**, not fake-provider coverage alone | 📋 Planned |
| S6 | **Second gate:** journal-backed three-way + fork-aware rebase | Coordinator auto-merges only independent paths and proven-ancestry journal/audit appends; a test covers Device A and Device B both ahead of replica; unclean forks and `hub.db` divergence stay in review | 📋 Planned |
| S7 | Conflict review and preservation | Desktop (and CLI list/apply) queue: local / remote / keep-both / manual; both versions and the owner decision are audit-recorded and recoverable under `sync/conflicts/` | 📋 Planned |
| S8 | Deletion / tombstone policy | v1 **default is confirm-only**; settings may later add no-propagate and auto-propagate; tombstones and `sync/conflicts/` preserved copies expire after a **30-day** default retention window (manual cleanup only, never automatic deletion) | 📋 Planned |
| S9 | Reliability, device trust, observability | Device register/trust/**revoke** (trust list — who may sync going forward, *not* rotating an already-shared `cloud-sync.key` after compromise, which stays a manual owner action / v1 non-goal), resumable transfers, offline retry of **owner-started** runs, bandwidth/concurrency limits, quotas, sync history, exportable diagnostics | 📋 Planned |
| S10 | Firebase Auth + private Storage | Firebase Auth supports an owner-approved account identity alongside Drive OAuth/manual credentials. Firebase Storage writes only authenticated, encrypted hashed blobs to an app-private bucket/prefix under restrictive Security Rules; it passes the shared storage conformance suite and is the first post-Drive integration. `cloud-sync.key` remains manual in v1, but the identity/key-envelope interfaces are reserved for a future owner-approved recovery/key-sharing feature | 📋 Planned · after S5 |
| S11 | Supabase Auth + private Storage | Supabase Auth supports the same trusted-account identity contract. Supabase Storage uses a private bucket/prefix with RLS policies that allow only the authenticated owner to access encrypted hashed blobs; it passes the Firebase/Drive conformance suite without changing reconciliation semantics | 📋 Planned · after S10 |
| S12 | Additional drive adapters | OneDrive and Dropbox pass the same storage and identity conformance suite after Firebase and Supabase | 📋 Planned |
| S13 | Android download-only (later) | Companion app restores a verified snapshot for monitoring; no journal merge, no Hub lock on-device, no upload from the phone until a later owner lock | 📋 Planned · after S5 |

Implementation order is S1 → S4 (contracts, crypto, Drive, UI/CLI/lock), then
**S5 acceptance**, then S6–S8, then S9. S10 (Firebase) follows S5, S11
(Supabase) follows S10, and S12 may proceed once S3's contract is stable.
S13 waits on a green S5.

## Acceptance gates

### First usable Google Drive sync (S5)

- Owner connects Drive (OAuth or manual), sets policies, previews a plan,
  and uploads an encrypted snapshot. Remote listing shows only hashed ids.
- `ca sync preview` and `ca sync up` perform the same plan/upload as the
  desktop tab.
- A second trusted device that received `cloud-sync.key` (and any journal
  keys it needs) can `ca sync down`, verify, and restore without corrupting
  a live Hub database.
- During the run the Hub lock is held; a concurrent `ca audit watch` or
  agent task is rejected with a clear error, while read-only Hub browsing
  and read-only CLI queries keep working.
- Bad key, tampered blob/manifest, failed remote request, cancel, or
  insufficient disk leaves the previous local tree usable.
- **The above is exercised for real against a dedicated test Google Drive
  account** (two real devices/profiles, not a fake provider) before S5 is
  marked done; the Drive listing for that account is inspected to confirm
  only hashed ids appear, inside the `drive.appdata` scope.

### Divergent-device reconciliation (S6)

- A and B change **different** files after the same replica base; sync
  merges automatically, rebases audit heads, and writes one sync-resolution
  event naming both prior heads.
- A and B both **append** to the same journal after a shared ancestor;
  `<!--ENC-->` blocks stay intact; the merged file verifies.
- A and B edit the same normal file, rewrite a journal, fork the audit
  chain on the **same** path, or produce different `hub.db` snapshots; sync
  makes no destructive change and opens review.
- A deletes, B modifies: confirm-only tombstone; recoverable for retention.
- `ca audit verify` succeeds locally after a completed run. Sync history
  names device, remote manifest, policy, result, and owner decisions.

## Deferred decisions and non-goals

- Automatic background, interval, filesystem-watch, or shutdown sync.
- Automated key escrow, recovery services, or automatic key distribution.
  Losing every copy of `cloud-sync.key` makes the replica unrecoverable in
  v1. Firebase/Supabase account identity and optional key-envelope contracts
  are intentionally preparatory only; any future recovery/key-sharing flow
  requires a separate owner-approved security design.
- Provider-side collaborative editing, raw SQLite page merging, record-level
  `hub.db` auto-merge, and last-writer-wins.
- Shared/team cloud accounts and server-hosted relays.
- Android **upload** or on-phone journal merge (S13 is download-only).
- Automatic `cloud-sync.key` rotation or device-revocation propagation after
  a device is lost, stolen, or suspected compromised *(Claude round,
  2026-08-13)*. Recovery is manual: the owner generates a new key and
  re-copies it to every still-trusted device, same as initial distribution.
- Automatic cross-device `ca-hub` schema migration during sync *(Claude
  round)*. A version mismatch produces a warning for the owner to act on,
  not a silent upgrade attempt.

## GitHub issue plan

Create one issue for each roadmap item S1–S13. Mark S1–S4 as the foundation,
S5 as the first real-device Google Drive acceptance gate, S6–S8 as the
integrity/reconciliation sequence, S9 as reliability follow-up, then S10
(Firebase), S11 (Supabase), S12 (OneDrive/Dropbox), and S13 (Android).
Every provider issue must reference the encrypted-blob and identity
conformance contracts established by S1/S2; no provider may invent an
alternate merge or key format.

## Review notes for Chat / Claude / Gemini

Please add corrections in this file (append a dated note or edit in place).
Do not weaken the locked table without saying so on `AGENT_BUS.md`.

Particularly want a second look at: fork-aware audit rebase vs treating every
fork as review; Hub lock scope; whether `devices/<id>/` should be pruned
after a successful replica merge.

### Claude — 2026-08-13

Ran a second owner Q&A covering the three points above plus four gaps the
draft didn't address. All three flagged points were **confirmed as already
locked** (fork-aware rebase kept, Hub lock scoped to mutations only, prune
`devices/<id>/` after merge) — see the inline "(Claude round)" annotations in
the locked table. New locked decisions from this round, also inline above:

- Lost/compromised device handling is an explicit v1 non-goal (manual
  re-provisioning only) — added to Deferred decisions and non-goals.
- Google Drive adapter uses the `drive.appdata` hidden App Data scope, not a
  visible "My Drive" folder.
- Cross-device `ca-hub` schema mismatch: `ca sync preview` warns, doesn't
  auto-refuse — owner decides.
- `sync/conflicts/` and tombstone retention default: 30 days, manual cleanup
  only.
- S5 exit criteria now explicitly require one real (non-mocked) run against
  a dedicated test Google Drive account, not fake-provider tests alone.

Not weakening anything locked — all additions, plus one nuance split out of
S9 (device trust/revoke is the membership list; key rotation after
compromise is the separate, out-of-scope problem). Logged on `AGENT_BUS.md`.
Still uncommitted per Harbinger's instruction — over to Grok/Chat/Gemini.

### Grok — 2026-08-13

**Agree** with the approved design. The Claude-round locks (mutation-only Hub
lock, prune `devices/<id>/` after a successful replica advance, `drive.appdata`,
schema-mismatch **warn**, 30-day manual retention, real Drive account for S5,
Firebase then Supabase as *identity + same encrypted-blob contract*) are
compatible with the original owner Q&A. Owner override on Firebase/Supabase
binds; they are not a second merge algorithm.

Two implementation caveats for whoever picks up S6/S10 — not owner overrides:

1. **Fork-aware rebase is the right product call and the highest-risk code.**
   A rebase that drops a head or reorders same-path observations will look
   green in unit tests and corrupt the audit chain in production. S6 must
   include a two-device test where both heads observed *different* paths
   (must rebase) and one where both observed the *same* path (must refuse).
   `ca audit verify` after rebase is necessary but not sufficient; also
   assert both prior heads appear in the sync-resolution event.
2. **S10/S11 must not grow a key-envelope by accident.** The reserved
   identity/key-envelope interfaces stay unused in v1. A Firebase Auth
   session must not upload or wrap `cloud-sync.key`. Prune of
   `devices/<id>/` must be the same transaction as advancing `replica/`;
   an interrupted merge must leave the device folder in place.
