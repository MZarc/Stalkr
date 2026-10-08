# Stalkr Provider Architecture & Integration Guide

## 1. Provider Isolation Overview

Instagram's unofficial endpoints, rate limits, and response shapes change over time. Stalkr decouples all network transport and parsing behind the `InstagramProvider` Rust trait (`src-tauri/src/providers/provider_trait.rs`):

- Core list/profile/access methods, plus **defaulted no-op secondary-surface methods** that only the live session provider overrides — so a dead secondary source can never break a sync.
- Shared helpers (e.g. multi-surface union with ID-first, case-insensitive-username-fallback identity keys) live next to the trait.
- No view, command handler, or database routine depends on raw HTTP endpoints or response payloads.

---

## 2. Production Providers

### 2.1 Authenticated Session Provider
- **Purpose**: Live synchronization using the user's own Instagram login session.
- **Primary surface**: Mobile friendships endpoints (`friendships/{id}/followers|following`), cursor-paginated.
- **Secondary surface** (best-effort): Web GraphQL follower/following edges in the style popularised by Instaloader. It runs **only** when Instagram's own header count says the primary surface came up short, is capped in pages, and any failure (HTTP 429/400, retired query hashes, schema drift) is silently ignored with the primary result standing alone. The union step is additive by construction.
- **Pacing & backoff** (aimed at keeping load minimal — not a guarantee of any outcome):
  - ~0.9–1.5s jittered delay between pages (fixed machine-like intervals are avoided on purpose).
  - On HTTP 429: back off ~4s, then ~9s, then stop the sync entirely and report rate-limited rather than pushing through.
  - A local per-account sync cooldown plus randomized background-sync windows further spread load.
- **Session handling**: HTTP 401/403 surfaces as session-expired (re-authenticate), never as data. Numeric user-ID resolution never falls back to local UUIDs — failing loudly beats fetching the wrong person's list.

### 2.2 Official Multi-Shard Export Provider
- **Purpose**: Importing official Instagram data-download archives — the lowest-risk data source since it needs no live session at all.
- **Shard discovery**: scans `followers*.json` / `following*.json` across the export directory (including `followers_and_following/` and `connections/` layouts) and de-duplicates case-insensitively.
- **Schema tolerance**: handles both the top-level-array shape and the `relationships_following` / `relationships_followers` object shapes. Export rows carry no numeric IDs, so identity backfills by username when live data later provides IDs.
- Imports are treated as ground truth for the imported account (no confirmation delay).

### 2.3 Public Profile Inspector Provider
- **Purpose**: Unauthenticated metadata only (counts, avatar, verification flag).
- **Strict boundary**: never attempts relationship-list retrieval.

---

## 3. Test & Development Provider

### 3.1 Deterministic Fixture Provider
- **Purpose**: Offline development, unit testing, and UI verification.
- **Characteristics**: Generates synthetic, deterministic relationship graphs with customizable follower, following, and mutual counts.
- Whenever fixture data is on screen, the UI labels it demo data.

---

## 4. Provider Diagnostics & Health States

`get_provider_health` reports a `ProviderHealthStatus` with connectivity, per-list retrieval flags, pagination/completeness flags, last successful sync, provider version, and the last error text. Access checks for monitored targets resolve to one of: `accessible`, `not_accessible`, `auth_required`, `rate_limited`, `provider_error`, or `unknown` (with a stored-ID direct probe as fallback before giving up with `unknown`).

| Situation | Meaning | Suggested user action |
|---|---|---|
| Rate limited (HTTP 429) | Instagram is throttling this session | Wait 15–30 minutes; prefer the hourly schedule |
| Session expired / auth required | Saved session no longer accepted | Re-authenticate from the connect screen |
| Not accessible | List not visible through this session (e.g. private target, not a follower) | Nothing is recorded — by design; check follow status |
| Unknown after recheck | Neither profile lookup nor direct probe resolved | Verify the username/ID, reconnect, retry once |
