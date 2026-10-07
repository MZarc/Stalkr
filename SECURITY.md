# Stalkr Security Specification

## 1. Threat Model & Security Posture

Stalkr is designed under the assumption of an untrusted mobile environment where physical device access, OS backups, and memory inspection could occur. The application adheres to a zero-trust model toward third-party servers and cloud providers: **no user data ever leaves the local device**.

---

## 2. Dual Keystore Architecture

Stalkr partitions security keys into two distinct cryptographic domains to balance headless background automation with high-assurance biometric protection:

```
+-------------------------------------------------------------------------+
|                              Android Keystore                           |
+------------------------------------+------------------------------------+
|               Domain A             |              Domain B              |
|        Background Service Key      |      App-Lock & Private Data Key   |
+------------------------------------+------------------------------------+
| * Hardware-backed (TEE/StrongBox)  | * Hardware-backed (TEE/StrongBox)  |
| * Unauthenticated access           | * Biometric authentication bound   |
| * Dedicated to WorkManager sync    | * Requires BiometricPrompt auth    |
| * Encrypts provider session tokens | * Unlocks UI session               |
| * Cannot decrypt user notes        | * Encrypts sensitive private notes |
+------------------------------------+------------------------------------+
```

### 2.1 Key A: Background Service Domain
- **Usage**: Used exclusively by `SyncWorker` and the JNI bridge during headless background execution.
- **Access Rule**: Requires device-level encryption, but does **not** prompt for biometric authentication, allowing WorkManager to operate when the device is locked.
- **Scope**: Can read/write network session tokens and update `relationship_state`. Strictly barred from accessing or decrypting private notes.

### 2.2 Key B: User-Facing Biometric Domain
- **Usage**: User interface unlock and personal note encryption/decryption.
- **Access Rule**: Cryptographically bound to the user's biometric enrolled credentials (`setUserAuthenticationRequired(true)`).
- **Scope**: Decrypts the master note encryption key into transient memory only while the UI is unlocked. Memory is wiped when the app transitions to the background.

---

## 3. Note Encryption at Rest

All user notes attached to accounts or profiles are encrypted before being written to SQLite using authenticated symmetric encryption:

- **Algorithm**: `AES-256-GCM` (Galois/Counter Mode).
- **Key Derivation**: SHA-256 HKDF over Key B master material.
- **Nonce/IV**: 96-bit (12-byte) cryptographically secure pseudorandom number generated per encryption operation via OS CSPRNG.
- **Authentication Tag**: 128-bit (16-byte) GMAC integrity tag.
- **Payload Format**: Base64-encoded binary packet:
  ```
  [ 12-byte IV ] + [ Ciphertext ] + [ 16-byte GCM Tag ]
  ```
- **Database Schema**:
  ```sql
  CREATE TABLE notes (
      id TEXT PRIMARY KEY,
      target_type TEXT NOT NULL,
      target_id TEXT NOT NULL,
      ciphertext TEXT NOT NULL,
      nonce TEXT NOT NULL,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
  );
  ```

---

## 4. Session Token Isolation & Sanitization

1. **No Hardcoded Tokens**: No default session cookies or API keys exist in the repository.
2. **Log Sanitization**: The logging pipeline (`tauri-plugin-log`) enforces strict redaction filters:
   - All `sessionid`, `csrftoken`, `ds_user_id`, and `Authorization` headers are masked as `[REDACTED]`.
   - Raw HTTP responses containing account passwords or challenge codes are truncated and discarded before log emission.
3. **Transport Security**:
   - Outbound requests use modern TLS 1.3 via `rustls`.
   - Android release builds explicitly enforce `android:usesCleartextTraffic="false"` in `AndroidManifest.xml`.
