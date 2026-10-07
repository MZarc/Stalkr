use std::path::Path;
use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection, Result, Row};
use crate::models::*;
use crate::db::schema::{CREATE_TABLES_SQL, CREATE_INDICES_SQL};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    fn run_migrations(conn: &Connection) -> Result<()> {
        let _ = conn.execute("PRAGMA foreign_keys = OFF;", []);
        let _ = conn.execute("ALTER TABLE accounts ADD COLUMN instagram_user_id TEXT", []);
        let _ = conn.execute("ALTER TABLE accounts ADD COLUMN authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL", []);
        let _ = conn.execute("ALTER TABLE accounts ADD COLUMN access_state TEXT NOT NULL DEFAULT 'unknown'", []);
        let _ = conn.execute("ALTER TABLE accounts ADD COLUMN access_reason TEXT", []);
        let _ = conn.execute("ALTER TABLE accounts ADD COLUMN target_privacy TEXT NOT NULL DEFAULT 'unknown'", []);
        let _ = conn.execute("ALTER TABLE accounts ADD COLUMN last_access_checked_at INTEGER", []);
        let _ = conn.execute("ALTER TABLE snapshots ADD COLUMN authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE", []);
        let _ = conn.execute("ALTER TABLE relationship_state ADD COLUMN authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE", []);
        let _ = conn.execute("ALTER TABLE relationship_changes ADD COLUMN authenticated_by_account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE", []);
        let _ = conn.execute("DELETE FROM accounts WHERE id = 'acc_primary_demo'", []);
        let _ = conn.execute("PRAGMA foreign_keys = ON;", []);
        Ok(())
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(CREATE_TABLES_SQL)?;
        Self::run_migrations(&conn)?;
        conn.execute_batch(CREATE_INDICES_SQL)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(CREATE_TABLES_SQL)?;
        Self::run_migrations(&conn)?;
        conn.execute_batch(CREATE_INDICES_SQL)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(sql)
    }

    // --- Accounts ---

    fn map_account(row: &Row) -> Result<Account> {
        let account_kind_str: String = row.get(4)?;
        let provider_type_str: String = row.get(5)?;
        Ok(Account {
            id: row.get(0)?,
            instagram_user_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            account_kind: AccountKind::from_str(&account_kind_str),
            provider_type: ProviderType::from_str(&provider_type_str),
            avatar_url: row.get(6)?,
            is_private: row.get::<_, i32>(7)? != 0,
            is_verified: row.get::<_, i32>(8)? != 0,
            followers_count: row.get(9)?,
            following_count: row.get(10)?,
            monitoring_enabled: row.get::<_, i32>(11)? != 0,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
            last_successful_sync_at: row.get(14)?,
            last_attempted_sync_at: row.get(15)?,
            authenticated_by_account_id: row.get(16)?,
            access_state: row.get(17)?,
            access_reason: row.get(18)?,
            target_privacy: row.get(19)?,
            last_access_checked_at: row.get(20)?,
        })
    }

    pub fn upsert_account(&self, account: &Account) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO accounts (
                id, instagram_user_id, username, display_name, account_kind, provider_type,
                avatar_url, is_private, is_verified, followers_count, following_count,
                monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            ON CONFLICT(id) DO UPDATE SET
                instagram_user_id = coalesce(excluded.instagram_user_id, accounts.instagram_user_id),
                username = excluded.username,
                display_name = excluded.display_name,
                account_kind = excluded.account_kind,
                provider_type = excluded.provider_type,
                avatar_url = coalesce(excluded.avatar_url, accounts.avatar_url),
                is_private = excluded.is_private,
                is_verified = excluded.is_verified,
                followers_count = excluded.followers_count,
                following_count = excluded.following_count,
                monitoring_enabled = excluded.monitoring_enabled,
                updated_at = excluded.updated_at,
                last_successful_sync_at = coalesce(excluded.last_successful_sync_at, accounts.last_successful_sync_at),
                last_attempted_sync_at = coalesce(excluded.last_attempted_sync_at, accounts.last_attempted_sync_at),
                authenticated_by_account_id = coalesce(excluded.authenticated_by_account_id, accounts.authenticated_by_account_id),
                access_state = excluded.access_state,
                access_reason = excluded.access_reason,
                target_privacy = excluded.target_privacy,
                last_access_checked_at = excluded.last_access_checked_at
            "#,
            params![
                account.id,
                account.instagram_user_id,
                account.username,
                account.display_name,
                account.account_kind.as_str(),
                account.provider_type.as_str(),
                account.avatar_url,
                account.is_private as i32,
                account.is_verified as i32,
                account.followers_count,
                account.following_count,
                account.monitoring_enabled as i32,
                account.created_at,
                account.updated_at,
                account.last_successful_sync_at,
                account.last_attempted_sync_at,
                account.authenticated_by_account_id,
                account.access_state,
                account.access_reason,
                account.target_privacy,
                account.last_access_checked_at,
            ],
        )?;

        // Record alias
        conn.execute(
            r#"
            INSERT INTO account_aliases (account_id, username, first_seen_at, last_seen_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(account_id, username) DO UPDATE SET last_seen_at = excluded.last_seen_at
            "#,
            params![account.id, account.username, account.updated_at, account.updated_at],
        )?;

        Ok(())
    }

    pub fn get_account(&self, id: &str) -> Result<Option<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, instagram_user_id, username, display_name, account_kind, provider_type,
                   avatar_url, is_private, is_verified, followers_count, following_count,
                   monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                   authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
            FROM accounts WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_account(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_account_by_username(&self, username: &str) -> Result<Option<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, instagram_user_id, username, display_name, account_kind, provider_type,
                   avatar_url, is_private, is_verified, followers_count, following_count,
                   monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                   authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
            FROM accounts WHERE username = ?1 COLLATE NOCASE
            "#,
        )?;

        let mut rows = stmt.query(params![username])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_account(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_account_by_instagram_id(&self, ig_id: &str) -> Result<Option<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, instagram_user_id, username, display_name, account_kind, provider_type,
                   avatar_url, is_private, is_verified, followers_count, following_count,
                   monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                   authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
            FROM accounts WHERE instagram_user_id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![ig_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_account(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, instagram_user_id, username, display_name, account_kind, provider_type,
                   avatar_url, is_private, is_verified, followers_count, following_count,
                   monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                   authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
            FROM accounts ORDER BY created_at ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| Self::map_account(row))?;
        let mut accounts = Vec::new();
        for a in rows {
            accounts.push(a?);
        }
        Ok(accounts)
    }

    pub fn list_owner_accounts(&self) -> Result<Vec<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, instagram_user_id, username, display_name, account_kind, provider_type,
                   avatar_url, is_private, is_verified, followers_count, following_count,
                   monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                   authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
            FROM accounts WHERE account_kind = 'owner' ORDER BY created_at ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| Self::map_account(row))?;
        let mut accounts = Vec::new();
        for a in rows {
            accounts.push(a?);
        }
        Ok(accounts)
    }

    pub fn list_monitored_accounts(&self, owner_account_id: Option<&str>) -> Result<Vec<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut accounts = Vec::new();
        if let Some(o) = owner_account_id {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, instagram_user_id, username, display_name, account_kind, provider_type,
                       avatar_url, is_private, is_verified, followers_count, following_count,
                       monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                       authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
                FROM accounts WHERE account_kind = 'monitored' AND (authenticated_by_account_id = ?1 OR authenticated_by_account_id IS NULL)
                ORDER BY created_at ASC
                "#,
            )?;
            let mut rows = stmt.query(params![o])?;
            while let Some(row) = rows.next()? {
                accounts.push(Self::map_account(row)?);
            }
        } else {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, instagram_user_id, username, display_name, account_kind, provider_type,
                       avatar_url, is_private, is_verified, followers_count, following_count,
                       monitoring_enabled, created_at, updated_at, last_successful_sync_at, last_attempted_sync_at,
                       authenticated_by_account_id, access_state, access_reason, target_privacy, last_access_checked_at
                FROM accounts WHERE account_kind = 'monitored' ORDER BY created_at ASC
                "#,
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                accounts.push(Self::map_account(row)?);
            }
        }
        Ok(accounts)
    }

    pub fn delete_account(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Account Sessions (Key A Encrypted) ---

    pub fn save_account_session(&self, session: &AccountSession) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO account_sessions (
                account_id, session_data_ciphertext, nonce, last_validated_at, is_healthy, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(account_id) DO UPDATE SET
                session_data_ciphertext = excluded.session_data_ciphertext,
                nonce = excluded.nonce,
                last_validated_at = excluded.last_validated_at,
                is_healthy = excluded.is_healthy,
                error_message = excluded.error_message
            "#,
            params![
                session.account_id,
                session.session_data_ciphertext,
                session.nonce,
                session.last_validated_at,
                session.is_healthy as i32,
                session.error_message,
            ],
        )?;
        Ok(())
    }

    pub fn get_account_session(&self, account_id: &str) -> Result<Option<AccountSession>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT account_id, session_data_ciphertext, nonce, last_validated_at, is_healthy, error_message
            FROM account_sessions WHERE account_id = ?1
            "#,
        )?;
        let mut rows = stmt.query(params![account_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(AccountSession {
                account_id: row.get(0)?,
                session_data_ciphertext: row.get(1)?,
                nonce: row.get(2)?,
                last_validated_at: row.get(3)?,
                is_healthy: row.get::<_, i32>(4)? != 0,
                error_message: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_account_session(&self, account_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM account_sessions WHERE account_id = ?1", params![account_id])?;
        Ok(())
    }

    pub fn update_session_health(
        &self,
        account_id: &str,
        is_healthy: bool,
        error_message: Option<&str>,
        validated_at: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE account_sessions
            SET is_healthy = ?1, error_message = ?2, last_validated_at = ?3
            WHERE account_id = ?4
            "#,
            params![is_healthy as i32, error_message, validated_at, account_id],
        )?;
        Ok(())
    }

    // --- People & Identity Resolution ---

    pub fn upsert_people_batch(
        &self,
        raw_members: &[RawMemberRecord],
        observed_at: i64,
    ) -> Result<Vec<Person>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let mut resolved_people = Vec::with_capacity(raw_members.len());
        // Within-batch cache: normalized username -> person id. Prevents
        // forking two rows when the same user appears twice in one payload
        // (e.g. overlapping export shards) with different id presence.
        let mut batch_cache: std::collections::HashMap<String, (String, i64)> =
            std::collections::HashMap::new();

        for raw in raw_members {
            let username_norm = raw.username.trim().to_string();
            let username_key = username_norm.to_lowercase();
            let ig_norm: Option<String> = raw
                .instagram_user_id
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            // Find existing person.
            // Premium rule: a stable Instagram numeric id is the ONLY join key
            // when present. Usernames get recycled by Instagram and must never
            // merge two different numeric ids. Username matching is used only
            // for id-less rows (official export archives carry no ids) and is
            // case-insensitive.
            let mut existing_id: Option<String> = None;
            let mut existing_first_seen: Option<i64> = None;

            if let Some(ref ig_id) = ig_norm {
                if let Some((cached_id, cached_first)) = batch_cache.get(&format!("id:{}", ig_id)) {
                    existing_id = Some(cached_id.clone());
                    existing_first_seen = Some(*cached_first);
                } else {
                    let mut stmt = tx.prepare("SELECT id, first_seen_at FROM people WHERE instagram_user_id = ?1")?;
                    let mut rows = stmt.query(params![ig_id])?;
                    if let Some(row) = rows.next()? {
                        existing_id = Some(row.get(0)?);
                        existing_first_seen = Some(row.get(1)?);
                    } else {
                        // ig miss → adopt an id-less row with the same username
                        // (export-then-session backfill) instead of forking a
                        // duplicate. Rows already carrying a DIFFERENT numeric
                        // id are never merged (recycled usernames).
                        let mut ustmt = tx.prepare(
                            "SELECT id, first_seen_at FROM people WHERE current_username = ?1 COLLATE NOCASE AND (instagram_user_id IS NULL OR instagram_user_id = '') LIMIT 1",
                        )?;
                        let mut urows = ustmt.query(params![username_norm])?;
                        if let Some(urow) = urows.next()? {
                            existing_id = Some(urow.get(0)?);
                            existing_first_seen = Some(urow.get(1)?);
                        }
                    }
                }
            } else if let Some((cached_id, cached_first)) = batch_cache.get(&format!("un:{}", username_key)) {
                existing_id = Some(cached_id.clone());
                existing_first_seen = Some(*cached_first);
            } else {
                let mut stmt = tx.prepare(
                    "SELECT id, first_seen_at FROM people WHERE current_username = ?1 COLLATE NOCASE",
                )?;
                let mut rows = stmt.query(params![username_norm])?;
                if let Some(row) = rows.next()? {
                    let found_id: String = row.get(0)?;
                    let found_first: i64 = row.get(1)?;
                    // Only reuse a username match when it cannot collide two
                    // different numeric identities: the stored row must itself
                    // be id-less (export-origin backfill path).
                    let mut chk = tx.prepare("SELECT instagram_user_id FROM people WHERE id = ?1")?;
                    let mut chk_rows = chk.query(params![found_id])?;
                    let stored_ig: Option<String> = if let Some(cr) = chk_rows.next()? {
                        cr.get(0)?
                    } else {
                        None
                    };
                    if stored_ig.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                        existing_id = Some(found_id);
                        existing_first_seen = Some(found_first);
                    }
                }
            }

            let person_id = existing_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let first_seen = existing_first_seen.unwrap_or(observed_at);

            tx.execute(
                r#"
                INSERT INTO people (
                    id, instagram_user_id, current_username, display_name, avatar_url,
                    is_verified, is_private, first_seen_at, last_seen_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    instagram_user_id = coalesce(excluded.instagram_user_id, people.instagram_user_id),
                    current_username = excluded.current_username,
                    display_name = coalesce(excluded.display_name, people.display_name),
                    avatar_url = coalesce(excluded.avatar_url, people.avatar_url),
                    is_verified = excluded.is_verified,
                    is_private = excluded.is_private,
                    last_seen_at = excluded.last_seen_at
                "#,
                params![
                    person_id,
                    ig_norm,
                    username_norm,
                    raw.display_name,
                    raw.avatar_url,
                    raw.is_verified as i32,
                    raw.is_private as i32,
                    first_seen,
                    observed_at,
                ],
            )?;

            if let Some(ref ig_id) = ig_norm {
                batch_cache.insert(format!("id:{}", ig_id), (person_id.clone(), first_seen));
            }
            batch_cache.insert(format!("un:{}", username_key), (person_id.clone(), first_seen));

            resolved_people.push(Person {
                id: person_id,
                instagram_user_id: ig_norm,
                current_username: username_norm,
                display_name: raw.display_name.clone(),
                avatar_url: raw.avatar_url.clone(),
                is_verified: raw.is_verified,
                is_private: raw.is_private,
                first_seen_at: first_seen,
                last_seen_at: observed_at,
            });
        }

        tx.commit()?;
        Ok(resolved_people)
    }

    pub fn get_person_by_username(&self, username: &str) -> Result<Option<Person>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, instagram_user_id, current_username, display_name, avatar_url,
                   is_verified, is_private, first_seen_at, last_seen_at
            FROM people WHERE current_username = ?1 COLLATE NOCASE
            "#,
        )?;

        let mut rows = stmt.query(params![username])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Person {
                id: row.get(0)?,
                instagram_user_id: row.get(1)?,
                current_username: row.get(2)?,
                display_name: row.get(3)?,
                avatar_url: row.get(4)?,
                is_verified: row.get::<_, i32>(5)? != 0,
                is_private: row.get::<_, i32>(6)? != 0,
                first_seen_at: row.get(7)?,
                last_seen_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    // --- Snapshots ---

    pub fn insert_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO snapshots (
                id, account_id, snapshot_type, started_at, completed_at,
                status, item_count, source_hash, error_message, authenticated_by_account_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                snapshot.id,
                snapshot.account_id,
                snapshot.snapshot_type.as_str(),
                snapshot.started_at,
                snapshot.completed_at,
                snapshot.status.as_str(),
                snapshot.item_count,
                snapshot.source_hash,
                snapshot.error_message,
                snapshot.authenticated_by_account_id,
            ],
        )?;
        Ok(())
    }

    pub fn update_snapshot_status(
        &self,
        snapshot_id: &str,
        status: SnapshotStatus,
        completed_at: i64,
        item_count: i64,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE snapshots
            SET status = ?1, completed_at = ?2, item_count = ?3, error_message = ?4
            WHERE id = ?5
            "#,
            params![
                status.as_str(),
                completed_at,
                item_count,
                error_message,
                snapshot_id
            ],
        )?;
        Ok(())
    }

    pub fn insert_snapshot_members(&self, snapshot_id: &str, members: &[Person]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO snapshot_members (
                    snapshot_id, person_id, username_at_snapshot, display_name_at_snapshot, avatar_url_at_snapshot
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(snapshot_id, person_id) DO NOTHING
                "#,
            )?;

            for p in members {
                stmt.execute(params![
                    snapshot_id,
                    p.id,
                    p.current_username,
                    p.display_name,
                    p.avatar_url
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_latest_complete_snapshot(
        &self,
        account_id: &str,
        snapshot_type: SnapshotType,
        authenticated_by_account_id: Option<&str>,
    ) -> Result<Option<Snapshot>> {
        let conn = self.conn.lock().unwrap();
        let (query, p_auth) = match authenticated_by_account_id {
            Some(auth_id) => (
                r#"
                SELECT id, account_id, snapshot_type, started_at, completed_at,
                       status, item_count, source_hash, error_message, authenticated_by_account_id
                FROM snapshots
                WHERE account_id = ?1 AND snapshot_type = ?2 AND status = 'complete'
                  AND (authenticated_by_account_id = ?3 OR authenticated_by_account_id IS NULL)
                ORDER BY completed_at DESC, ROWID DESC LIMIT 1
                "#,
                Some(auth_id),
            ),
            None => (
                r#"
                SELECT id, account_id, snapshot_type, started_at, completed_at,
                        status, item_count, source_hash, error_message, authenticated_by_account_id
                FROM snapshots
                WHERE account_id = ?1 AND snapshot_type = ?2 AND status = 'complete'
                ORDER BY completed_at DESC, ROWID DESC LIMIT 1
                "#,
                None,
            ),
        };

        let mut stmt = conn.prepare(query)?;
        let mut rows = if let Some(auth_id) = p_auth {
            stmt.query(params![account_id, snapshot_type.as_str(), auth_id])?
        } else {
            stmt.query(params![account_id, snapshot_type.as_str()])?
        };

        if let Some(row) = rows.next()? {
            let snap_type_str: String = row.get(2)?;
            let status_str: String = row.get(5)?;
            Ok(Some(Snapshot {
                id: row.get(0)?,
                account_id: row.get(1)?,
                snapshot_type: SnapshotType::from_str(&snap_type_str),
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                status: SnapshotStatus::from_str(&status_str),
                item_count: row.get(6)?,
                source_hash: row.get(7)?,
                error_message: row.get(8)?,
                authenticated_by_account_id: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Newest-first complete snapshots of one type (for multi-cycle event
    /// confirmation). `limit` caps the history depth walked per sync.
    pub fn get_complete_snapshots(
        &self,
        account_id: &str,
        snapshot_type: SnapshotType,
        authenticated_by_account_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Snapshot>> {
        let conn = self.conn.lock().unwrap();
        let (query, p_auth) = match authenticated_by_account_id {
            Some(auth_id) => (
                r#"
                SELECT id, account_id, snapshot_type, started_at, completed_at,
                       status, item_count, source_hash, error_message, authenticated_by_account_id
                FROM snapshots
                WHERE account_id = ?1 AND snapshot_type = ?2 AND status = 'complete'
                  AND (authenticated_by_account_id = ?3 OR authenticated_by_account_id IS NULL)
                ORDER BY completed_at DESC, ROWID DESC LIMIT ?4
                "#,
                Some(auth_id),
            ),
            None => (
                r#"
                SELECT id, account_id, snapshot_type, started_at, completed_at,
                       status, item_count, source_hash, error_message, authenticated_by_account_id
                FROM snapshots
                WHERE account_id = ?1 AND snapshot_type = ?2 AND status = 'complete'
                ORDER BY completed_at DESC, ROWID DESC LIMIT ?3
                "#,
                None,
            ),
        };

        let mut stmt = conn.prepare(query)?;
        let mut rows = if let Some(auth_id) = p_auth {
            stmt.query(params![account_id, snapshot_type.as_str(), auth_id, limit])?
        } else {
            stmt.query(params![account_id, snapshot_type.as_str(), limit])?
        };

        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let snap_type_str: String = row.get(2)?;
            let status_str: String = row.get(5)?;
            out.push(Snapshot {
                id: row.get(0)?,
                account_id: row.get(1)?,
                snapshot_type: SnapshotType::from_str(&snap_type_str),
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                status: SnapshotStatus::from_str(&status_str),
                item_count: row.get(6)?,
                source_hash: row.get(7)?,
                error_message: row.get(8)?,
                authenticated_by_account_id: row.get(9)?,
            });
        }
        Ok(out)
    }

    // --- Pending (unconfirmed candidate) events ---

    pub fn upsert_pending_event(&self, pending: &PendingEvent) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO pending_events (
                id, account_id, person_id, related_username, change_type,
                first_seen_at, first_snapshot_id, authenticated_by_account_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(account_id, person_id, change_type) DO NOTHING
            "#,
            params![
                pending.id,
                pending.account_id,
                pending.person_id,
                pending.related_username,
                pending.change_type.as_str(),
                pending.first_seen_at,
                pending.first_snapshot_id,
                pending.authenticated_by_account_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_pending_events(&self, account_id: &str) -> Result<Vec<PendingEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, account_id, person_id, related_username, change_type,
                   first_seen_at, first_snapshot_id, authenticated_by_account_id
            FROM pending_events WHERE account_id = ?1
            "#,
        )?;
        let rows = stmt.query_map(params![account_id], |row| {
            let ct: String = row.get(4)?;
            Ok(PendingEvent {
                id: row.get(0)?,
                account_id: row.get(1)?,
                person_id: row.get(2)?,
                related_username: row.get(3)?,
                change_type: ChangeType::from_str(&ct),
                first_seen_at: row.get(5)?,
                first_snapshot_id: row.get(6)?,
                authenticated_by_account_id: row.get(7)?,
            })
        })?;

        let mut out = Vec::new();
        for p in rows {
            out.push(p?);
        }
        Ok(out)
    }

    /// Take (fetch + delete) the pending event for one person+direction, if
    /// any exists. Returns it so the caller can promote it to a real change.
    pub fn take_pending_for_person(
        &self,
        account_id: &str,
        person_id: &str,
        change_type: &ChangeType,
    ) -> Result<Option<PendingEvent>> {
        let existing = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                r#"
                SELECT id, account_id, person_id, related_username, change_type,
                       first_seen_at, first_snapshot_id, authenticated_by_account_id
                FROM pending_events
                WHERE account_id = ?1 AND person_id = ?2 AND change_type = ?3
                "#,
            )?;
            let mut rows = stmt.query(params![account_id, person_id, change_type.as_str()])?;
            if let Some(row) = rows.next()? {
                let ct: String = row.get(4)?;
                Some(PendingEvent {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    person_id: row.get(2)?,
                    related_username: row.get(3)?,
                    change_type: ChangeType::from_str(&ct),
                    first_seen_at: row.get(5)?,
                    first_snapshot_id: row.get(6)?,
                    authenticated_by_account_id: row.get(7)?,
                })
            } else {
                None
            }
        };
        if existing.is_some() {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "DELETE FROM pending_events WHERE account_id = ?1 AND person_id = ?2 AND change_type = ?3",
                params![account_id, person_id, change_type.as_str()],
            )?;
        }
        Ok(existing)
    }

    pub fn delete_pending_for_person(
        &self,
        account_id: &str,
        person_id: &str,
        change_type: &ChangeType,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM pending_events WHERE account_id = ?1 AND person_id = ?2 AND change_type = ?3",
            params![account_id, person_id, change_type.as_str()],
        )?;
        Ok(())
    }

    pub fn purge_old_pending_events(&self, older_than_ts: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(
            "DELETE FROM pending_events WHERE first_seen_at < ?1",
            params![older_than_ts],
        )?)
    }

    pub fn get_snapshot_members(&self, snapshot_id: &str) -> Result<Vec<SnapshotMember>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT snapshot_id, person_id, username_at_snapshot, display_name_at_snapshot, avatar_url_at_snapshot
            FROM snapshot_members WHERE snapshot_id = ?1
            "#,
        )?;

        let rows = stmt.query_map(params![snapshot_id], |row| {
            Ok(SnapshotMember {
                snapshot_id: row.get(0)?,
                person_id: row.get(1)?,
                username_at_snapshot: row.get(2)?,
                display_name_at_snapshot: row.get(3)?,
                avatar_url_at_snapshot: row.get(4)?,
            })
        })?;

        let mut members = Vec::new();
        for m in rows {
            members.push(m?);
        }
        Ok(members)
    }

    // --- Relationship State ---

    pub fn upsert_relationship_state_batch(
        &self,
        account_id: &str,
        states: &[RelationshipState],
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO relationship_state (
                    account_id, person_id, is_follower, is_following, first_observed_at, last_observed_at,
                    authenticated_by_account_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(account_id, person_id) DO UPDATE SET
                    is_follower = excluded.is_follower,
                    is_following = excluded.is_following,
                    last_observed_at = excluded.last_observed_at,
                    authenticated_by_account_id = coalesce(excluded.authenticated_by_account_id, relationship_state.authenticated_by_account_id)
                "#,
            )?;

            for s in states {
                stmt.execute(params![
                    account_id,
                    s.person_id,
                    s.is_follower as i32,
                    s.is_following as i32,
                    s.first_observed_at,
                    s.last_observed_at,
                    s.authenticated_by_account_id,
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Delete stale `relationship_state` rows that are no longer in the
    /// current follower∪following union. Without this, unfollowed people
    /// linger as ghosts and counts drift upward forever.
    /// Implemented via a temp keep-table so ANY list size works in a single
    /// correct pass (a chunked `NOT IN` would delete each chunk's complement).
    /// Never fails a sync: callers ignore the result with `let _ =`.
    pub fn prune_relationship_state(
        &self,
        account_id: &str,
        keep_person_ids: &[String],
    ) -> Result<usize> {
        if keep_person_ids.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TEMPORARY TABLE IF NOT EXISTS _prune_keep (person_id TEXT PRIMARY KEY);",
        )?;
        conn.execute("DELETE FROM _prune_keep", [])?;
        {
            let mut stmt = conn.prepare("INSERT OR IGNORE INTO _prune_keep (person_id) VALUES (?1)")?;
            for id in keep_person_ids {
                stmt.execute(params![id])?;
            }
        }
        let pruned = conn.execute(
            "DELETE FROM relationship_state WHERE account_id = ?1 AND person_id NOT IN (SELECT person_id FROM _prune_keep)",
            params![account_id],
        )?;
        Ok(pruned)
    }

    pub fn get_relationship_counts(
        &self,
        account_id: &str,
        authenticated_by_account_id: Option<&str>,
    ) -> Result<(i64, i64, i64, i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from(
            r#"
            SELECT
                COUNT(CASE WHEN is_follower = 1 THEN 1 END) AS followers,
                COUNT(CASE WHEN is_following = 1 THEN 1 END) AS following,
                COUNT(CASE WHEN is_follower = 1 AND is_following = 1 THEN 1 END) AS mutual,
                COUNT(CASE WHEN is_following = 1 AND is_follower = 0 THEN 1 END) AS not_following_back,
                COUNT(CASE WHEN is_follower = 1 AND is_following = 0 THEN 1 END) AS fans
            FROM relationship_state
            WHERE account_id = ?1
            "#,
        );

        if authenticated_by_account_id.is_some() {
            query.push_str(" AND (authenticated_by_account_id = ?2 OR authenticated_by_account_id IS NULL)");
        }

        let mut stmt = conn.prepare(&query)?;
        let mut rows = if let Some(auth_id) = authenticated_by_account_id {
            stmt.query(params![account_id, auth_id])?
        } else {
            stmt.query(params![account_id])?
        };

        if let Some(row) = rows.next()? {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        } else {
            Ok((0, 0, 0, 0, 0))
        }
    }

    pub fn get_people_count(
        &self,
        account_id: &str,
        authenticated_by_account_id: Option<&str>,
        filter_type: &str,
        search_query: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from(
            r#"
            SELECT COUNT(*)
            FROM relationship_state r
            JOIN people p ON p.id = r.person_id
            WHERE r.account_id = ?1
            "#,
        );

        if authenticated_by_account_id.is_some() {
            query.push_str(" AND (r.authenticated_by_account_id = ?2 OR r.authenticated_by_account_id IS NULL)");
        }

        match filter_type {
            "mutual" => query.push_str(" AND r.is_follower = 1 AND r.is_following = 1"),
            "fans" => query.push_str(" AND r.is_follower = 1 AND r.is_following = 0"),
            "not_following_back" => query.push_str(" AND r.is_following = 1 AND r.is_follower = 0"),
            "following" => query.push_str(" AND r.is_following = 1"),
            "followers" => query.push_str(" AND r.is_follower = 1"),
            _ => {}
        }

        if let Some(s) = search_query {
            let clean = s.trim().trim_start_matches('@');
            if !clean.is_empty() {
                let escaped = clean.replace('\'', "''");
                query.push_str(&format!(
                    " AND (p.current_username LIKE '%{}%' OR p.display_name LIKE '%{}%')",
                    escaped,
                    escaped
                ));
            }
        }

        let mut stmt = conn.prepare(&query)?;
        let count: i64 = if let Some(auth_id) = authenticated_by_account_id {
            stmt.query_row(params![account_id, auth_id], |row| row.get(0))?
        } else {
            stmt.query_row(params![account_id], |row| row.get(0))?
        };
        Ok(count)
    }

    // --- People Paginated Query ---

    pub fn get_people_paginated(
        &self,
        account_id: &str,
        authenticated_by_account_id: Option<&str>,
        filter_type: &str, // 'all', 'mutual', 'fans', 'not_following_back'
        search_query: Option<&str>,
        sort_by: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PersonListItem>> {
        let conn = self.conn.lock().unwrap();

        let mut query = String::from(
            r#"
            SELECT
                p.id,
                p.instagram_user_id,
                p.current_username,
                p.display_name,
                p.avatar_url,
                p.is_verified,
                p.is_private,
                r.is_follower,
                r.is_following,
                p.last_seen_at,
                (SELECT COUNT(*) FROM notes n WHERE n.account_id = r.account_id AND n.person_id = p.id) AS note_count,
                rc.change_type,
                rc.detected_at
            FROM relationship_state r
            JOIN people p ON p.id = r.person_id
            LEFT JOIN (
                SELECT rc1.person_id, rc1.change_type, rc1.detected_at
                FROM relationship_changes rc1
                INNER JOIN (
                    SELECT person_id, MAX(detected_at) AS max_detected
                    FROM relationship_changes
                    WHERE account_id = ?1
                    GROUP BY person_id
                ) rc2 ON rc1.person_id = rc2.person_id AND rc1.detected_at = rc2.max_detected
                WHERE rc1.account_id = ?1
            ) rc ON rc.person_id = p.id
            WHERE r.account_id = ?1
            "#,
        );

        if authenticated_by_account_id.is_some() {
            query.push_str(" AND (r.authenticated_by_account_id = ?4 OR r.authenticated_by_account_id IS NULL)");
        }

        match filter_type {
            "mutual" => query.push_str(" AND r.is_follower = 1 AND r.is_following = 1"),
            "fans" => query.push_str(" AND r.is_follower = 1 AND r.is_following = 0"),
            "not_following_back" => query.push_str(" AND r.is_following = 1 AND r.is_follower = 0"),
            "following" => query.push_str(" AND r.is_following = 1"),
            "followers" => query.push_str(" AND r.is_follower = 1"),
            _ => {}
        }

        if let Some(s) = search_query {
            let clean = s.trim().trim_start_matches('@');
            if !clean.is_empty() {
                let escaped = clean.replace('\'', "''");
                query.push_str(&format!(
                    " AND (p.current_username LIKE '%{}%' OR p.display_name LIKE '%{}%')",
                    escaped,
                    escaped
                ));
            }
        }

        let order_clause = match sort_by.unwrap_or("name_asc") {
            "mutual_first" => "ORDER BY (r.is_follower = 1 AND r.is_following = 1) DESC, p.current_username COLLATE NOCASE ASC",
            "followers_first" => "ORDER BY r.is_follower DESC, p.current_username COLLATE NOCASE ASC",
            _ => "ORDER BY p.current_username COLLATE NOCASE ASC",
        };

        query.push_str(&format!(" {} LIMIT ?2 OFFSET ?3", order_clause));

        let mut stmt = conn.prepare(&query)?;
        let mut items = Vec::new();
        if let Some(auth_id) = authenticated_by_account_id {
            let mut rows = stmt.query(params![account_id, limit, offset, auth_id])?;
            while let Some(row) = rows.next()? {
                items.push(Self::map_person_list_item(row)?);
            }
        } else {
            let mut rows = stmt.query(params![account_id, limit, offset])?;
            while let Some(row) = rows.next()? {
                items.push(Self::map_person_list_item(row)?);
            }
        }

        for item in &mut items {
            let mut tag_stmt = conn.prepare(
                r#"
                SELECT t.id, t.name, t.color
                FROM person_tags pt
                JOIN tags t ON t.id = pt.tag_id
                WHERE pt.account_id = ?1 AND pt.person_id = ?2
                "#,
            )?;
            let tag_rows = tag_stmt.query_map(params![account_id, item.id], |tr| {
                Ok(Tag {
                    id: tr.get(0)?,
                    name: tr.get(1)?,
                    color: tr.get(2)?,
                })
            })?;
            for tag in tag_rows {
                item.tags.push(tag?);
            }
        }

        Ok(items)
    }

    pub fn get_target_overlap(
        &self,
        owner_id: &str,
        target_id: &str,
        limit: i64,
    ) -> Result<(i64, Vec<PersonListItem>)> {
        let conn = self.conn.lock().unwrap();

        let count_query = r#"
            SELECT COUNT(DISTINCT p.id)
            FROM people p
            JOIN relationship_state ro ON ro.person_id = p.id AND ro.account_id = ?1
            JOIN relationship_state rt ON rt.person_id = p.id AND rt.account_id = ?2
        "#;
        let mut count_stmt = conn.prepare(count_query)?;
        let count: i64 = count_stmt.query_row(params![owner_id, target_id], |row| row.get(0)).unwrap_or(0);

        let query = r#"
            SELECT DISTINCT
                p.id,
                p.instagram_user_id,
                p.current_username,
                p.display_name,
                p.avatar_url,
                p.is_verified,
                p.is_private,
                ro.is_follower,
                ro.is_following,
                p.last_seen_at,
                0 AS note_count,
                NULL AS change_type,
                NULL AS detected_at
            FROM people p
            JOIN relationship_state ro ON ro.person_id = p.id AND ro.account_id = ?1
            JOIN relationship_state rt ON rt.person_id = p.id AND rt.account_id = ?2
            LIMIT ?3
        "#;
        let mut stmt = conn.prepare(query)?;
        let mut rows = stmt.query(params![owner_id, target_id, limit])?;
        let mut list = Vec::new();
        while let Some(row) = rows.next()? {
            list.push(Self::map_person_list_item(row)?);
        }
        Ok((count, list))
    }

    fn map_person_list_item(row: &Row) -> Result<PersonListItem> {
        let is_follower: bool = row.get::<_, i32>(7)? != 0;
        let is_following: bool = row.get::<_, i32>(8)? != 0;
        let note_count: i64 = row.get(10)?;
        let change_type_str: Option<String> = row.get(11)?;

        Ok(PersonListItem {
            id: row.get(0)?,
            instagram_user_id: row.get(1)?,
            username: row.get(2)?,
            display_name: row.get(3)?,
            avatar_url: row.get(4)?,
            is_verified: row.get::<_, i32>(5)? != 0,
            is_private: row.get::<_, i32>(6)? != 0,
            is_follower,
            is_following,
            is_mutual: is_follower && is_following,
            last_seen_at: row.get(9)?,
            tags: Vec::new(),
            has_note: note_count > 0,
            last_change_type: change_type_str,
            last_change_at: row.get(12)?,
        })
    }

    // --- Relationship Changes ---

    pub fn insert_relationship_changes(&self, changes: &[RelationshipChange]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO relationship_changes (
                    id, account_id, person_id, related_username, change_type,
                    confidence, metadata_json, detected_at, before_snapshot_id, after_snapshot_id,
                    authenticated_by_account_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
            )?;

            for c in changes {
                stmt.execute(params![
                    c.id,
                    c.account_id,
                    c.person_id,
                    c.related_username,
                    c.change_type.as_str(),
                    c.confidence.as_str(),
                    c.metadata_json,
                    c.detected_at,
                    c.before_snapshot_id,
                    c.after_snapshot_id,
                    c.authenticated_by_account_id,
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_changes_count(
        &self,
        account_id: &str,
        authenticated_by_account_id: Option<&str>,
        filter_type: Option<&str>,
        search_query: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from(
            r#"
            SELECT COUNT(*)
            FROM relationship_changes rc
            LEFT JOIN people p ON p.id = rc.person_id
            WHERE rc.account_id = ?1
            "#,
        );

        let mut param_values: Vec<rusqlite::types::Value> = vec![account_id.to_string().into()];

        if let Some(auth_id) = authenticated_by_account_id {
            param_values.push(auth_id.to_string().into());
            let idx = param_values.len();
            query.push_str(&format!(" AND (rc.authenticated_by_account_id = ?{} OR rc.authenticated_by_account_id IS NULL)", idx));
        }

        if let Some(ft) = filter_type {
            match ft {
                "lost" | "unfollowed_you" => query.push_str(" AND rc.change_type = 'unfollowed_you'"),
                "gained" | "followed_you" => query.push_str(" AND rc.change_type = 'followed_you'"),
                "renamed" | "username_changed" => query.push_str(" AND rc.change_type = 'username_changed'"),
                "outbound" => query.push_str(" AND rc.change_type IN ('you_followed', 'you_unfollowed')"),
                _ => {}
            }
        }

        if let Some(q) = search_query {
            let clean_q = q.trim().trim_start_matches('@');
            if !clean_q.is_empty() {
                let escaped_q = clean_q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
                let like_pattern = format!("%{}%", escaped_q);
                param_values.push(like_pattern.into());
                let idx = param_values.len();
                query.push_str(&format!(
                    " AND (rc.related_username LIKE ?{0} ESCAPE '\\' OR p.display_name LIKE ?{0} ESCAPE '\\' OR rc.metadata_json LIKE ?{0} ESCAPE '\\')",
                    idx
                ));
            }
        }

        let mut stmt = conn.prepare(&query)?;
        let count: i64 = stmt.query_row(rusqlite::params_from_iter(param_values), |r| r.get(0))?;
        Ok(count)
    }

    pub fn get_changes_feed(
        &self,
        account_id: &str,
        authenticated_by_account_id: Option<&str>,
        filter_type: Option<&str>,
        search_query: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChangeFeedItem>> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from(
            r#"
            SELECT
                rc.id, rc.account_id, rc.person_id, rc.related_username,
                p.display_name, p.avatar_url, rc.change_type, rc.confidence,
                rc.metadata_json, rc.detected_at, rc.before_snapshot_id, rc.after_snapshot_id
            FROM relationship_changes rc
            LEFT JOIN people p ON p.id = rc.person_id
            WHERE rc.account_id = ?1
            "#,
        );

        let mut param_values: Vec<rusqlite::types::Value> = vec![account_id.to_string().into()];

        if let Some(auth_id) = authenticated_by_account_id {
            param_values.push(auth_id.to_string().into());
            let idx = param_values.len();
            query.push_str(&format!(" AND (rc.authenticated_by_account_id = ?{} OR rc.authenticated_by_account_id IS NULL)", idx));
        }

        if let Some(ft) = filter_type {
            match ft {
                "lost" | "unfollowed_you" => query.push_str(" AND rc.change_type = 'unfollowed_you'"),
                "gained" | "followed_you" => query.push_str(" AND rc.change_type = 'followed_you'"),
                "renamed" | "username_changed" => query.push_str(" AND rc.change_type = 'username_changed'"),
                "outbound" => query.push_str(" AND rc.change_type IN ('you_followed', 'you_unfollowed')"),
                _ => {}
            }
        }

        if let Some(q) = search_query {
            let clean_q = q.trim().trim_start_matches('@');
            if !clean_q.is_empty() {
                let escaped_q = clean_q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
                let like_pattern = format!("%{}%", escaped_q);
                param_values.push(like_pattern.into());
                let idx = param_values.len();
                query.push_str(&format!(
                    " AND (rc.related_username LIKE ?{0} ESCAPE '\\' OR p.display_name LIKE ?{0} ESCAPE '\\' OR rc.metadata_json LIKE ?{0} ESCAPE '\\')",
                    idx
                ));
            }
        }

        param_values.push(limit.into());
        let limit_idx = param_values.len();
        param_values.push(offset.into());
        let offset_idx = param_values.len();

        query.push_str(&format!(" ORDER BY rc.detected_at DESC LIMIT ?{} OFFSET ?{}", limit_idx, offset_idx));

        let mut stmt = conn.prepare(&query)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(param_values))?;
        let mut feed = Vec::new();
        while let Some(row) = rows.next()? {
            feed.push(Self::map_change_feed_item(row)?);
        }
        Ok(feed)
    }

    fn map_change_feed_item(row: &Row) -> Result<ChangeFeedItem> {
        let change_type_str: String = row.get(6)?;
        let confidence_str: String = row.get(7)?;

        Ok(ChangeFeedItem {
            id: row.get(0)?,
            account_id: row.get(1)?,
            person_id: row.get(2)?,
            related_username: row.get(3)?,
            display_name: row.get(4)?,
            avatar_url: row.get(5)?,
            change_type: ChangeType::from_str(&change_type_str),
            confidence: Confidence::from_str(&confidence_str),
            metadata_json: row.get(8)?,
            detected_at: row.get(9)?,
            before_snapshot_id: row.get(10)?,
            after_snapshot_id: row.get(11)?,
        })
    }

    // --- Encrypted Notes (Key B) ---

    pub fn upsert_note_encrypted(&self, note: &NoteRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO notes (id, account_id, person_id, content_ciphertext, nonce, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(account_id, person_id) DO UPDATE SET
                content_ciphertext = excluded.content_ciphertext,
                nonce = excluded.nonce,
                updated_at = excluded.updated_at
            "#,
            params![
                note.id,
                note.account_id,
                note.person_id,
                note.content_ciphertext,
                note.nonce,
                note.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_note_encrypted(&self, account_id: &str, person_id: &str) -> Result<Option<NoteRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, account_id, person_id, content_ciphertext, nonce, updated_at
            FROM notes WHERE account_id = ?1 AND person_id = ?2
            "#,
        )?;

        let mut rows = stmt.query(params![account_id, person_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(NoteRecord {
                id: row.get(0)?,
                account_id: row.get(1)?,
                person_id: row.get(2)?,
                content_ciphertext: row.get(3)?,
                nonce: row.get(4)?,
                updated_at: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_note(&self, account_id: &str, person_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM notes WHERE account_id = ?1 AND person_id = ?2",
            params![account_id, person_id],
        )?;
        Ok(())
    }

    // --- Tags ---

    pub fn create_tag(&self, id: &str, name: &str, color: Option<&str>) -> Result<Tag> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tags (id, name, color) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET color = excluded.color",
            params![id, name, color],
        )?;
        Ok(Tag {
            id: id.to_string(),
            name: name.to_string(),
            color: color.map(|s| s.to_string()),
        })
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, color FROM tags ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?;

        let mut tags = Vec::new();
        for t in rows {
            tags.push(t?);
        }
        Ok(tags)
    }

    pub fn add_tag_to_person(&self, account_id: &str, person_id: &str, tag_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO person_tags (account_id, person_id, tag_id) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
            params![account_id, person_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_tag_from_person(&self, account_id: &str, person_id: &str, tag_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM person_tags WHERE account_id = ?1 AND person_id = ?2 AND tag_id = ?3",
            params![account_id, person_id, tag_id],
        )?;
        Ok(())
    }

    // --- Settings ---

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AccountKind, ProviderType, ChangeType, Confidence};

    #[test]
    fn test_legacy_database_migration() {
        let db_path = std::env::temp_dir().join(format!("legacy_stalkr_{}.db", uuid::Uuid::new_v4()));

        // Simulate an older database without authenticated_by_account_id
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(r#"
                CREATE TABLE accounts (
                    id TEXT PRIMARY KEY NOT NULL,
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
                    last_attempted_sync_at INTEGER
                );
                CREATE INDEX idx_accounts_username ON accounts(username);
            "#).unwrap();
        }

        // Now open with Database::new, which runs CREATE_TABLES_SQL, run_migrations, and CREATE_INDICES_SQL
        let db = Database::new(&db_path).expect("Migration must succeed on legacy schema");
        let accounts = db.list_accounts().expect("Must be able to list accounts after migration");
        assert_eq!(accounts.len(), 0);
    }

    #[test]
    fn test_account_crud_and_isolation() {
        let db_path = std::env::temp_dir().join(format!("test_acc_{}.db", uuid::Uuid::new_v4()));
        let db = Database::new(&db_path).unwrap();
        let now = chrono::Utc::now().timestamp();

        // 1. Create Owner Account
        let owner = Account {
            id: "owner_1".to_string(),
            instagram_user_id: Some("1111".to_string()),
            username: "main_user".to_string(),
            display_name: "Main User".to_string(),
            account_kind: AccountKind::Owner,
            provider_type: ProviderType::Session,
            avatar_url: None,
            is_private: false,
            is_verified: true,
            followers_count: 500,
            following_count: 300,
            monitoring_enabled: true,
            created_at: now,
            updated_at: now,
            last_successful_sync_at: None,
            last_attempted_sync_at: None,
            authenticated_by_account_id: None,
            access_state: "accessible".to_string(),
            access_reason: None,
            target_privacy: "public".to_string(),
            last_access_checked_at: Some(now),
        };
        db.upsert_account(&owner).unwrap();

        // 2. Create Target Account monitored by owner
        let target = Account {
            id: "target_1".to_string(),
            instagram_user_id: Some("2222".to_string()),
            username: "tracked_target".to_string(),
            display_name: "Tracked Target".to_string(),
            account_kind: AccountKind::Monitored,
            provider_type: ProviderType::Session,
            avatar_url: None,
            is_private: true,
            is_verified: false,
            followers_count: 120,
            following_count: 80,
            monitoring_enabled: true,
            created_at: now,
            updated_at: now,
            last_successful_sync_at: None,
            last_attempted_sync_at: None,
            authenticated_by_account_id: Some("owner_1".to_string()),
            access_state: "accessible".to_string(),
            access_reason: None,
            target_privacy: "private".to_string(),
            last_access_checked_at: Some(now),
        };
        db.upsert_account(&target).unwrap();

        // 3. Verify accounts listed
        let all_accounts = db.list_accounts().unwrap();
        assert_eq!(all_accounts.len(), 2);

        let retrieved_owner = db.get_account("owner_1").unwrap().unwrap();
        assert_eq!(retrieved_owner.username, "main_user");

        let retrieved_by_uname = db.get_account_by_username("tracked_target").unwrap().unwrap();
        assert_eq!(retrieved_by_uname.id, "target_1");
        assert_eq!(retrieved_by_uname.authenticated_by_account_id.as_deref(), Some("owner_1"));

        // 4. Delete target, verify owner remains intact
        db.delete_account("target_1").unwrap();
        let remaining = db.list_accounts().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "owner_1");
    }

    fn create_test_account(db: &Database, id: &str) {
        let now = chrono::Utc::now().timestamp();
        let account = Account {
            id: id.to_string(),
            instagram_user_id: Some(format!("ig_{}", id)),
            username: format!("user_{}", id),
            display_name: format!("User {}", id),
            account_kind: AccountKind::Owner,
            provider_type: ProviderType::Session,
            avatar_url: None,
            is_private: false,
            is_verified: false,
            followers_count: 10,
            following_count: 10,
            monitoring_enabled: true,
            created_at: now,
            updated_at: now,
            last_successful_sync_at: None,
            last_attempted_sync_at: None,
            authenticated_by_account_id: None,
            access_state: "accessible".to_string(),
            access_reason: None,
            target_privacy: "public".to_string(),
            last_access_checked_at: Some(now),
        };
        db.upsert_account(&account).unwrap();
    }

    #[test]
    fn test_people_paginated_and_counts_with_filters() {
        let db_path = std::env::temp_dir().join(format!("test_people_{}.db", uuid::Uuid::new_v4()));
        let db = Database::new(&db_path).unwrap();
        let now = chrono::Utc::now().timestamp();
        create_test_account(&db, "acc_1");

        let members = vec![
            RawMemberRecord {
                instagram_user_id: Some("m1".to_string()),
                username: "alice_crypto".to_string(),
                display_name: Some("Alice Crypto".to_string()),
                avatar_url: None,
                is_private: false,
                is_verified: false,
            },
            RawMemberRecord {
                instagram_user_id: Some("m2".to_string()),
                username: "bob_dev".to_string(),
                display_name: Some("Bob Developer".to_string()),
                avatar_url: None,
                is_private: true,
                is_verified: true,
            },
            RawMemberRecord {
                instagram_user_id: Some("m3".to_string()),
                username: "charlie_art".to_string(),
                display_name: Some("Charlie Artist".to_string()),
                avatar_url: None,
                is_private: false,
                is_verified: false,
            },
        ];

        // Upsert people
        let people = db.upsert_people_batch(&members, now).unwrap();
        assert_eq!(people.len(), 3);

        // Set relationship states
        let states = vec![
            RelationshipState {
                account_id: "acc_1".to_string(),
                person_id: people[0].id.clone(),
                is_follower: true,
                is_following: false,
                first_observed_at: now,
                last_observed_at: now,
                authenticated_by_account_id: None,
            },
            RelationshipState {
                account_id: "acc_1".to_string(),
                person_id: people[1].id.clone(),
                is_follower: true,
                is_following: true,
                first_observed_at: now,
                last_observed_at: now,
                authenticated_by_account_id: None,
            },
            RelationshipState {
                account_id: "acc_1".to_string(),
                person_id: people[2].id.clone(),
                is_follower: false,
                is_following: true,
                first_observed_at: now,
                last_observed_at: now,
                authenticated_by_account_id: None,
            },
        ];
        db.upsert_relationship_state_batch("acc_1", &states).unwrap();

        // Test counts
        let count_all = db.get_people_count("acc_1", None, "all", None).unwrap();
        assert_eq!(count_all, 3);

        let count_mutual = db.get_people_count("acc_1", None, "mutual", None).unwrap();
        assert_eq!(count_mutual, 1); // bob is both follower and following

        let count_followers = db.get_people_count("acc_1", None, "followers", None).unwrap();
        assert_eq!(count_followers, 2);

        let count_following = db.get_people_count("acc_1", None, "following", None).unwrap();
        assert_eq!(count_following, 2);

        // Test search filter
        let count_search = db.get_people_count("acc_1", None, "all", Some("crypto")).unwrap();
        assert_eq!(count_search, 1);

        let paginated = db.get_people_paginated("acc_1", None, "all", Some("crypto"), None, 10, 0).unwrap();
        assert_eq!(paginated.len(), 1);
        assert_eq!(paginated[0].username, "alice_crypto");
    }

    #[test]
    fn test_relationship_changes_pagination_all_time() {
        let db_path = std::env::temp_dir().join(format!("test_changes_{}.db", uuid::Uuid::new_v4()));
        let db = Database::new(&db_path).unwrap();
        let now = chrono::Utc::now().timestamp();
        create_test_account(&db, "acc_1");

        let people_raw = vec![
            RawMemberRecord {
                instagram_user_id: Some("ig_p1".to_string()),
                username: "alice".to_string(),
                display_name: Some("Alice".to_string()),
                avatar_url: None,
                is_private: false,
                is_verified: false,
            },
            RawMemberRecord {
                instagram_user_id: Some("ig_p2".to_string()),
                username: "bob".to_string(),
                display_name: Some("Bob".to_string()),
                avatar_url: None,
                is_private: false,
                is_verified: false,
            },
            RawMemberRecord {
                instagram_user_id: Some("ig_p3".to_string()),
                username: "claire".to_string(),
                display_name: Some("Claire".to_string()),
                avatar_url: None,
                is_private: false,
                is_verified: false,
            },
        ];
        let p_recs = db.upsert_people_batch(&people_raw, now).unwrap();

        let changes = vec![
            RelationshipChange {
                id: "c1".to_string(),
                account_id: "acc_1".to_string(),
                person_id: p_recs[0].id.clone(),
                related_username: "alice".to_string(),
                change_type: ChangeType::FollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 2, // 2h ago
                before_snapshot_id: None,
                after_snapshot_id: None,
                authenticated_by_account_id: None,
            },
            RelationshipChange {
                id: "c2".to_string(),
                account_id: "acc_1".to_string(),
                person_id: p_recs[1].id.clone(),
                related_username: "bob".to_string(),
                change_type: ChangeType::UnfollowedYou,
                confidence: Confidence::Confirmed,
                metadata_json: None,
                detected_at: now - 3600 * 72, // 3 days ago (outside 24h)
                before_snapshot_id: None,
                after_snapshot_id: None,
                authenticated_by_account_id: None,
            },
            RelationshipChange {
                id: "c3".to_string(),
                account_id: "acc_1".to_string(),
                person_id: p_recs[2].id.clone(),
                related_username: "claire".to_string(),
                change_type: ChangeType::UsernameChanged,
                confidence: Confidence::Confirmed,
                metadata_json: Some("{\"old\":\"claire_old\"}".to_string()),
                detected_at: now - 3600 * 300, // ~12 days ago
                before_snapshot_id: None,
                after_snapshot_id: None,
                authenticated_by_account_id: None,
            },
        ];

        db.insert_relationship_changes(&changes).unwrap();

        // All time count
        let total_count = db.get_changes_count("acc_1", None, None, None).unwrap();
        assert_eq!(total_count, 3);

        // Filtered count
        let unfollowed_count = db.get_changes_count("acc_1", None, Some("unfollowed_you"), None).unwrap();
        assert_eq!(unfollowed_count, 1);

        // Paginated all-time feed
        let all_changes = db.get_changes_feed("acc_1", None, None, None, 10, 0).unwrap();
        assert_eq!(all_changes.len(), 3);
        assert_eq!(all_changes[0].related_username, "alice"); // Latest first
        assert_eq!(all_changes[2].related_username, "claire"); // Oldest last
    }

    #[test]
    fn test_settings_and_encrypted_notes() {
        let db_path = std::env::temp_dir().join(format!("test_settings_{}.db", uuid::Uuid::new_v4()));
        let db = Database::new(&db_path).unwrap();
        let now = chrono::Utc::now().timestamp();
        create_test_account(&db, "acc_1");

        let target_raw = RawMemberRecord {
            instagram_user_id: Some("ig_target".to_string()),
            username: "target_person".to_string(),
            display_name: Some("Target Person".to_string()),
            avatar_url: None,
            is_private: false,
            is_verified: false,
        };
        let p_recs = db.upsert_people_batch(&[target_raw], now).unwrap();
        let target_pid = p_recs[0].id.clone();

        // 1. Settings get / set
        assert_eq!(db.get_setting("biometric_lock").unwrap(), None);
        db.set_setting("biometric_lock", "true").unwrap();
        assert_eq!(db.get_setting("biometric_lock").unwrap().as_deref(), Some("true"));

        // 2. Note upsert and read
        let note = NoteRecord {
            id: "note_1".to_string(),
            account_id: "acc_1".to_string(),
            person_id: target_pid.clone(),
            content_ciphertext: "encrypted_blob_test".to_string(),
            nonce: "test_nonce_12".to_string(),
            updated_at: now,
        };
        db.upsert_note_encrypted(&note).unwrap();

        let retrieved = db.get_note_encrypted("acc_1", &target_pid).unwrap().unwrap();
        assert_eq!(retrieved.content_ciphertext, "encrypted_blob_test");
        assert_eq!(retrieved.nonce, "test_nonce_12");

        // Delete note
        db.delete_note("acc_1", &target_pid).unwrap();
        assert_eq!(db.get_note_encrypted("acc_1", &target_pid).unwrap().is_none(), true);
    }
}
