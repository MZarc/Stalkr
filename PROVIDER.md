# Stalkr Provider Architecture & Integration Guide

## 1. Provider Isolation Overview

Instagram frequently modifies internal web endpoints, GraphQL query hashes, and rate-limiting responses. Stalkr decouples all network transport and parsing logic behind the `InstagramProvider` Rust trait:

```rust
#[async_trait]
pub trait InstagramProvider: Send + Sync {
    async fn fetch_profile(&self, username: &str) -> Result<InstagramUser, ProviderError>;
    async fn fetch_followers(&self, user_id: &str) -> Result<Vec<InstagramUser>, ProviderError>;
    async fn fetch_following(&self, user_id: &str) -> Result<Vec<InstagramUser>, ProviderError>;
    fn provider_type(&self) -> &'static str;
}
```

No view, command handler, or database routine ever depends on raw HTTP endpoints or response payloads.

---

## 2. Production Providers

### 2.1 Authenticated Session Provider
- **Purpose**: Direct live synchronization using an active Instagram Web session.
- **Headers & Identity**:
  - `User-Agent`: Modern Android mobile web browser user agent.
  - `X-IG-App-ID`: `936619743392459` (Current mobile web client application ID).
  - `Accept`: `*/*`
- **Cursor Pagination**: Follower and following lists are retrieved in sequential chunks via cursor pagination (`max_id` or GraphQL cursors).
- **Rate-Limiting & Jitter**:
  - Automatically handles HTTP `429 Too Many Requests`.
  - Introduces randomized exponential jitter between page fetches (1.5s - 4.5s) to avoid burst detection.
  - Halts pagination immediately upon receiving a `checkpoint_required` or session invalidation response without emitting false unfollows.

### 2.2 Official Multi-Shard Export Provider
- **Purpose**: Completely safe, zero-risk import of official Instagram account downloads.
- **Multi-Shard Shard Discovery**:
  Meta exports partition large follower lists into multiple JSON shards:
  ```
  followers_and_following/
      ├── followers_1.json
      ├── followers_2.json
      ├── followers_3.json
      └── following.json
  ```
  The parser scans for any file matching `followers*.json` (regex or glob) rather than hardcoding a single filename.
- **Resilient Parsing**:
  Supports both schema variants:
  - Variant A: Top-level array of `{ "string_list_data": [{ "value": "username", "timestamp": 1234567890 }] }`
  - Variant B: Root object `{ "relationships_followers": [ ... ] }`

### 2.3 Public Profile Inspector Provider
- **Purpose**: Unauthenticated profile inspection for basic metadata.
- **Strict Boundary**: Fetches public bio, follower/following counts, profile picture URL, and verified badge.
- **Safety Rule**: **Never attempts follower or following list scraping**. Unauthenticated scraping of relationships is structurally unreliable and violates Stalkr's core principle of correctness over fake completeness.

---

## 3. Test & Development Provider

### 3.1 Deterministic Fixture Provider
- **Purpose**: Offline development, unit testing, and UI verification.
- **Characteristics**: Generates synthetic, deterministic relationship graphs with customizable follower, following, and mutual counts.
- **Labeling Non-Negotiable**: Whenever data originates from the Fixture Provider, the UI permanently renders a high-visibility badge: `DEMO DATA`.

---

## 4. Provider Diagnostics & Health States

Each provider reports health status through the `get_provider_health` IPC command:

| Health State | Meaning | Suggested User Action |
|---|---|---|
| `HEALTHY` | Provider is responsive and operating normally | None |
| `RATE_LIMITED` | HTTP 429 received from Meta servers | Back off sync; retry automatically in 15–60 minutes |
| `SESSION_EXPIRED` | Instagram session cookies invalid or logged out | Re-authenticate via Settings > Session |
| `CHECKPOINT_REQUIRED` | Instagram requested 2FA or security challenge | Log into Instagram app/web to resolve checkpoint |
| `EXPORT_PARSED` | Official archive loaded successfully | Safe to inspect historical data |
