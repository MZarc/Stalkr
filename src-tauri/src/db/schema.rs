pub const CREATE_TABLES_SQL: &str = r#"
PRAGMA foreign_keys = ON;

-- Accounts (Owner or Monitored Target)
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY NOT NULL,
    instagram_user_id TEXT UNIQUE,
    username TEXT NOT NULL,
    display_name TEXT NOT NULL,
    account_kind TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    avatar_url TEXT,
    is_private INTEGER NOT NULL DEFAULT 0,
    is_verified INTEGER NOT NULL DEFAULT 0,
    followers_count INTEGER NOT NULL DEFAULT 0,
    following_count INTEGER NOT NULL DEFAULT 0,
    monitoring_enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_successful_sync_at INTEGER,
    last_attempted_sync_at INTEGER,
    authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
    access_state TEXT NOT NULL DEFAULT 'unknown',
    access_reason TEXT,
    target_privacy TEXT NOT NULL DEFAULT 'unknown',
    last_access_checked_at INTEGER
);

-- Encrypted Session Credentials for Owner Accounts (Key A domain)
CREATE TABLE IF NOT EXISTS account_sessions (
    account_id TEXT PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    session_data_ciphertext TEXT NOT NULL,
    nonce TEXT NOT NULL,
    last_validated_at INTEGER NOT NULL,
    is_healthy INTEGER NOT NULL DEFAULT 1,
    error_message TEXT
);

-- Account Aliases
CREATE TABLE IF NOT EXISTS account_aliases (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    username TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    PRIMARY KEY (account_id, username)
);

-- Canonical People Entity
CREATE TABLE IF NOT EXISTS people (
    id TEXT PRIMARY KEY NOT NULL,
    instagram_user_id TEXT UNIQUE,
    current_username TEXT NOT NULL,
    display_name TEXT,
    avatar_url TEXT,
    is_verified INTEGER DEFAULT 0,
    is_private INTEGER DEFAULT 0,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL
);

-- Materialized Relationship State
CREATE TABLE IF NOT EXISTS relationship_state (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    is_follower INTEGER NOT NULL DEFAULT 0,
    is_following INTEGER NOT NULL DEFAULT 0,
    first_observed_at INTEGER NOT NULL,
    last_observed_at INTEGER NOT NULL,
    authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, person_id)
);

-- Complete Historical Snapshots
CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    snapshot_type TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL,
    item_count INTEGER NOT NULL DEFAULT 0,
    source_hash TEXT NOT NULL,
    error_message TEXT,
    authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE
);

-- Snapshot Membership
CREATE TABLE IF NOT EXISTS snapshot_members (
    snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    username_at_snapshot TEXT NOT NULL,
    display_name_at_snapshot TEXT,
    avatar_url_at_snapshot TEXT,
    PRIMARY KEY (snapshot_id, person_id)
);

-- Computed Relationship Changes
CREATE TABLE IF NOT EXISTS relationship_changes (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    related_username TEXT NOT NULL,
    change_type TEXT NOT NULL,
    confidence TEXT NOT NULL,
    metadata_json TEXT,
    detected_at INTEGER NOT NULL,
    before_snapshot_id TEXT REFERENCES snapshots(id),
    after_snapshot_id TEXT REFERENCES snapshots(id),
    authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE
);

-- Unconfirmed candidate events awaiting two-cycle confirmation.
-- A disappearance becomes an "unfollow" only if still gone on the next
-- sync; single-sync list flaps are deleted here and never reach the feed.
CREATE TABLE IF NOT EXISTS pending_events (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    related_username TEXT NOT NULL,
    change_type TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    first_snapshot_id TEXT,
    authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE (account_id, person_id, change_type)
);

-- Tags
CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    color TEXT
);

-- Person Tags
CREATE TABLE IF NOT EXISTS person_tags (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, person_id, tag_id)
);

-- Encrypted Private Notes (Encrypted at rest via Key B)
CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    content_ciphertext TEXT NOT NULL,
    nonce TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (account_id, person_id)
);

-- Local Settings
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
"#;

pub const CREATE_INDICES_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_accounts_username ON accounts(username);
CREATE INDEX IF NOT EXISTS idx_accounts_ig_id ON accounts(instagram_user_id);
CREATE INDEX IF NOT EXISTS idx_accounts_auth_by ON accounts(authenticated_by_account_id);
CREATE INDEX IF NOT EXISTS idx_people_ig_id ON people(instagram_user_id);
CREATE INDEX IF NOT EXISTS idx_people_username ON people(current_username);
CREATE INDEX IF NOT EXISTS idx_rel_state_flags ON relationship_state(account_id, is_follower, is_following);
CREATE INDEX IF NOT EXISTS idx_rel_state_auth_by ON relationship_state(account_id, authenticated_by_account_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_account ON snapshots(account_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_snapshots_auth_by ON snapshots(account_id, authenticated_by_account_id);
CREATE INDEX IF NOT EXISTS idx_snapshot_members_person ON snapshot_members(person_id);
CREATE INDEX IF NOT EXISTS idx_rel_changes_account_date ON relationship_changes(account_id, detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_rel_changes_person ON relationship_changes(person_id);
"#;
