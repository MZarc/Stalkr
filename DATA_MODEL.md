# Stalkr Data Model Specification

## 1. Relational Entity-Relationship Diagram

```mermaid
erDiagram
    ACCOUNTS ||--o{ ACCOUNT_ALIASES : tracks
    ACCOUNTS ||--o{ RELATIONSHIP_STATE : maintains
    ACCOUNTS ||--o{ SNAPSHOTS : captures
    ACCOUNTS ||--o{ RELATIONSHIP_CHANGES : generates
    
    PEOPLE ||--o{ RELATIONSHIP_STATE : relates_to
    PEOPLE ||--o{ SNAPSHOT_MEMBERS : recorded_in
    PEOPLE ||--o{ RELATIONSHIP_CHANGES : subject_of
    PEOPLE ||--o{ PERSON_TAGS : labeled_with
    
    SNAPSHOTS ||--o{ SNAPSHOT_MEMBERS : contains
    TAGS ||--o{ PERSON_TAGS : classifies
    
    ACCOUNTS ||--o{ NOTES : annotated_with
    PEOPLE ||--o{ NOTES : annotated_with

    ACCOUNTS {
        TEXT id PK "Stable UUID"
        TEXT instagram_user_id UK "Meta numeric identity"
        TEXT username "Current handle"
        TEXT full_name
        TEXT profile_pic_url
        INTEGER followers_count
        INTEGER following_count
        TEXT last_sync_at
        TEXT created_at
        TEXT updated_at
    }

    ACCOUNT_ALIASES {
        TEXT id PK
        TEXT account_id FK
        TEXT username
        TEXT first_seen_at
        TEXT last_seen_at
    }

    PEOPLE {
        TEXT id PK "Internal UUID"
        TEXT instagram_user_id UK "Meta numeric identity"
        TEXT current_username "Latest handle"
        TEXT full_name
        TEXT profile_pic_url
        INTEGER is_verified
        INTEGER is_private
        TEXT first_discovered_at
        TEXT last_updated_at
    }

    RELATIONSHIP_STATE {
        TEXT account_id PK,FK
        TEXT person_id PK,FK
        INTEGER is_follower "1 = true, 0 = false"
        INTEGER is_following "1 = true, 0 = false"
        TEXT relationship_type "mutual, fan, following_only, none"
        TEXT last_verified_at
        TEXT created_at
        TEXT updated_at
    }

    SNAPSHOTS {
        TEXT id PK
        TEXT account_id FK
        TEXT direction "followers, following, both"
        TEXT status "complete, partial, failed"
        INTEGER member_count
        TEXT created_at
    }

    SNAPSHOT_MEMBERS {
        TEXT snapshot_id PK,FK
        TEXT person_id PK,FK
        TEXT username_at_snapshot
    }

    RELATIONSHIP_CHANGES {
        TEXT id PK
        TEXT account_id FK
        TEXT person_id FK
        TEXT change_type "followed, unfollowed, lost_follower, gained_follower, rename"
        TEXT direction "incoming, outgoing, neutral"
        TEXT snapshot_id_before FK
        TEXT snapshot_id_after FK
        TEXT detected_at
    }

    TAGS {
        TEXT id PK
        TEXT name UK
        TEXT color
        TEXT created_at
    }

    PERSON_TAGS {
        TEXT person_id PK,FK
        TEXT tag_id PK,FK
        TEXT created_at
    }

    NOTES {
        TEXT id PK
        TEXT target_type "account, person"
        TEXT target_id
        TEXT ciphertext "AES-256-GCM encrypted"
        TEXT nonce "12-byte CSPRNG IV"
        TEXT created_at
        TEXT updated_at
    }
```

---

## 2. Materialized Relationship States

The `relationship_state` table is continuously maintained by the `DiffEngine` to allow sub-millisecond querying without reconstructing historical diff trees:

| `is_follower` | `is_following` | `relationship_type` | Visual Representation |
|---|---|---|---|
| `1` | `1` | `mutual` | Concentric Radar Ring 1 (Emerald) |
| `1` | `0` | `fan` | Concentric Radar Ring 2 (Amber) |
| `0` | `1` | `following_only` | Concentric Radar Ring 3 (Slate) |
| `0` | `0` | `none` | Archival / Historical only |

---

## 3. High-Performance Indexing Strategy

```sql
-- Fast filter by relationship type
CREATE INDEX idx_rel_state_type ON relationship_state (account_id, relationship_type);

-- Fast lookup of changes by time
CREATE INDEX idx_rel_changes_time ON relationship_changes (account_id, detected_at DESC);

-- Fast lookup of snapshots by account
CREATE INDEX idx_snapshots_account ON snapshots (account_id, created_at DESC);

-- Fast person lookup by Instagram identity
CREATE INDEX idx_people_ig_id ON people (instagram_user_id);
```
