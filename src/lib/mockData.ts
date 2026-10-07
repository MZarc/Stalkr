import type { Account, PersonListItem, RelationshipSummary, ChangeFeedItem, NoteDto, ProviderHealthStatus, SyncReport } from './api';

// ─── Helpers ─────────────────────────────────────────────────────────────────

function ri(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}
function rc<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}
function shuffle<T>(arr: T[]): T[] {
  const a = [...arr];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

// ─── Avatar pools ─────────────────────────────────────────────────────────────

const FEMALE_AVATARS = [
  'https://images.unsplash.com/photo-1534528741775-53994a69daeb?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1494790108377-be9c29b29330?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1524504388940-b1c1722653e1?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1517841905240-472988babdf9?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1544005313-94ddf0286df2?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1573496359142-b8d87734a5a2?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1529626455594-4ff0802cfb7e?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1488426862026-3ee34a7d66df?auto=format&fit=crop&w=120&q=80',
];
const MALE_AVATARS = [
  'https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1500648767791-00dcc994a43e?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1519085360753-af0119f7cbe7?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1501196354995-cbb51c65aaea?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1492562080023-ab3db95bfbce?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1539571696357-5a69c17a67c6?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?auto=format&fit=crop&w=120&q=80',
  'https://images.unsplash.com/photo-1522075469751-3a6694fb2f61?auto=format&fit=crop&w=120&q=80',
];

// ─── Username / name pools ────────────────────────────────────────────────────

const USERNAMES = [
  'elena.rostova', 'alex_vance', 'sora_k', 'marcus_dev', 'luna_art',
  'david.co', 'clara_h', 'kai_zenith', 'maya_lin', 'hugo_nordic',
  'olivia.wong', 'sebastian_k', 'chloe_paris', 'zane.ai', 'liam_creates',
  'isabella_style', 'noah_visuals', 'mia_aesthetic', 'ethan_sound', 'ava_ventures',
  'lucas_lens', 'harper_daily', 'felix_r', 'sofia_mag', 'theo_b',
  'nadia_pro', 'ryu_studio', 'priya_d', 'jake_w', 'amara_c',
  'yuki_shot', 'ben_type', 'ana_m', 'leo_j', 'rue_v',
  'sam_arts', 'tara_b', 'nico_f', 'zoe_q', 'kai_style',
];
const DISPLAY_NAMES = [
  'Elena Rostova', 'Alex Vance', 'Sora Kuroki', 'Marcus Thorne', 'Luna Sterling',
  'David Chen', 'Clara Hughes', 'Kai Zenith', 'Maya Lin', 'Hugo Lindqvist',
  'Olivia Wong', 'Sebastian Klein', 'Chloé Laurent', 'Zane Robertson', 'Liam O\'Connor',
  'Isabella Rossi', 'Noah Miller', 'Mia Chen', 'Ethan Walker', 'Ava Morales',
  'Lucas Silva', 'Harper Evans', 'Felix Richter', 'Sofia Magnusson', 'Theo Beck',
  'Nadia Prokop', 'Ryu Nakamura', 'Priya Das', 'Jake Williams', 'Amara Coulibaly',
  'Yuki Tanaka', 'Ben Tyler', 'Ana Moreira', 'Leo Jung', 'Rue Vasiliev',
  'Sam Artega', 'Tara Bloom', 'Nico Ferro', 'Zoë Quentin', 'Kai Styles',
];
const TAGS = [
  { id: 't1', name: 'Close Friends', color: '#10b981' },
  { id: 't2', name: 'Architecture', color: '#6366f1' },
  { id: 't3', name: 'San Francisco', color: '#e5b84c' },
  { id: 't4', name: 'Founders', color: '#f43f5e' },
  { id: 't5', name: 'Art & Design', color: '#8b5cf6' },
  { id: 't6', name: 'Curators', color: '#38bdf8' },
  { id: 't7', name: 'Tech', color: '#06b6d4' },
  { id: 't8', name: 'Photography', color: '#f97316' },
];
const NOTES_POOL = [
  'Met at design week. Great collaborator.',
  'College friend from back home.',
  'Creative lead at a top Berlin studio. Impressive portfolio.',
  'Follows back rarely — curated list.',
  'Unfollowed quietly after the rebrand.',
  'Ceramics designer from Kyoto. Close friend.',
  'Founder of a stealth startup. Sharp thinker.',
  'Recommended by Marcus. Worth keeping in touch.',
  'Curates independent gallery prints.',
  'Lead architect at Studio Berlin.',
  'Shared connection from the monograph event.',
  'Photography partner since 2023.',
  'Followed after the podcast collab.',
  'Based in Paris. Art film community.',
];

// ─── Random data generator ────────────────────────────────────────────────────

export interface DemoDataSet {
  ownerAccount: Account;
  targetAccount: Account;
  ownerSummary: RelationshipSummary;
  targetSummary: RelationshipSummary;
  people: PersonListItem[];
  targetPeople: PersonListItem[];
  changes: ChangeFeedItem[];
  targetChanges: ChangeFeedItem[];
  notes: Record<string, string>;
  targetNotes: Record<string, string>;
}

export function generateRandomDemoData(): DemoDataSet {
  const now = Math.floor(Date.now() / 1000);

  // ── Owner account stats ──
  const ownerFollowers = ri(1200, 9800);
  const ownerFollowing = ri(400, Math.min(ownerFollowers, 3200));
  const ownerMutual = ri(Math.floor(ownerFollowing * 0.4), Math.min(ownerFollowing, ownerFollowers));
  const ownerFans = ownerFollowers - ownerMutual;
  const ownerNFB = ownerFollowing - ownerMutual;
  const ownerDelta = ri(-120, 280);

  const ownerAccount: Account = {
    id: 'acc_demo_meetzarc',
    instagram_user_id: 'ig_meetzarc',
    username: 'meetzarc',
    display_name: 'Meet Mistry',
    account_kind: 'owner',
    provider_type: 'mock',
    avatar_url: '/profile.png',
    is_private: false,
    is_verified: true,
    followers_count: ownerFollowers,
    following_count: ownerFollowing,
    monitoring_enabled: true,
    created_at: now - 86400 * ri(30, 90),
    updated_at: now,
    last_successful_sync_at: now - ri(60, 400),
    last_attempted_sync_at: now,
    authenticated_by_account_id: null,
    access_state: 'accessible',
    access_reason: 'Authenticated owner session (Demo Mode)',
    target_privacy: 'public',
    last_access_checked_at: now,
  };

  const ownerSummary: RelationshipSummary = {
    followers: ownerFollowers,
    following: ownerFollowing,
    mutual: ownerMutual,
    not_following_back: ownerNFB,
    fans: ownerFans,
    net_delta_7d: ownerDelta,
  };

  // ── Target (monitored) account stats ──
  const targetFollowers = ri(12, 890);
  const targetFollowing = ri(8, Math.min(targetFollowers, 600));
  const targetMutual = ri(Math.floor(targetFollowing * 0.4), Math.min(targetFollowing, targetFollowers));
  const targetFans = targetFollowers - targetMutual;
  const targetNFB = targetFollowing - targetMutual;
  const targetDelta = ri(-8, 35);

  const targetAccount: Account = {
    id: 'acc_demo_private_friend',
    instagram_user_id: 'ig_private_friend',
    username: 'private_friend',
    display_name: 'Private Friend',
    account_kind: 'monitored',
    provider_type: 'mock',
    avatar_url: rc(FEMALE_AVATARS),
    is_private: true,
    is_verified: false,
    followers_count: targetFollowers,
    following_count: targetFollowing,
    monitoring_enabled: true,
    created_at: now - 86400 * ri(5, 20),
    updated_at: now,
    last_successful_sync_at: now - ri(60, 300),
    last_attempted_sync_at: now,
    authenticated_by_account_id: 'acc_demo_meetzarc',
    access_state: 'accessible',
    access_reason: 'Private account accessible through authenticated approved follower (@meetzarc).',
    target_privacy: 'private',
    last_access_checked_at: now,
  };

  const targetSummary: RelationshipSummary = {
    followers: targetFollowers,
    following: targetFollowing,
    mutual: targetMutual,
    not_following_back: targetNFB,
    fans: targetFans,
    net_delta_7d: targetDelta,
  };

  // ── Generate owner people list ──
  const shuffledUsernames = shuffle(USERNAMES);
  const shuffledNames = shuffle(DISPLAY_NAMES);
  const totalPeople = ri(18, 30);
  const mutualCount = Math.min(ownerMutual, ri(8, Math.floor(totalPeople * 0.55)));
  const nfbCount = Math.min(ownerNFB, ri(3, Math.floor(totalPeople * 0.2)));
  const fanCount = totalPeople - mutualCount - nfbCount;

  const people: PersonListItem[] = [];
  const notes: Record<string, string> = {};
  let pIdx = 0;

  const makeP = (
    suffix: string,
    is_follower: boolean,
    is_following: boolean,
    changeType?: ChangeFeedItem['change_type'],
    changeHoursAgo?: number
  ): PersonListItem => {
    const username = shuffledUsernames[pIdx % shuffledUsernames.length];
    const display_name = shuffledNames[pIdx % shuffledNames.length];
    const isFemale = pIdx % 3 !== 1;
    const avatar_url = rc(isFemale ? FEMALE_AVATARS : MALE_AVATARS);
    pIdx++;
    const id = `p_${suffix}_${pIdx}`;
    const tagCount = ri(0, 2);
    const tags = shuffle(TAGS).slice(0, tagCount);
    const has_note = Math.random() < 0.25;
    if (has_note) notes[id] = rc(NOTES_POOL);
    return {
      id,
      username,
      display_name,
      avatar_url,
      is_verified: Math.random() < 0.2,
      is_private: Math.random() < 0.25,
      is_follower,
      is_following,
      is_mutual: is_follower && is_following,
      last_seen_at: now - ri(1800, 3600 * 96),
      tags,
      has_note,
      ...(changeType ? { last_change_type: changeType, last_change_at: now - 3600 * (changeHoursAgo ?? ri(1, 72)) } : {}),
    };
  };

  for (let i = 0; i < mutualCount; i++) {
    people.push(makeP('m', true, true));
  }
  for (let i = 0; i < nfbCount; i++) {
    people.push(makeP('nfb', false, true, rc(['unfollowed_you', 'unfollowed_you']), ri(2, 72)));
  }
  for (let i = 0; i < fanCount; i++) {
    people.push(makeP('fan', true, false, ri(0, 4) === 0 ? 'followed_you' : undefined, ri(1, 48)));
  }

  // ── Generate owner changes feed ──
  const changeTypes: ChangeFeedItem['change_type'][] = [
    'followed_you', 'unfollowed_you', 'unfollowed_you', 'followed_you',
    'you_followed', 'you_unfollowed', 'username_changed',
  ];
  const changeCount = ri(12, 20);
  const changes: ChangeFeedItem[] = [];
  const usedPeople = shuffle(people).slice(0, Math.min(changeCount, people.length));
  // Spread across all time: Today (1-18h), Yesterday (24-40h), This Week (3-7d), Earlier (8-60d)
  const timeBucketsHours = [2, 6, 14, 28, 38, 72, 120, 168, 240, 360, 480, 720, 960, 1200];
  usedPeople.forEach((p, i) => {
    const ct = rc(changeTypes);
    const baseH = i < timeBucketsHours.length ? timeBucketsHours[i] : ri(100, 1400);
    const hoursAgo = baseH + ri(0, 6);
    changes.push({
      id: `c_${i + 1}_${Date.now()}`,
      account_id: 'acc_demo_meetzarc',
      person_id: p.id,
      related_username: p.username,
      display_name: p.display_name,
      avatar_url: p.avatar_url,
      change_type: ct,
      confidence: 'confirmed',
      detected_at: now - 3600 * hoursAgo,
      before_snapshot_id: `snap_prev_${i}`,
      after_snapshot_id: `snap_curr_${i}`,
      ...(ct === 'username_changed'
        ? { metadata_json: JSON.stringify({ old_username: p.username + '_old', new_username: p.username }) }
        : {}),
    });
  });
  changes.sort((a, b) => b.detected_at - a.detected_at);

  // ── Generate target people list ──
  pIdx = 100;
  const targetPeople: PersonListItem[] = [];
  const targetNotes: Record<string, string> = {};
  const tTotalPeople = ri(10, 22);
  const tMutualCount = Math.min(targetMutual, ri(4, Math.floor(tTotalPeople * 0.6)));
  const tNfbCount = Math.min(targetNFB, ri(2, Math.floor(tTotalPeople * 0.2)));
  const tFanCount = tTotalPeople - tMutualCount - tNfbCount;

  const makeTP = (
    suffix: string,
    is_follower: boolean,
    is_following: boolean,
    changeType?: ChangeFeedItem['change_type'],
    changeHoursAgo?: number
  ): PersonListItem => {
    const username = shuffledUsernames[pIdx % shuffledUsernames.length];
    const display_name = shuffledNames[pIdx % shuffledNames.length];
    const isFemale = pIdx % 3 === 0;
    const avatar_url = rc(isFemale ? FEMALE_AVATARS : MALE_AVATARS);
    pIdx++;
    const id = `tp_${suffix}_${pIdx}`;
    const has_note = Math.random() < 0.2;
    if (has_note) targetNotes[id] = rc(NOTES_POOL);
    return {
      id,
      username,
      display_name,
      avatar_url,
      is_verified: Math.random() < 0.15,
      is_private: Math.random() < 0.3,
      is_follower,
      is_following,
      is_mutual: is_follower && is_following,
      last_seen_at: now - ri(1800, 3600 * 72),
      tags: [],
      has_note,
      ...(changeType ? { last_change_type: changeType, last_change_at: now - 3600 * (changeHoursAgo ?? ri(1, 48)) } : {}),
    };
  };

  for (let i = 0; i < tMutualCount; i++) targetPeople.push(makeTP('m', true, true));
  for (let i = 0; i < tNfbCount; i++) targetPeople.push(makeTP('nfb', false, true, 'unfollowed_you', ri(2, 48)));
  for (let i = 0; i < tFanCount; i++) targetPeople.push(makeTP('fan', true, false, ri(0, 3) === 0 ? 'followed_you' : undefined, ri(1, 36)));

  // ── Generate target changes feed ──
  const tChangeCount = ri(6, 12);
  const targetChanges: ChangeFeedItem[] = [];
  const usedTargetPeople = shuffle(targetPeople).slice(0, Math.min(tChangeCount, targetPeople.length));
  const tTimeBucketsHours = [3, 8, 26, 42, 96, 180, 300, 500, 800];
  usedTargetPeople.forEach((p, i) => {
    const ct = rc(changeTypes);
    const baseH = i < tTimeBucketsHours.length ? tTimeBucketsHours[i] : ri(80, 900);
    const hoursAgo = baseH + ri(0, 5);
    targetChanges.push({
      id: `c_target_${i + 1}_${Date.now()}`,
      account_id: 'acc_demo_private_friend',
      person_id: p.id,
      related_username: p.username,
      display_name: p.display_name,
      avatar_url: p.avatar_url,
      change_type: ct,
      confidence: 'confirmed',
      detected_at: now - 3600 * hoursAgo,
      before_snapshot_id: `tsnap_prev_${i}`,
      after_snapshot_id: `tsnap_curr_${i}`,
      ...(ct === 'username_changed'
        ? { metadata_json: JSON.stringify({ old_username: p.username + '_old', new_username: p.username }) }
        : {}),
    });
  });
  targetChanges.sort((a, b) => b.detected_at - a.detected_at);

  return {
    ownerAccount,
    targetAccount,
    ownerSummary,
    targetSummary,
    people,
    targetPeople,
    changes,
    targetChanges,
    notes,
    targetNotes,
  };
}

// ─── Stable state for reactive sync simulation ────────────────────────────────

/** Mutable live summaries — updated by syncNow simulation */
export let liveSummary: RelationshipSummary | null = null;
export let liveTargetSummary: RelationshipSummary | null = null;

export function setLiveSummary(s: RelationshipSummary | null) { liveSummary = s; }
export function setLiveTargetSummary(s: RelationshipSummary | null) { liveTargetSummary = s; }

// ─── Legacy static exports (kept for type compatibility) ─────────────────────
// These are no longer used directly — use generateRandomDemoData() instead.

export const mockAccount: Account = {
  id: 'acc_demo_meetzarc',
  username: 'meetzarc',
  display_name: 'Meet Mistry',
  account_kind: 'owner',
  provider_type: 'mock',
  avatar_url: '/profile.png',
  is_private: false,
  is_verified: true,
  followers_count: 1842,
  following_count: 936,
  monitoring_enabled: true,
  created_at: Math.floor(Date.now() / 1000) - 86400 * 45,
  updated_at: Math.floor(Date.now() / 1000) - 180,
  last_successful_sync_at: Math.floor(Date.now() / 1000) - 340,
  authenticated_by_account_id: null,
  access_state: 'accessible',
  access_reason: 'Authenticated owner session',
  target_privacy: 'public',
  last_access_checked_at: Math.floor(Date.now() / 1000) - 340,
};

export const mockTargetAccount: Account = {
  id: 'acc_demo_private_friend',
  username: 'private_friend',
  display_name: 'Private Friend',
  account_kind: 'monitored',
  provider_type: 'mock',
  avatar_url: 'https://images.unsplash.com/photo-1544005313-94ddf0286df2?auto=format&fit=crop&w=120&q=80',
  is_private: true,
  is_verified: false,
  followers_count: 18,
  following_count: 14,
  monitoring_enabled: true,
  created_at: Math.floor(Date.now() / 1000) - 86400 * 7,
  updated_at: Math.floor(Date.now() / 1000) - 180,
  last_successful_sync_at: Math.floor(Date.now() / 1000) - 180,
  authenticated_by_account_id: 'acc_demo_meetzarc',
  access_state: 'accessible',
  access_reason: 'Private account accessible through authenticated approved follower (@meetzarc).',
  target_privacy: 'private',
  last_access_checked_at: Math.floor(Date.now() / 1000) - 180,
};

export const mockSummary: RelationshipSummary = { followers: 1842, following: 936, mutual: 612, not_following_back: 324, fans: 1230, net_delta_7d: 15 };
export const mockTargetSummary: RelationshipSummary = { followers: 18, following: 14, mutual: 10, not_following_back: 4, fans: 8, net_delta_7d: 2 };
export const mockPeople: PersonListItem[] = [];
export const mockTargetPeople: PersonListItem[] = [];
export const mockChanges: ChangeFeedItem[] = [];
export const mockTargetChanges: ChangeFeedItem[] = [];
export const mockNotes: Record<string, string> = {};
export const mockTargetNotes: Record<string, string> = {};
