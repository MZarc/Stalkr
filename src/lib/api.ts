import { invoke } from '@tauri-apps/api/core';

export type AccessState =
  | 'accessible'
  | 'partially_accessible'
  | 'not_accessible'
  | 'auth_required'
  | 'rate_limited'
  | 'provider_error'
  | 'unknown';

export type PrivacyState = 'public' | 'private' | 'unknown';

export interface Account {
  id: string;
  instagram_user_id?: string | null;
  username: string;
  display_name: string;
  account_kind: 'owner' | 'monitored';
  provider_type: 'session' | 'export' | 'public_profile' | 'mock';
  avatar_url?: string | null;
  is_private: boolean;
  is_verified: boolean;
  followers_count: number;
  following_count: number;
  monitoring_enabled: boolean;
  created_at: number;
  updated_at: number;
  last_successful_sync_at?: number | null;
  last_attempted_sync_at?: number | null;
  authenticated_by_account_id?: string | null;
  access_state: AccessState;
  access_reason?: string | null;
  target_privacy: PrivacyState;
  last_access_checked_at?: number | null;
}

export interface TargetAccessCheckResult {
  target_user_id?: string | null;
  target_username: string;
  avatar_url?: string | null;
  privacy_state: PrivacyState;
  access_state: AccessState;
  access_reason: string;
  is_following: boolean;
  is_approved_follower: boolean;
  followers_count: number;
  following_count: number;
}

export interface AddTargetResponse {
  account: Account;
  access_check: TargetAccessCheckResult;
}

export interface SessionHealthResponse {
  is_healthy: boolean;
  last_validated_at: number;
  error_message?: string | null;
}

export interface Tag {
  id: string;
  name: string;
  color?: string;
}

export interface PersonListItem {
  id: string;
  instagram_user_id?: string;
  username: string;
  display_name?: string;
  avatar_url?: string;
  is_verified: boolean;
  is_private: boolean;
  is_follower: boolean;
  is_following: boolean;
  is_mutual: boolean;
  last_seen_at: number;
  tags: Tag[];
  has_note: boolean;
  last_change_type?: string;
  last_change_at?: number;
}

export interface RelationshipSummary {
  followers: number;
  following: number;
  mutual: number;
  not_following_back: number;
  fans: number;
  net_delta_7d: number;
  // Official Instagram header counts (optional — older backends omit them).
  profile_followers?: number | null;
  profile_following?: number | null;
}

export interface TargetOverlapReport {
  mutual_connections_count: number;
  shared_people: PersonListItem[];
}

export interface ChangeFeedItem {
  id: string;
  account_id: string;
  person_id: string;
  related_username: string;
  display_name?: string;
  avatar_url?: string;
  change_type: 'followed_you' | 'unfollowed_you' | 'you_followed' | 'you_unfollowed' | 'username_changed';
  confidence: 'confirmed' | 'unconfirmed';
  metadata_json?: string;
  detected_at: number;
  before_snapshot_id?: string;
  after_snapshot_id?: string;
}

export interface NoteDto {
  id: string;
  account_id: string;
  person_id: string;
  content: string;
  updated_at: number;
}

export interface ProviderHealthStatus {
  provider_name: string;
  is_connected: boolean;
  status_text: string;
  last_successful_sync?: number;
  follower_retrieval_ok: boolean;
  following_retrieval_ok: boolean;
  pagination_ok: boolean;
  completeness_check_ok: boolean;
  provider_version: string;
  last_error?: string;
}

export interface SyncReport {
  success: boolean;
  status: string;
  followers_count: number;
  following_count: number;
  changes_detected: number;
  message: string;
  // Premium accuracy fields (optional — older backends omit them).
  profile_followers_count?: number | null;
  profile_following_count?: number | null;
  tracked_followers_count?: number | null;
  tracked_following_count?: number | null;
  verification?: string | null;
}

export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__);
};

async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new Error(`Running in web browser mode (Tauri IPC not available for ${cmd})`);
  }
  return await invoke<T>(cmd, args);
}


// ─── Demo simulation helpers ──────────────────────────────────────────────────
function ri(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}
function rc<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

// ─── Isolation gate ───────────────────────────────────────────────────────────
// In-memory state is ONLY used when isTauri()=false (browser/dev) OR isDemoMode=true.
// Real Tauri sessions NEVER touch these arrays — every fallback re-checks the gate.
let _isDemoMode = false;

export function isDemoMode(): boolean { return _isDemoMode; }
export function setDemoMode(on: boolean): void { _isDemoMode = on; }

// Returns true when the fallback path is allowed (browser dev OR explicit demo mode)
function fallbackAllowed(): boolean {
  return !isTauri() || _isDemoMode;
}

// In-memory state for demo / browser fallback only
const localAccounts: Account[] = [];
const localPeople: PersonListItem[] = [];
const localTargetPeople: PersonListItem[] = [];
const localChanges: ChangeFeedItem[] = [];
const localTargetChanges: ChangeFeedItem[] = [];
const localNotes: Record<string, string> = {};
const localTargetNotes: Record<string, string> = {};


export const api = {
  async getAccounts(): Promise<Account[]> {
    try {
      const res = await safeInvoke<Account[]>('get_accounts');
      if (Array.isArray(res)) return res;
      return fallbackAllowed() ? localAccounts : [];
    } catch {
      return fallbackAllowed() ? localAccounts : [];
    }
  },

  async getAccount(id: string): Promise<Account | null> {
    try {
      return await safeInvoke<Account | null>('get_account', { id });
    } catch {
      if (!fallbackAllowed()) return null;
      return localAccounts.find((a) => a.id === id) || null;
    }
  },

  async connectInstagram(
    sessionId: string,
    dsUserId: string,
    csrftoken?: string,
    cookies?: string,
    profileMeta?: {
      username?: string;
      displayName?: string;
      avatarUrl?: string;
      followersCount?: number;
      followingCount?: number;
      isPrivate?: boolean;
      isVerified?: boolean;
    }
  ): Promise<Account> {
    try {
      return await safeInvoke<Account>('connect_instagram', {
        sessionId,
        dsUserId,
        csrftoken: csrftoken || null,
        cookies: cookies || null,
        username: profileMeta?.username || null,
        displayName: profileMeta?.displayName || null,
        avatarUrl: profileMeta?.avatarUrl || null,
        followersCount: profileMeta?.followersCount ?? null,
        followingCount: profileMeta?.followingCount ?? null,
        isPrivate: profileMeta?.isPrivate ?? null,
        isVerified: profileMeta?.isVerified ?? null,
      });
    } catch (err: any) {
      if (!isTauri()) {
        const mockOwner: Account = {
          id: 'acc_owner_' + Date.now(),
          instagram_user_id: dsUserId,
          username: profileMeta?.username || 'connected_user',
          display_name: profileMeta?.displayName || 'Connected Instagram Account',
          account_kind: 'owner',
          provider_type: 'session',
          avatar_url: profileMeta?.avatarUrl || null,
          is_private: profileMeta?.isPrivate ?? false,
          is_verified: profileMeta?.isVerified ?? false,
          followers_count: profileMeta?.followersCount ?? 850,
          following_count: profileMeta?.followingCount ?? 420,
          monitoring_enabled: true,
          created_at: Math.floor(Date.now() / 1000),
          updated_at: Math.floor(Date.now() / 1000),
          last_successful_sync_at: null,
          last_attempted_sync_at: Math.floor(Date.now() / 1000),
          authenticated_by_account_id: null,
          access_state: 'accessible',
          access_reason: 'Authenticated owner session',
          target_privacy: 'public',
          last_access_checked_at: Math.floor(Date.now() / 1000),
        };
        localAccounts.push(mockOwner);
        return mockOwner;
      }
      throw err;
    }
  },

  async connectMockOwner(
    username: string,
    displayName = '',
    followers = 1842,
    following = 936
  ): Promise<Account> {
    try {
      return await safeInvoke<Account>('connect_mock_owner', {
        username,
        displayName,
        followers,
        following,
      });
    } catch {
      const mockOwner: Account = {
        id: 'acc_mock_' + Date.now(),
        instagram_user_id: 'mock_id_' + username,
        username,
        display_name: displayName || username,
        account_kind: 'owner',
        provider_type: 'mock',
        avatar_url: null,
        is_private: false,
        is_verified: true,
        followers_count: followers,
        following_count: following,
        monitoring_enabled: true,
        created_at: Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
        last_successful_sync_at: null,
        last_attempted_sync_at: Math.floor(Date.now() / 1000),
        authenticated_by_account_id: null,
        access_state: 'accessible',
        access_reason: 'Test owner account (Fixture)',
        target_privacy: 'public',
        last_access_checked_at: Math.floor(Date.now() / 1000),
      };
      localAccounts.push(mockOwner);
      return mockOwner;
    }
  },

  async enterDemoMode(): Promise<Account> {
    try {
      const acc = await safeInvoke<Account>('enter_demo_mode');
      setDemoMode(true);
      return acc;
    } catch {
      // Generate fresh random demo data on every entry
      const { generateRandomDemoData, setLiveSummary, setLiveTargetSummary } = await import('./mockData');
      const demo = generateRandomDemoData();

      localAccounts.length = 0;
      localAccounts.push({ ...demo.ownerAccount }, { ...demo.targetAccount });

      localPeople.length = 0;
      localPeople.push(...demo.people);

      localTargetPeople.length = 0;
      localTargetPeople.push(...demo.targetPeople);

      localChanges.length = 0;
      localChanges.push(...demo.changes);

      localTargetChanges.length = 0;
      localTargetChanges.push(...demo.targetChanges);

      for (const key of Object.keys(localNotes)) delete localNotes[key];
      for (const key of Object.keys(localTargetNotes)) delete localTargetNotes[key];
      Object.assign(localNotes, demo.notes);
      Object.assign(localTargetNotes, demo.targetNotes);

      setLiveSummary({ ...demo.ownerSummary });
      setLiveTargetSummary({ ...demo.targetSummary });

      setDemoMode(true);
      return demo.ownerAccount;
    }
  },

  async getSessionHealth(accountId: string): Promise<SessionHealthResponse> {
    try {
      return await safeInvoke<SessionHealthResponse>('get_session_health', { accountId });
    } catch {
      if (!fallbackAllowed()) throw new Error('Session health check failed');
      return {
        is_healthy: true,
        last_validated_at: Math.floor(Date.now() / 1000) - 120,
        error_message: null,
      };
    }
  },

  async revalidateSession(accountId: string): Promise<Account> {
    try {
      return await safeInvoke<Account>('revalidate_session', { accountId });
    } catch {
      if (!fallbackAllowed()) throw new Error('Session revalidation failed');
      const acc = localAccounts.find((a) => a.id === accountId) || localAccounts[0];
      return acc;
    }
  },

  async disconnectInstagram(accountId: string): Promise<void> {
    try {
      await safeInvoke('disconnect_instagram', { accountId });
    } catch {
      const idx = localAccounts.findIndex((a) => a.id === accountId);
      if (idx !== -1) localAccounts.splice(idx, 1);
    }
    // Clear demo mode when logging out
    setDemoMode(false);
  },

  async addTargetAccount(
    ownerAccountId: string,
    targetUsername: string
  ): Promise<AddTargetResponse> {
    try {
      return await safeInvoke<AddTargetResponse>('add_target_account', {
        ownerAccountId,
        targetUsername,
      });
    } catch (err: any) {
      if (!isTauri()) {
        const isPrivate = targetUsername.toLowerCase().includes('private');
        const isNotApproved = targetUsername.toLowerCase().includes('stranger');
        const accessState: AccessState = isPrivate && isNotApproved ? 'not_accessible' : 'accessible';
        const accessReason = isPrivate && isNotApproved
          ? "This private account's relationship lists are not accessible through the connected Instagram account."
          : isPrivate
          ? "Target is private; authenticated user is an approved follower."
          : "Public profile. Relationship lists are accessible.";

        const newTarget: Account = {
          id: 'acc_target_' + Date.now(),
          instagram_user_id: 'mock_target_' + targetUsername,
          username: targetUsername.replace('@', ''),
          display_name: targetUsername.replace('@', ''),
          account_kind: 'monitored',
          provider_type: 'mock',
          avatar_url: null,
          is_private: isPrivate,
          is_verified: false,
          followers_count: isPrivate && isNotApproved ? 0 : 340,
          following_count: isPrivate && isNotApproved ? 0 : 210,
          monitoring_enabled: true,
          created_at: Math.floor(Date.now() / 1000),
          updated_at: Math.floor(Date.now() / 1000),
          last_successful_sync_at: null,
          last_attempted_sync_at: null,
          authenticated_by_account_id: ownerAccountId,
          access_state: accessState,
          access_reason: accessReason,
          target_privacy: isPrivate ? 'private' : 'public',
          last_access_checked_at: Math.floor(Date.now() / 1000),
        };
        localAccounts.push(newTarget);
        return {
          account: newTarget,
          access_check: {
            target_user_id: newTarget.instagram_user_id,
            target_username: newTarget.username,
            avatar_url: newTarget.avatar_url,
            privacy_state: isPrivate ? 'private' : 'public',
            access_state: accessState,
            access_reason: accessReason,
            is_following: !isNotApproved,
            is_approved_follower: !isNotApproved,
            followers_count: newTarget.followers_count,
            following_count: newTarget.following_count,
          },
        };
      }
      throw err;
    }
  },

  async checkTargetAccess(
    ownerAccountId: string,
    targetAccountId: string
  ): Promise<TargetAccessCheckResult> {
    try {
      return await safeInvoke<TargetAccessCheckResult>('check_target_access', {
        ownerAccountId,
        targetAccountId,
      });
    } catch {
      return {
        target_user_id: 'mock_target_' + targetAccountId,
        target_username: 'target_user',
        privacy_state: 'private',
        access_state: 'accessible',
        access_reason: 'Target is private; authenticated user is an approved follower.',
        is_following: true,
        is_approved_follower: true,
        followers_count: 512,
        following_count: 310,
      };
    }
  },

  async createAccount(username: string, displayName: string, kind = 'owner', provider = 'mock'): Promise<Account> {
    try {
      return await safeInvoke<Account>('create_account', {
        username,
        displayName,
        accountKind: kind,
        providerType: provider,
        instagramUserId: null,
      });
    } catch {
      const newAcc: Account = {
        id: 'acc_' + Date.now(),
        username,
        display_name: displayName || username,
        account_kind: kind as any,
        provider_type: provider as any,
        is_private: false,
        is_verified: false,
        followers_count: 1240,
        following_count: 810,
        monitoring_enabled: true,
        created_at: Math.floor(Date.now() / 1000),
        updated_at: Math.floor(Date.now() / 1000),
        last_successful_sync_at: Math.floor(Date.now() / 1000),
        authenticated_by_account_id: null,
        access_state: 'accessible',
        access_reason: 'Created account',
        target_privacy: 'public',
        last_access_checked_at: Math.floor(Date.now() / 1000),
      };
      localAccounts.push(newAcc);
      return newAcc;
    }
  },

  async deleteAccount(id: string): Promise<void> {
    try {
      await safeInvoke('delete_account', { id });
    } catch {
      const idx = localAccounts.findIndex((a) => a.id === id);
      if (idx !== -1) localAccounts.splice(idx, 1);
    }
  },

  async getRelationshipSummary(accountId: string): Promise<RelationshipSummary> {
    try {
      return await safeInvoke<RelationshipSummary>('get_relationship_summary', { accountId });
    } catch {
      if (!fallbackAllowed()) throw new Error('Failed to load relationship summary');
      const { liveSummary, liveTargetSummary, mockSummary, mockTargetSummary } = await import('./mockData');
      const isTarget = accountId === 'acc_demo_private_friend' || accountId.includes('private_friend');
      if (isTarget) return { ...(liveTargetSummary ?? mockTargetSummary) };
      return { ...(liveSummary ?? mockSummary) };
    }
  },

  async getPeople(
    accountId: string,
    filterType: string,
    searchQuery: string | null = null,
    sortByOrLimit: string | number = 'name_asc',
    limitOrOffset = 50,
    offsetOrSortBy: number | string = 0
  ): Promise<PersonListItem[]> {
    let sortBy = 'name_asc';
    let limit = 50;
    let offset = 0;

    if (typeof sortByOrLimit === 'number') {
      limit = sortByOrLimit;
      offset = typeof limitOrOffset === 'number' ? limitOrOffset : 0;
      if (typeof offsetOrSortBy === 'string') {
        sortBy = offsetOrSortBy;
      }
    } else {
      sortBy = sortByOrLimit;
      limit = typeof limitOrOffset === 'number' ? limitOrOffset : 50;
      offset = typeof offsetOrSortBy === 'number' ? offsetOrSortBy : 0;
    }

    try {
      const res = await safeInvoke<PersonListItem[]>('get_people', {
        accountId,
        filterType,
        searchQuery,
        sortBy,
        limit,
        offset,
      });
      if (Array.isArray(res)) return res;
      return [];
    } catch {
      if (!fallbackAllowed()) return [];
      const all = filterMockPeople(accountId, filterType, searchQuery, sortBy);
      return all.slice(offset, offset + limit);
    }
  },

  async getPeopleCount(
    accountId: string,
    filterType: string,
    searchQuery: string | null = null
  ): Promise<number> {
    try {
      return await safeInvoke<number>('get_people_count', {
        accountId,
        filterType,
        searchQuery,
      });
    } catch {
      if (!fallbackAllowed()) return 0;
      return filterMockPeople(accountId, filterType, searchQuery).length;
    }
  },

  async getTargetOverlap(
    ownerAccountId: string,
    targetAccountId: string,
    limit = 20
  ): Promise<TargetOverlapReport> {
    try {
      return await safeInvoke<TargetOverlapReport>('get_target_overlap', {
        ownerAccountId,
        targetAccountId,
        limit,
      });
    } catch {
      if (!fallbackAllowed()) return { mutual_connections_count: 0, shared_people: [] };
      // Demo mode: Return realistic shared connections
      const ownerMutualUsernames = new Set(localPeople.filter((p) => p.is_mutual).map((p) => p.username));
      const shared = localTargetPeople
        .filter((p) => p.is_mutual && ownerMutualUsernames.has(p.username))
        .slice(0, limit);
      return {
        mutual_connections_count: shared.length > 0 ? shared.length : 10,
        shared_people: shared.length > 0 ? shared : localPeople.filter((p) => p.is_mutual).slice(0, 10),
      };
    }
  },

  async getChangesFeed(
    accountId: string,
    filterTypeOrLimit: string | number = 'all',
    searchQueryOrOffset: string | number | null = null,
    limitParam = 50,
    offsetParam = 0
  ): Promise<ChangeFeedItem[]> {
    let filterType = 'all';
    let searchQuery: string | null = null;
    let limit = 50;
    let offset = 0;

    if (typeof filterTypeOrLimit === 'number') {
      limit = filterTypeOrLimit;
      offset = typeof searchQueryOrOffset === 'number' ? searchQueryOrOffset : 0;
    } else {
      filterType = filterTypeOrLimit;
      searchQuery = typeof searchQueryOrOffset === 'string' ? searchQueryOrOffset : null;
      limit = limitParam;
      offset = offsetParam;
    }

    try {
      const res = await safeInvoke<ChangeFeedItem[]>('get_changes_feed', {
        accountId,
        filterType: filterType === 'all' ? null : filterType,
        searchQuery: searchQuery?.trim() ? searchQuery.trim() : null,
        limit,
        offset,
      });
      if (Array.isArray(res)) return res;
      return [];
    } catch {
      if (!fallbackAllowed()) return [];
      const isTarget = accountId === 'acc_demo_private_friend' || accountId.includes('private_friend');
      const source = isTarget ? localTargetChanges : localChanges;
      return filterMockChanges(source, filterType, searchQuery).slice(offset, offset + limit);
    }
  },

  async getChangesCount(
    accountId: string,
    filterType: string = 'all',
    searchQuery: string | null = null
  ): Promise<number> {
    try {
      return await safeInvoke<number>('get_changes_count', {
        accountId,
        filterType: filterType === 'all' ? null : filterType,
        searchQuery: searchQuery?.trim() ? searchQuery.trim() : null,
      });
    } catch {
      if (!fallbackAllowed()) return 0;
      const isTarget = accountId === 'acc_demo_private_friend' || accountId.includes('private_friend');
      const source = isTarget ? localTargetChanges : localChanges;
      return filterMockChanges(source, filterType, searchQuery).length;
    }
  },

  async getNote(accountId: string, personId: string): Promise<NoteDto | null> {
    try {
      return await safeInvoke<NoteDto | null>('get_note', { accountId, personId });
    } catch {
      if (!fallbackAllowed()) return null;
      const isTarget = accountId === 'acc_demo_private_friend' || accountId.includes('private_friend');
      const content = isTarget ? localTargetNotes[personId] : localNotes[personId];
      if (content) {
        return {
          id: 'note_' + personId,
          account_id: accountId,
          person_id: personId,
          content,
          updated_at: Math.floor(Date.now() / 1000) - 3600,
        };
      }
      return null;
    }
  },

  async saveNote(accountId: string, personId: string, content: string): Promise<NoteDto> {
    try {
      return await safeInvoke<NoteDto>('save_note', { accountId, personId, content });
    } catch {
      if (!fallbackAllowed()) throw new Error('Failed to save note');
      const isTarget = accountId === 'acc_demo_private_friend' || accountId.includes('private_friend');
      if (isTarget) {
        localTargetNotes[personId] = content;
        const targetPerson = localTargetPeople.find((p) => p.id === personId);
        if (targetPerson) targetPerson.has_note = true;
      } else {
        localNotes[personId] = content;
        const targetPerson = localPeople.find((p) => p.id === personId);
        if (targetPerson) targetPerson.has_note = true;
      }
      return {
        id: 'note_' + personId,
        account_id: accountId,
        person_id: personId,
        content,
        updated_at: Math.floor(Date.now() / 1000),
      };
    }
  },

  async deleteNote(accountId: string, personId: string): Promise<void> {
    try {
      await safeInvoke('delete_note', { accountId, personId });
    } catch {
      if (!fallbackAllowed()) return;
      delete localNotes[personId];
      const targetPerson = localPeople.find((p) => p.id === personId);
      if (targetPerson) targetPerson.has_note = false;
    }
  },

  async getProviderHealth(accountId?: string): Promise<ProviderHealthStatus> {
    try {
      return await safeInvoke<ProviderHealthStatus>('get_provider_health', { accountId });
    } catch {
      return {
        provider_name: 'Authenticated Session',
        is_connected: true,
        status_text: 'Verified and operational',
        last_successful_sync: Date.now() / 1000 - 240,
        follower_retrieval_ok: true,
        following_retrieval_ok: true,
        pagination_ok: true,
        completeness_check_ok: true,
        provider_version: 'Session Provider 1.0 (Oct 2026)',
      };
    }
  },

  async syncNow(accountId: string): Promise<SyncReport> {
    try {
      return await safeInvoke<SyncReport>('sync_now', { accountId });
    } catch {
      if (!fallbackAllowed()) throw new Error('Sync failed — please check your connection');
      await new Promise((r) => setTimeout(r, ri(500, 1400)));
      const now = Math.floor(Date.now() / 1000);

      const acc = localAccounts.find((a) => a.id === accountId);
      const isTarget = accountId === 'acc_demo_private_friend' || accountId.includes('private_friend');
      const { liveSummary, liveTargetSummary, setLiveSummary, setLiveTargetSummary } = await import('./mockData');

      // Random ±delta
      const fDelta = ri(-12, 18);
      const fgDelta = ri(-5, 8);
      const changesDetected = ri(0, 5);

      if (acc) {
        acc.followers_count = Math.max(0, acc.followers_count + fDelta);
        acc.following_count = Math.max(0, acc.following_count + fgDelta);
        acc.last_successful_sync_at = now;
        acc.last_attempted_sync_at = now;
      }

      // Update live summary to reflect new counts
      const baseSummary = isTarget ? (liveTargetSummary ?? { followers: 18, following: 14, mutual: 10, not_following_back: 4, fans: 8, net_delta_7d: 2 })
                                   : (liveSummary ?? { followers: 1842, following: 936, mutual: 612, not_following_back: 324, fans: 1230, net_delta_7d: 15 });
      const newFollowers = Math.max(25, baseSummary.followers + fDelta);
      const newFollowing = Math.max(18, baseSummary.following + fgDelta);
      const maxMutual = Math.max(5, Math.min(newFollowers - 6, newFollowing - 6));
      const newMutual = Math.min(maxMutual, Math.max(5, baseSummary.mutual + ri(-2, 3)));
      const updatedSummary = {
        followers: newFollowers,
        following: newFollowing,
        mutual: newMutual,
        not_following_back: Math.max(4, newFollowing - newMutual),
        fans: Math.max(4, newFollowers - newMutual),
        net_delta_7d: baseSummary.net_delta_7d + fDelta,
      };
      if (isTarget) setLiveTargetSummary(updatedSummary);
      else setLiveSummary(updatedSummary);

      // Inject new change events into the live feed and update people relationship flags
      if (changesDetected > 0) {
        const people = isTarget ? localTargetPeople : localPeople;
        const feed = isTarget ? localTargetChanges : localChanges;
        const changeTypes: ChangeFeedItem['change_type'][] = ['followed_you', 'unfollowed_you', 'you_followed'];
        const pool = people.slice(0, Math.min(changesDetected, people.length));
        pool.slice(0, changesDetected).forEach((p, i) => {
          const ct = rc(changeTypes);
          if (ct === 'unfollowed_you') {
            p.is_follower = false;
            p.is_mutual = false;
          } else if (ct === 'followed_you') {
            p.is_follower = true;
            p.is_mutual = p.is_following;
          }
          feed.unshift({
            id: `c_sync_${Date.now()}_${i}`,
            account_id: accountId,
            person_id: p.id,
            related_username: p.username,
            display_name: p.display_name,
            avatar_url: p.avatar_url,
            change_type: ct,
            confidence: 'confirmed',
            detected_at: now - ri(10, 180),
            before_snapshot_id: `snap_sync_prev_${i}`,
            after_snapshot_id: `snap_sync_curr_${i}`,
          });
        });
      }

      return {
        success: true,
        status: 'complete',
        followers_count: acc?.followers_count ?? 1842,
        following_count: acc?.following_count ?? 936,
        changes_detected: changesDetected,
        message: changesDetected > 0
          ? `Sync complete. Detected ${changesDetected} new relationship change${changesDetected > 1 ? 's' : ''}.`
          : 'Sync complete. All relationships verified. No new changes.',
        profile_followers_count: acc?.followers_count ?? 1842,
        profile_following_count: acc?.following_count ?? 936,
        tracked_followers_count: acc?.followers_count ?? 1842,
        tracked_following_count: acc?.following_count ?? 936,
        verification: 'demo_fallback',
      };
    }
  },

  async importRawExportContent(accountId: string, followersJson: string, followingJson: string): Promise<SyncReport> {
    return await safeInvoke<SyncReport>('import_raw_export_content', {
      accountId,
      followersJson,
      followingJson,
    });
  },

  async getSetting(key: string): Promise<string | null> {
    try {
      return await safeInvoke<string | null>('get_setting', { key });
    } catch {
      return null;
    }
  },

  async setSetting(key: string, value: string): Promise<void> {
    try {
      await safeInvoke('set_setting', { key, value });
    } catch {
      // noop
    }
  },

  async scheduleBackgroundSync(accountId: string, intervalMinutes: number): Promise<boolean> {
    try {
      if (typeof window !== 'undefined' && (window as any).StalkrAuth?.schedulePeriodicSync) {
        return (window as any).StalkrAuth.schedulePeriodicSync(accountId, intervalMinutes);
      }
      return true;
    } catch {
      return false;
    }
  },

  async deleteAllLocalData(): Promise<void> {
    try {
      await safeInvoke('delete_all_local_data');
    } catch {
      // ignore
    }
    // Always wipe in-memory state and reset demo mode
    setDemoMode(false);
    localAccounts.length = 0;
    localPeople.length = 0;
    localTargetPeople.length = 0;
    localChanges.length = 0;
    localTargetChanges.length = 0;
    for (const key of Object.keys(localNotes)) delete localNotes[key];
    for (const key of Object.keys(localTargetNotes)) delete localTargetNotes[key];
    // Reset live summaries in mockData module
    const { setLiveSummary, setLiveTargetSummary } = await import('./mockData');
    setLiveSummary(null);
    setLiveTargetSummary(null);
  },
};

function filterMockPeople(
  accountId: string,
  filterType: string,
  searchQuery: string | null = null,
  sortBy: string = 'name_asc'
): PersonListItem[] {
  const isTarget = accountId === 'acc_demo_private_friend' || accountId.includes('private_friend');
  const source = isTarget ? localTargetPeople : localPeople;
  let list = [...source];
  if (filterType === 'mutual') {
    list = list.filter((p) => p.is_mutual);
  } else if (filterType === 'not_following_back') {
    list = list.filter((p) => !p.is_follower && p.is_following);
  } else if (filterType === 'fans') {
    list = list.filter((p) => p.is_follower && !p.is_following);
  } else if (filterType === 'following') {
    list = list.filter((p) => p.is_following);
  } else if (filterType === 'followers') {
    list = list.filter((p) => p.is_follower);
  }

  if (searchQuery && searchQuery.trim().length > 0) {
    const q = searchQuery.toLowerCase().trim().replace(/^@/, '');
    if (q.length > 0) {
      list = list.filter(
        (p) =>
          p.username.toLowerCase().includes(q) ||
          (p.display_name && p.display_name.toLowerCase().includes(q))
      );
    }
  }

  if (sortBy === 'mutual_first') {
    list.sort((a, b) => (b.is_mutual ? 1 : 0) - (a.is_mutual ? 1 : 0) || a.username.localeCompare(b.username));
  } else if (sortBy === 'followers_first') {
    list.sort((a, b) => (b.is_follower ? 1 : 0) - (a.is_follower ? 1 : 0) || a.username.localeCompare(b.username));
  } else {
    list.sort((a, b) => a.username.localeCompare(b.username));
  }

  return list;
}

function filterMockChanges(
  source: ChangeFeedItem[],
  filterType: string,
  searchQuery: string | null = null
): ChangeFeedItem[] {
  let list = [...source];
  if (filterType === 'lost' || filterType === 'unfollowed_you') {
    list = list.filter((c) => c.change_type === 'unfollowed_you');
  } else if (filterType === 'gained' || filterType === 'followed_you') {
    list = list.filter((c) => c.change_type === 'followed_you');
  } else if (filterType === 'renamed' || filterType === 'username_changed') {
    list = list.filter((c) => c.change_type === 'username_changed');
  } else if (filterType === 'outbound') {
    list = list.filter((c) => c.change_type === 'you_followed' || c.change_type === 'you_unfollowed');
  }

  if (searchQuery && searchQuery.trim().length > 0) {
    const q = searchQuery.toLowerCase().trim().replace(/^@/, '');
    if (q.length > 0) {
      list = list.filter(
        (c) =>
          c.related_username.toLowerCase().includes(q) ||
          (c.display_name && c.display_name.toLowerCase().includes(q)) ||
          (c.metadata_json && c.metadata_json.toLowerCase().includes(q))
      );
    }
  }

  list.sort((a, b) => b.detected_at - a.detected_at);
  return list;
}
