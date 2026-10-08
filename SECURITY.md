# Stalkr Security Specification

> What this document claims is limited to what the code in `src-tauri/src/security/` and the Android shell actually does. Anything stronger (hardware-backed keys, biometric-bound crypto) is a roadmap item, not a current property.

## 1. Threat Model & Security Posture

Stalkr assumes a phone that could be lost, backed up, or inspected. The posture is simple: keep everything on-device, encrypt secrets at rest, and never send user data anywhere except Instagram's own servers during a sync the user initiated.

---

## 2. Encryption at Rest (What Exists Today)

Sessions and private notes are encrypted with **AES-256-GCM** (authenticated encryption) before touching SQLite:

- **Key derivation**: SHA-256 over app-defined domain-separation strings (e.g. a session domain and per-note seeds) into 32-byte keys. This is *not* HKDF, and keys are **not** currently held in the Android Keystore — they are derived in-app. Moving key material into hardware-backed storage is planned work, not a shipped property.
- **Nonce**: 96-bit random value per encryption, stored alongside the ciphertext (both Base64).
- **Payloads**: Instagram session JSON (`session_id`, `ds_user_id`, tokens/cookies) in `account_sessions`; note bodies in `notes` (`content_ciphertext` + `nonce`, one row per account+person).
- **Database file itself is not encrypted at rest** — protection comes from the Android app sandbox plus the per-value encryption above. A rooted device or a backup with the app data could expose metadata (usernames, counts, timestamps).

### 2.1 Roles (Enforced by Code Paths, Not by Hardware Domains)

- **Background sync** (`SyncWorker` → JNI → Rust) can decrypt *sessions* to run headless syncs. It has no code path that decrypts notes.
- **Notes** are only decrypted in UI flows (with the app-lock/biometric gate in front where enabled).

---

## 3. Session & Secret Hygiene

1. **No hardcoded secrets**: the repository contains no session cookies, tokens, or API keys (verified by pattern scan). Credentials only ever enter at runtime via the user's own login.
2. **Transport**: outbound traffic uses HTTPS via `rustls`. Release builds set `usesCleartextTraffic=false`; debug builds allow cleartext only for the local Vite dev server.
3. **Log caution**: logs may contain operational metadata (sync states, counts). verbose logging of raw network bodies is avoided; users sharing logs should still redact by hand anything sensitive.

---

## 4. Honest Limitations

- Encryption is only as strong as the device: no verified-boot/attestation checks, no anti-tamper, no remote wipe.
- The app cannot prevent Instagram from throttling, challenging, or flagging automated access — see the cautious request etiquette in [PROVIDER.md](PROVIDER.md) and the disclaimer in [README.md](README.md).
- Backups of app data carry the encrypted database; anyone with the backup and the app's derivation logic faces only the AES layer, not hardware binding.
