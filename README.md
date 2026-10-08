<div align="center">

<img src="icon.png" alt="Stalkr" width="120" />

# Stalkr

**A private, local-first Android app for keeping an eye on Instagram followers, following, and relationship changes — for your own profile and people you monitor, with safeguards designed to reduce false reports.**

[![Android](https://img.shields.io/badge/Android-API%2024%2B-3DDC84?logo=android&logoColor=white)](https://developer.android.com)
[![arm64](https://img.shields.io/badge/ABI-arm64--v8a-blueviolet)](https://developer.android.com/ndk/guides/abis)
[![Tauri](https://img.shields.io/badge/Tauri-v2-ffc131?logo=tauri&logoColor=black)](https://tauri.app)
[![Svelte](https://img.shields.io/badge/Svelte-v5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev)
[![Rust](https://img.shields.io/badge/Rust-2024-black?logo=rust)](https://www.rust-lang.org)
[![SQLite](https://img.shields.io/badge/SQLite-local--first-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)

*Zero cloud. Zero telemetry. Your sessions and your data never leave your device.*

</div>

---

## ✨ Why Stalkr

Follower lists from Instagram can wobble between requests, and naive trackers turn every wobble into phantom events. Stalkr is built around safeguards meant to — reduce false reports — it tries hard **not to report changes it cannot stand behind**:

- **Two-cycle event confirmation** — a disappearance is held aside first and only enters your feed if it's still gone on the *next* sync, so one-off list wobbles stay out of your history.
- **Dual-surface fetching** — primary mobile endpoint + Instaloader-style web GraphQL surface, unioned. The union step is additive by construction (it adds members the first surface missed but never removes anyone on its own), which narrows coverage gaps without inventing removals.
- **Honest headline counts** — the dashboard leads with Instagram's official header count (aiming to match what the Instagram app shows) and shows your verified tracked list underneath it. The small residual gap (deactivated/restricted accounts Instagram counts but serves to no one) is labeled, not hidden.
- **Quarantine, not fiction** — truncated pages, rate limits, and revoked access produce clear "incomplete, nothing stored" states instead of fake events.
- **Session-safe by design** — human-paced jittered requests, backoff on HTTP 429 with a full stop under repeated throttling, a local sync cooldown, and randomized background-sync windows. This is intended to keep load minimal, but no third-party tool can promise Instagram will never throttle or flag automated access — keep schedules conservative (hourly is the recommended default) and use at your own risk.
- **Username-change tracking** — identity is keyed on stable numeric Instagram IDs, so `priya_99` → `priya.sharma` records one rename event instead of a phantom unfollow + follow pair.

## 📱 Features

| View | What you get |
|---|---|
| **Dashboard** | Official follower/following counts, tracked-list coverage, 7-day net change, 7-day velocity sparkline with real weekday labels, reciprocity gauge |
| **Circle** | Full directory with filters: All, Mutuals, Not Following Back, Fans + search |
| **Changes** | Confirmed-only audit feed: Gained, Lost, Outbound, Renames |
| **Sync** | Per-target sync, access-state checks (accessible / not accessible / rate-limited / session expired), stealth background-sync scheduling |
| **Settings** | Official Instagram ZIP export import, encrypted notes, app lock, database wipe |

Under the hood: snapshot history in local SQLite, AES-256-GCM encrypted sessions and notes, headless background sync via Android `WorkManager` + JNI bridge, and a demo mode with clearly-labeled synthetic data.

---

## 🚀 Get the app running from a fresh clone

These steps were verified end-to-end. Follow them in order and the build works first try.

### 1. Requirements

| Tool | Version / notes |
|---|---|
| **Node.js + npm** | Node 20+ (verified on Node 24 + npm 11) |
| **Rust** | 1.90+ (`rustup update stable`), plus Android targets below |
| **Java (JDK)** | 17+ and set `JAVA_HOME` to it |
| **Android SDK** | `platform-tools`, one `platforms;android-3x` image, `build-tools`, Android NDK r27 (set `ANDROID_HOME` to the SDK root, e.g. `C:\Android`) |
| **Git** | any recent version |

Install the Rust Android targets (arm64 is the primary build target):

```bash
rustup target add aarch64-linux-android
```

> First-time note: a fresh clone has **no** build caches (they're gitignored), so the first Android build takes ~10–15 minutes while Rust and Gradle compile everything. Later builds take ~2–4 minutes.

### 2. Clone & install

```bash
git clone https://github.com/MZarc/Stalkr.git
cd Stalkr
npm ci
```

> Use `npm ci` (not `npm install`) so dependencies match `package-lock.json` exactly.

### 3. Build the production APK (arm64)

```bash
npm run tauri -- android build --target aarch64 --ci
```

This builds the web frontend, compiles the Rust core for `aarch64-linux-android`, and packages the APK at:

```
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk
```

Copy it to your phone and install (enable *Install unknown apps* for your file manager/browser when prompted). The APK contains **only** the `arm64-v8a` native library.

### 4. Everyday commands

```bash
npm run dev      # Vite dev server (web preview)
npm run check    # Svelte + TS type checks
npm test         # Node test suite (tests/)
cargo test --manifest-path src-tauri/Cargo.toml   # Rust suite: diff engine,
                                                  # verification gates, union logic,
                                                  # sync pipeline, encryption (30 tests)
```

### 5. Using the app

1. Open the app → connect your Instagram via the official in-app login (your session is encrypted on-device; nothing is sent anywhere except Instagram itself).
2. Your profile syncs automatically — counts appear on the Dashboard.
3. Add a target with **Sync → +** (private targets require your logged-in account to be an approved follower).
4. Sync each profile from its view. The **first** sync is always a baseline (no change events by design); gains, losses, and renames start appearing from the **second** sync onward, after two-cycle confirmation.
5. Optional: enable stealth background sync per target from the Sync view.

---

## 🧠 How it tries to stay honest (and where it can still be wrong)

1. Each sync paginates the full follower + following lists, then — only if Instagram's own header says members are missing — pulls the secondary web surface and unions both.
2. Results are reconciled against Instagram's official counts: catastrophic corruption is quarantined (nothing stored, clear message); moderate gaps flow through labeled `Unconfirmed`.
3. Candidate follow/unfollow events wait one more sync for confirmation; flaps are discarded; reappearances after a single miss are ignored rather than reported as "new".
4. Counts in the UI reflect the latest verified list; the dashboard headline follows Instagram's official number when it can be fetched.

Known limits, stated plainly: the first sync of any profile is only a baseline (no events by design); real follows/unfollows generally appear one sync later than they happen; a few untrackable accounts (deactivated, restricted) may keep the tracked list slightly below the official total; and if Instagram serves inconsistent slices on back-to-back syncs, some true events can be delayed while contradictions resolve.

## 🛡️ Privacy & safety

- **Local-first**: SQLite database, sessions, snapshots, and notes live only on your device. No accounts, no servers, no analytics.
- **Encrypted at rest**: Instagram sessions and private notes are AES-256-GCM encrypted (Android Keystore-backed keys on device).
- **Read-only behavior**: the app never follows, unfollows, likes, or messages anyone, and never attempts to bypass private-account privacy — inaccessible targets simply report `not_accessible`.
- **Anti-ban posture**: jittered human-paced requests, backoff + full stop on rate limits, and a local sync cooldown. That said, Stalkr is an **unofficial** tool: use your own account, keep sync intervals sane (hourly recommended), and accept Instagram's terms and rate limits as ground truth.

## 🗂️ Project layout

```
src/                 # Svelte 5 + TypeScript frontend (views, components, API layer)
src-tauri/src/       # Rust core: sync engine, diff + verification, providers,
                     #   encrypted storage (db/), commands, JNI bridge
src-tauri/gen/android/  # Android shell: Gradle project + custom Kotlin
                     #   (login WebView, biometric + auth bridges, WorkManager sync)
src-tauri/icons/     # App icons (bundled into the APK — do not delete)
scripts/             # Icon generation utilities
tests/               # Node test suite
```

Regenerable (gitignored) artifacts: `src-tauri/target*/`, Gradle `build/` + `.gradle/`, `dist/`, `release/` APKs, `graft/` index.

## 📚 Deeper docs

- [ARCHITECTURE.md](ARCHITECTURE.md) — layers, sync pipeline, verification design
- [DATA_MODEL.md](DATA_MODEL.md) — relational schema and entity relationships
- [PROVIDER.md](PROVIDER.md) — data providers (session, export, public, fixture)
- [SECURITY.md](SECURITY.md) — encryption domains, key handling, log sanitization
- [PRIVACY.md](PRIVACY.md) — data boundaries and retention

## 🩺 Troubleshooting

| Symptom | Fix |
|---|---|
| `SDK location not found` / Gradle can't find Android | Set `ANDROID_HOME` to your SDK root (or keep `src-tauri/gen/android/local.properties` with `sdk.dir=` + `ndk.dir=`) |
| `NDK version disagrees` warning (`CXX1104`) | Harmless warning — the build uses the installed NDK and succeeds |
| Sync says *throttled, Ns cooldown* | Wait out the countdown, then sync once — it protects your session |
| Sync says *rate limited / 429* | Instagram is throttling: wait 15–30 min, keep the hourly schedule |
| *Session expired / re-authenticate* | Log in again from the connect screen; sessions do expire |
| *Empty lists / reconnect* on first sync | Session isn't returning data — reconnect Instagram, then sync |
| Target shows `not_accessible` | Your logged-in account can't view that list (private + not a follower). Nothing is recorded — by design |
| Changes page empty after 1st sync | Expected — the first sync is the baseline; events start from the 2nd sync |

## License

MIT — see [LICENSE](LICENSE).

## ⚠️ Disclaimer

Stalkr is a private, educational, personal-use project. It is **not affiliated with, endorsed by, or sponsored by Meta or Instagram**. Automating access to Instagram carries account-risk trade-offs that are yours to accept — use responsibly and within applicable laws and terms of service.
