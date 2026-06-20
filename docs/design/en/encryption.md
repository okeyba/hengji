# Hengji · App-level password & local encryption — implementation notes (as-built)

> **中文版本：[`../encryption.md`](../encryption.md)。**
>
> Status: **implemented and landed (branch `feat/local-encryption`, committed locally only, not yet released).** This document describes **what was actually built**, not a proposal. For the backstory of how the design evolved, see "Appendix · Design evolution" at the end and the companion docs:
> - [`spike-results.md`](../spike-results.md) — measured conclusions from the four technical spikes done before implementation.
> - [`soft-path-plan.md`](../soft-path-plan.md) — the phased implementation contract and landing record.
>
> Scope: **Windows desktop only (Tauri)**. The browser demo is pure in-memory (`InMemoryRepository`), has no disk file, and is **not involved in encryption**. The `packages/core` double-entry engine, business logic, and SQL shape were **untouched throughout**.

---

## 0. The one-liner (for non-technical readers)

- Your ledger is locked by a **random key (the DEK)**; that key is in turn held by **this computer's security chip (the TPM)** — it is only released when you are **on this computer AND type the right password**.
- **Copy the database file to another computer: it won't open.** The key is bound to this chip and can't be extracted, and there is no path to "derive the key from the password alone." This tier is real and stronger than pure-software encryption.
- **Deletion is something you press on purpose, not an "auto-destroy after N wrong tries."** Settings has a "Wipe all data" button: when encrypted it requires the correct current password; when plaintext, a second confirmation. Once run, it is **permanent and unrecoverable**.
- **The honest gap**: a PC's TPM is **not** an iPhone's Secure Enclave — it has no hardware ability to "count to 5 wrong tries for this one key and self-destruct it." Its guess resistance is a **global rate limit** (after a few wrong tries the chip temporarily refuses, and recovers after a while), governed by system/chip policy that our software cannot change. So the real moat is **"the encryption itself + a strong password + the chip's rate limiting,"** not any destroy action.
- **The cost (acknowledged and accepted)**: there is **no local backdoor**. Forgetting the password / clearing the TPM / swapping the motherboard or CPU (fTPM is bound to the CPU) / a failed OS reinstall = your data is **gone for good**. This is the price of this strength. **Keep your own backup** (Settings can export a plaintext backup, see §7).

---

## 1. Threat model and honest boundaries

**Threat model = "A: device-level privacy"**: defending against someone who picks up / borrows / shares this computer and snoops your books.

### What this genuinely gives you
- **Copying the file gives no offline brute force**: the DEK is random, sealed by a non-exportable chip key, and can't leave the chip; the chip enforces a **global anti-guessing rate limit (DA — Dictionary Attack lockout)** on every "unseal with password" attempt. This is the core tier that beats pure software.

### What it does NOT give / does NOT defend (stated plainly in both UI and this doc, no overclaiming)
- **A PC TPM ≠ an iPhone Secure Enclave**: no hardware-autonomous "destroy the key after N tries." This implementation **does not do** a software-layer "auto-destroy after N wrong tries" either (see §5).
- **TPM guess resistance is global**: it shares one DA counter set with BitLocker / Windows Hello and they interfere with each other; the threshold and recovery time are chip/system policy and the app can't change them. Wrong passwords may trigger a system-level lockout as collateral.
- **Measured on this machine: the fTPM's DA locks out quickly** (see §4, "DA preemption") — about 3 wrong tries and the chip temporarily refuses to use that key (returns `TPM_20_E_LOCKOUT`), recovering slowly over time. **The chip's rate limiting is the primary defense** — a good thing (brute force gets jammed), but it also means "guessing the password inside the app" is bounced by the chip itself almost immediately.
- **A physical attacker can "Clear TPM" to destroy the DB**: via UEFI or as an admin, Clearing the TPM removes the wrapping key = data permanently unsealable. **No password needed, bypasses every confirmation.** This is the cheapest destructive path for a physical adversary (destruction, not theft).
- **Bus sniffing / fault injection**: a discrete TPM's (dTPM) CPU↔chip bus, if not running an encrypted session, can in theory be sniffed for unsealed data by a physical attacker (demonstrated against BitLocker); firmware TPMs (fTPM) have public attacks like faulTPM (voltage injection) / TPM-Fail (timing). **This implementation, going through NCrypt PCP, cannot obtain an "encrypted + HMAC TPM session" primitive** (the spike proved NCrypt doesn't expose it), so this item is **not mitigated**; positioned as "residual fTPM sniffing risk on this machine is low — accepted, not worth dropping to raw TBS."
- **During the unlocked runtime**: the decrypted DEK lives in process memory; the OS may page it to swap / write `hiberfil.sys` on hibernate. Mitigated by full-disk encryption (BitLocker).
- **Pre-encryption historical plaintext / migration residue**: SSD TRIM / wear-leveling / CoW means "overwrite deletion" doesn't guarantee physical erasure.
- **Runtime memory forensics / malware / nation-state adversaries**: out of scope.

**Honest ceiling**: the positioning is "**hard for an ordinary person who got hold of this computer**," **not** "absolutely unrecoverable."

---

## 2. Opt-in and the status line

- **Encryption is opt-in, plaintext by default**: unchanged behavior, so a sole proprietor jotting entries isn't forced to add a gate.
- **Status line (always shown on the Settings "Security · local encryption" card)**:
  - `Unencrypted (plaintext)`
  - `Encrypted · security-chip protected (strong)` (the only "encrypted" form this round: scheme `tpm-pcp`)
  - corrupt envelope → still shows "Encrypted" but flags the abnormal state (see §3, `scheme=None`).
- **No-chip / weak software-only version: not implemented this round (deferred).** The current encryption path **requires a usable TPM**; if "Microsoft Platform Crypto Provider" can't be opened, setting a password fails with "chip unavailable" and **does not** silently degrade to weak encryption. The original design's "weaker software-only version + third status state" is left as a future enhancement.
- `core` / business logic / SQL: **untouched**.

---

## 3. Key architecture (random DEK + security-chip-bound wrapping)

Implemented in `apps/desktop/src-tauri/src/crypto.rs` (key layer) + `db.rs` (SQLCipher open).

- **DEK (data key)**: random 256-bit (`BCryptGenRandom`), it **actually encrypts the database**. It is not derived from the password — this is the root of "copying the file gives no offline brute force."
- **SQLCipher raw-key open**: `PRAGMA key = "x'<64-hex>'"` (raw key, skipping SQLCipher's own PBKDF2; **must be the first PRAGMA on open**), then `journal_mode=WAL` / `foreign_keys=ON` / `busy_timeout`, then migrations.
- **Security-chip wrapping**: NCrypt + the **Microsoft Platform Crypto Provider (PCP)** creates a **non-exportable** RSA-2048 key in the TPM; the password is that key's **usage authorization `PCP_USAGEAUTH`** (value = `SHA256(UTF-16LE(password))`). Wrapping algorithm is **OAEP-SHA256**. Unsealing the DEK requires ① being on this machine's chip ② the right password (spike #2 measured: a wrong password is rejected by the chip with `NTE_PERM` and counts against the global DA).
- **Two fixed slots (`heng-dek-wrap-a` / `-b`)**: ping-pong on password change — the new key is created in the other slot, and the old one is deleted only after verify + commit (see §6).
- **Envelope file `heng.dek.tpm`** (same directory as the DB, JSON): `{ version, scheme:"tpm-pcp", alg:"rsa2048-oaep-sha256", slot:"a"|"b", created_unix, wrapped_dek_hex }`.
- **The DEK never crosses IPC back to JS**: the unsealed DEK lives only in Rust-side `Crypto` state (`Zeroizing`, wiped on replace/drop); JS only issues commands (`set_password` / `unlock` / `db_open(encrypted)` …), and on open Rust internally takes the DEK from state as the raw key.
- **Commands are fully serialized**: each of the 5 crypto commands holds the `Crypto` mutex for its entire duration — change-password / unlock / remove / wipe all do multi-step non-atomic work on fixed slots + a single envelope, and concurrency would clash (e.g. a change-password commit racing another path's reconcile slot-delete = the DB permanently unsealable).

> Honest boundary: what PCP gives is "non-exportable key + password authorization + global DA rate limiting"; it **cannot read or change** the TPM's exact failure count and threshold (that's system-level policy). So this implementation **does not rely** on reading the chip's hardware counter.

---

## 4. Failure classification and "DA preemption"

Unlock / wrap failures are classified by `FailClass` (`crypto.rs`); the UI splits screens accordingly:

| Class | Trigger (HRESULT) | Meaning / UI |
| --- | --- | --- |
| `WrongPassword` | `NTE_PERM 0x80090010` | Wrong password (chip rejected the usage auth). **Consumes 1 DA strike.** |
| `Locked` | `TPM_20_E_LOCKOUT 0x80280921` / `TPM_E_DEFEND_LOCK_RUNNING 0x80280210` | Too many wrong tries triggered the chip's DA anti-brute-force lockout. **Does not judge password correctness, not counted toward any wipe**; resets via cooldown or the correct password (reboot may not clear it; the DA count decays over time). UI shows "chip temporarily locked" + the raw HRESULT. |
| `Corrupt` | `NTE_BAD_DATA 0x80090005` / envelope parse failure | Envelope or ciphertext damaged → "data may be corrupt." |
| `ChipUnavailable` | `NTE_NOT_FOUND` / `NTE_BAD_KEYSET` / handle errors / other | Chip busy, key missing, TBS error → "chip temporarily unavailable, please restart." **Not counted toward any wipe.** |
| `Internal` | directory resolution / IO / serialization | Infrastructure error (not one of the three unlock states). |

**DA preemption (measured on this machine, written honestly into the doc)**: this machine is an **fTPM** with a very low DA threshold — **about 3 wrong tries** and the chip flips from `WrongPassword` to `Locked` (returns `TPM_20_E_LOCKOUT`), then restores roughly 1 attempt's worth of budget about every 2 hours. Consequences:

- **"Guessing the password inside the app" is jammed by the chip itself almost immediately** — exactly what we want (brute force is hardware-rate-limited).
- This is also why the originally designed "software-layer auto-destroy after 5 wrong tries" **can't even fire on this machine** — the count hasn't reached 5 before the chip is already `Locked` (and `Locked` by design is not counted). The chip's rate limiting preempts it = safer, but makes "software-managed destroy" both slow and uncertain. **This is one of the measured reasons §5 removed auto-destroy.**
- **Testing discipline**: each wrong password costs 1 DA. **Don't repeatedly type wrong passwords just to test.** The TPM integration tests contain exactly **one** deliberate wrong password (`tpm_wrong_password_one_strike`, consuming exactly 1 strike); everything else uses correct passwords (0 DA).

> `NTE_NOT_FOUND` / `NTE_BAD_KEYSET` are currently coarsely classed as `ChipUnavailable`, but they can also mean **Clear TPM / motherboard swap / envelope copied to another machine** = the key is **gone for good** (DB unsealable). The UI keeps the raw code, so a dedicated "permanent loss" prompt could later be split out from "temporarily unavailable · restart."

---

## 5. Deletion = user-initiated "Wipe all data" (NO auto-destroy after N tries)

> **Major decision (2026-06-16, user's call): remove "auto-destroy after N wrong tries," replace with a user-initiated "Wipe all data."** The original design's entire "software-managed failure count → delete the key at N tries + a quarantine undo zone + a destroyed terminal screen + a sentinel" **has all been removed**; it survives only in the appendix as history.

**Why it was removed** (the user's reasoning):
1. **Lockout (keeping others from decrypting) should be the chip's job** — the TPM's global DA rate limiting already does exactly this, and on this machine the fTPM locks at ~3 tries (see §4), so stacking a software "count to 5" on top is both redundant and never reached.
2. **Deletion should be a deliberate user action**, not the system "fat-fingering" a delete for you. Auto-destroy carries accidental-deletion / backfire risk (handing "drop the DB" to anyone who can physically touch the computer and deliberately type wrong), while its benefit is preempted by the chip's rate limiting.
3. A software-triggered destroy is inherently weak: an adversary who pulls the power / kills the process / copies the envelope before the Nth try evades that one destroy. The real moat is "encryption + strong password + chip rate limiting," not the destroy action.

**What it became (as built)** — the `wipe_data` command + `engine::wipe` (`crypto.rs`):

- **Settings "Security" card → "Wipe all data…" button**:
  - **Encrypted**: must **enter the correct current password** (the command layer first runs `unlock` to verify — only a successful unseal counts as correct; otherwise it returns `WrongPassword`/`Locked`/… unchanged and **does not delete**) + a second confirmation.
  - **Plaintext**: no password, just a second confirmation.
- **Execution = permanent deletion right away** (no undo zone, no quarantine): delete both TPM slot keys (the encrypted DB is instantly unsealable) → delete the envelope `heng.dek.tpm` / staging `.new` / migration marker `heng.migrate` / pre-unlock state `heng.security` → delete `heng.db` and its `-wal/-shm` sidecars → clean up any historical `heng.destroyed*` quarantine residue (just in case; normally none).
  - **The DB delete retries + verifies it's actually gone**: on Windows a just-closed file handle may be released late and a bare `remove_file` can silently fail (a plaintext "wipe" falsely succeeds, data survives) → if it can't be deleted, it reports "wipe failed · retry" rather than falsely claiming success while plaintext data still exists.
- **Slot-delete tolerance**: the TPM is occasionally busy → small 3-try retry; if it still can't be deleted, let it go — what's left is a **benign orphan key** (the DB it protected is already deleted and it can't unseal anything; the next set-password's `create_slot_key(OVERWRITE)` + reconcile orphan cleanup will collect it).
- **Afterward**: JS-side `handleWiped` clears memory → `resetDesktopRepo` → opens a **brand-new empty plaintext DB** (one default book) → returns to the overview. A fresh start from a clean initial state.

Two explicit trade-offs (user's call): ① **permanent deletion right away**, no undo; ② **the button is available in plaintext too** (not encryption-only).

---

## 6. Unlock / set / change / remove password UX

Implementation: `apps/web/src/db.ts` (bootstrap gate) + `App.tsx` (startup state machine + auto-lock) + `UnlockScreen` + `SecurityCard`.

- **Bootstrap gate**: on desktop startup, first `security_status()` (which internally `reconcile`s to self-heal any interrupted migration/change-password) → if encrypted, **render the unlock screen first** (before the repo is ready), and only after the DEK is unsealed call `db_open(encrypted)`; if unencrypted, open the plaintext DB as before.
- **Unlock screen**: password box; on failure it splits screens per §4 (wrong password / locked / corrupt / chip unavailable), and the locked state shows the raw HRESULT. The password is typed by the user in a native input box; the assistant does not fill it.
- **Settings "Security" card**: status line (§2) + set / change / remove password + export plaintext backup (§7) + wipe data (§5) + auto-lock toggle.
- **Auto-lock**: default **15 minutes** of inactivity → the `lock` command clears the DEK + closes the DB → back to the unlock screen. Toggle / duration are configurable in Settings (`autoLockMinOf`).
- **Change password = a two-phase atomic protocol that re-wraps under a new key** (the spike found `PCP_CHANGEPASSWORD` unusable — it returns `NTE_INVALID_PARAMETER` and has a DA cost):
  1. Old password unseals the DEK (verifies the old password) → 2. the other slot wraps the same DEK with the new password → 3. write staging `heng.dek.tpm.new` (+fsync) → 4. verify from staging that the new password unseals it and == DEK → 5. atomically rename `.new` → envelope (**commit point**) → 6. delete the old slot key (non-fatal on failure; reconcile is the backstop).
  - **The database ciphertext is untouched** (only the DEK's wrapping key is swapped), sub-second.
  - **Startup self-heal recognizes both old/new envelope files**: a crash before commit (`.new` present, not renamed) → reconcile deletes the staging slot + `.new`, and the old password still unseals the original DEK; this eliminates "power loss mid-change-password leaving neither old nor new able to unseal = silent lockout."
- **Remove password = reverse ciphertext→plaintext migration** (see §8): verify the password → migrate the DB back to plaintext → delete the envelope + both slots. The wording makes clear "removing = the database reverts to plaintext, and anyone who gets the file can open it directly."

---

## 7. Backup and recovery

### Recovery — no local backdoor (user's call: forget = gone for good)
- **Ironclad rule**: keep **no "decrypt with the password alone" backdoor** locally. Full enumeration of consequences: **forget password / Clear TPM / OS reinstall / motherboard or CPU swap (fTPM bound to CPU) / TPM firmware-upgrade invalidation = local data gone for good.**
- **No recovery code** (it's essentially an offline-usable backdoor, conflicting with "truly hard").
- **Real recovery relies on future end-to-end cloud sync** (§9, deferred this round; note its single-device limit).

### Plaintext backup export (phase 4a, implemented)
The `export_backup` command + `engine::export_backup`:
- Goes through a native "Save As" dialog (`tauri-plugin-dialog`) for the path; **the actual file write still happens inside Rust** (no tauri-plugin-fs).
- A unified `sqlcipher_export` (not two code paths for plaintext/encrypted): write `.tmp` → export + backfill `PRAGMA user_version` (sqlcipher_export doesn't copy it) + `integrity_check` + per-table row-count verification → DETACH + close → no WAL sidecar → same-volume atomic rename to the destination. An encrypted DB must be decrypted out using the unlocked DEK; a plaintext DB exports directly.
- **The backup is always plaintext** — clearly labeled as "**the equivalent of turning encryption off, not password-protected**," so move it to offline media and don't keep it next to the computer.
- **Path collision guard**: refuses to export onto a `heng.*` control file inside the app data directory (preventing overwrite of the live DB/envelope).
- **Freshness awareness**: on success it writes `last_backup_unix/path` into `heng.security` (a plaintext state file readable while locked); the Settings card shows "last backup N days ago" with an overdue soft reminder.

---

## 8. Plaintext↔ciphertext atomic migration (the §9 protocol, implemented)

Implemented in `crypto.rs::engine` (`migrate_encrypt` / `migrate_decrypt` / `reconcile`). **The whole migration happens inside Rust** (Rust's `std::fs::rename` is an atomic replace on the same volume).

**Migration protocol** (plaintext→ciphertext; reverse is symmetric):
1. Close the State connection (so heng.db isn't held) → clean leftover tmp + sidecars.
2. Open the source DB → `wal_checkpoint(TRUNCATE)` to fold the WAL → ATTACH the target tmp (ciphertext with the raw key / plaintext with `KEY ''`).
3. `sqlcipher_export` the whole DB + **backfill user_version** + `integrity_check` + **verify per-table row counts match the source** (the cheap equivalent of §9's "row-by-row identical"; a full-row hash comparison is left for Phase 4 hardening).
4. DETACH → explicit `close` (synchronous, to avoid Windows delaying handle release and causing the subsequent rename to `ACCESS_DENIED`).
5. **Atomic rename tmp → heng.db (commit point)** → clean plaintext sidecars. Roll back before any failure (delete tmp).

**`rename_with_retry`**: on Windows a just-closed DB file may be briefly locked by AV / the indexer / a pending close, and rename reports `ACCESS_DENIED (os error 5)` — spike #2 probe2 deemed this **retryable**: small backoff, retry 12 times, then give up.

**Startup self-heal `reconcile` (DA-free, never guesses the password)**, three stages in dependency order:
1. **Migration marker `heng.migrate`** (holds direction + envelope): from the **DB file header** (plaintext `SQLite format 3\0` vs random ciphertext) decide which side of the atomic-rename commit point we're on → roll forward (commit the envelope) / roll back (delete the uncommitted envelope + tmp + staging slot). If the marker is corrupt, do **direction-agnostic** self-heal (plaintext DB ⇒ delete the stale envelope/slot; ciphertext DB ⇒ leave it). If a slot isn't fully deleted, keep the marker as a retry anchor for next time.
2. **Change-password staging `heng.dek.tpm.new`** (interrupted before commit): delete the staging slot + `.new` (the old password still unseals).
3. **Orphan slots**: clean up the residual key in the non-live slot of the live envelope.

The `set_password` / `remove_password` commands hold **both the Crypto and Db locks** throughout (fixed lock order Crypto→Db), close the connection → migrate → reopen. `security_status` runs `reconcile` first, then decides (startup-gate self-heal).

---

## 9. Future end-to-end cloud sync (deferred this round, interface reserved)

- **Goal (WhatsApp-like)**: data encrypted on your device, the server stores only gibberish and never sees plaintext; reuse the same DEK.
- **Cross-device key distribution = per-device key wrapping + a new device authorized by an existing one** (per-device revocable, avoiding collapsing E2E security down to password strength).
- **⚠️ The single-device user deadlock**: the target users mostly have a single computer. Per-device wrapping means the cloud copy is wrapped only by this device's key; if this one breaks / its chip is cleared, **there's no "other surviving device" to authorize a new one → the cloud blob is also forever unsealable**. That is, "cloud recovery" fails in exactly the scenario that most needs a backstop (the sole device breaks).
- **Trade-off (decided at the cloud-sync stage)**: to let single-device users recover from the cloud too, you must introduce a **device-independent recovery factor** (a password-derived KEK or a recovery code wrapping the DEK, stored in the cloud) — but that drops the strength back to password brute force. "Hard for others to recover" vs "a single-device user can recover for themselves" can't both hold. **Honestly stated this round: there is no cloud backstop right now.**
- Continuity with this round: the DEK is already a "random, chip-bound" standalone key, so adding cloud later is just "per-device wrapping" toward the cloud, no local rework needed.

---

## 10. Layered changes + build prerequisites

| Layer | Change |
| --- | --- |
| **New Rust core** | `crypto.rs`: NCrypt PCP creates the non-exportable wrapping key, password as authorization, wrap/unwrap the DEK, re-wrap on change-password, plaintext↔ciphertext migration, `reconcile` self-heal, `wipe` data, `export_backup`, `NCryptDeleteKey` to delete keys. `db.rs`: rusqlite + SQLCipher raw-key open, single-connection `Mutex<Connection>`, `db_batch` transactions. |
| **store** | tauri-plugin-sql wholesale-replaced by a self-written rusqlite+SQLCipher bridge (`$N→?N`, column_name→JSON, `PRAGMA key` first). Bound by the same `Repository` contract, zero change to SQL shape. `@app/store/crypto` exposes the commands + `FailClass`. |
| **app startup** | An unlock/set-password gate before bootstrap + an unlock screen (four-way failure split) + auto-lock. |
| **settings** | "Security" card: status line + set/change/remove password + backup export (freshness) + wipe data + auto-lock. |
| **platform** | **Windows only** (TPM 2.0). The no-chip software-only weak version is deferred (§2). Mac (Secure Enclave) / Linux later. |

### Build prerequisites (Windows)
SQLCipher is built via `rusqlite`'s `bundled-sqlcipher-vendored-openssl` feature, compiling the C source + vendored OpenSSL (libcrypto for AES/HMAC). Building locally needs:
- **The MSVC build environment** (`vcvars64.bat`, i.e. VS Build Tools "Desktop development with C++").
- **Strawberry Perl** (the vendored OpenSSL configure scripts need perl; **must be Strawberry Perl, not msys perl**).
- **nasm is NOT needed** (spike #1 proved this).
- The first build compiles OpenSSL once (~8 minutes, one-time). `tauri dev` uses `--no-default-features` and has a different cache key from `cargo test`, so each compiles OpenSSL once the first time.

See [Developer manual · Prerequisites](../../en/development.md).

---

## 11. Testing

`cargo test` (17 passed / 4 ignored; requires the vcvars + Strawberry Perl environment):

- **Pure-logic, always-run (0 DA, no TPM)**: UTF-16LE digest pinned (incl. empty string / multi-code-unit / no trailing NUL), hex roundtrip, envelope serde, `classify` mapping, slot helpers, migration-marker serde, DB-header detection.
- **SQLCipher / migration / backup / wipe, auto-run (0 DA, pure DEK no TPM)**: raw-key true-encryption roundtrip (header not plaintext + wrong-key fail-fast), plaintext→ciphertext→plaintext migration preserving data + user_version + header flip, backup export (plaintext/encrypted paths + path-collision guard), `wipe` removing all local data.
- **Real-TPM integration tests `#[ignore]`** (manual `cargo test -- --ignored --test-threads=1`; fixed slots must run serially): ① set→unlock→change→unlock→remove [0 DA] ② change-password crash-before-commit rollback self-heal [0 DA] ③ encrypt a test DB, lock→unlock→read back [0 DA] ④ **the single deliberate wrong password [consumes exactly 1 DA strike]**. Verified green in phase 2.

---

## Appendix · Design evolution (the plan changed during implementation; kept as history)

The original design doc (`encryption.md` v4) went through three red-team rounds; during implementation there were four major changes:

1. **Hard path NO-GO → soft path**: the original idea was to use a TPM **NV monotonic counter + PolicyNV ("count < N")** to make the chip itself refuse to unseal (approaching iPhone-grade determinism). Spike #3 measured: a non-elevated `TPM2_NV_DefineSpace` is blocked by the Windows TBS command filter (`TPM_E_COMMAND_BLOCKED 0x80280400`, regardless of whether owner-auth is empty) → fall back to the **soft path** (DEK wrapped by a PCP key + password authorization, §3).
2. **Encryption does NOT use "a password-KDF-derived key"**: changed to **a random DEK doing real encryption + the DEK wrapped by a non-exportable TPM key, password as PCP_USAGEAUTH** — copying the file gives no offline brute force, stronger than KDF (under a KDF route, copying the file allows offline brute-forcing the password).
3. **The §9 plaintext↔ciphertext atomic migration was folded into the "unlock/set-password" phase** (originally planned for a later phase): because an end-to-end "set password" can't stand without migration (otherwise setting a password leaves a corrupt "plaintext DB + envelope" state).
4. **"Auto-destroy after N wrong tries" was vetoed and removed** (see §5): the original design had an entire "software-managed count + a quarantine undo zone + a destroyed-terminal sentinel screen + three-way failure classification not counting toward destroy." This machine's fTPM DA locks at ~3 tries (< N=5) and `Locked` isn't counted → auto-destroy is slow and preempted by the chip, with accidental-deletion/backfire risk → changed to a user-initiated **"Wipe all data"** (§5). **Lockout is the chip's job; deletion is the user's deliberate action.**

> The still-valid parts of the v4 red team (retained in the implementation): opt-in plaintext-by-default + status line, honest threat-model disclosure (swap/hiberfil/SSD residue/Clear-TPM destroy DoS/bus sniffing), "corrupt vs wrong password" unlock-failure split, three-stage atomic migration + startup self-heal, retracting the "unencrypted backup" recommendation in favor of an explicit "equivalent of turning encryption off" + freshness awareness, the change-password independent TPM-object-switch atomic protocol, and laying out the single-device cloud-recovery deadlock.
