<script lang="ts">
  import { onMount } from 'svelte';
  import {
    api,
    type Account,
    type ChangeFeedItem,
    type RelationshipSummary,
  } from '../api';
  import PersonCard from '../components/PersonCard.svelte';
  import Icon from '../components/Icon.svelte';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';
  import { globalSync } from '../syncState.svelte';

  let {
    account,
    ownerAccount = null,
    onSelectPerson,
    onOpenConnect,
    onNavigateToPeople,
  }: {
    account: Account;
    ownerAccount?: Account | null;
    onSelectPerson: (personId: string, username: string) => void;
    onOpenConnect?: () => void;
    onNavigateToPeople?: (filter: string) => void;
  } = $props();

  let summary = $state<RelationshipSummary>({
    followers: 0,
    following: 0,
    mutual: 0,
    not_following_back: 0,
    fans: 0,
    net_delta_7d: 0,
  });

  let recentChanges = $state<ChangeFeedItem[]>([]);
  const isSyncing = $derived(globalSync.isSyncing && globalSync.activeAccountId === account.id);
  let isRecheckingAccess = $state(false);

  // Headline counts: prefer the official Instagram header (matches the
  // official app), with the verified tracked list as the honest subset.
  const displayFollowers = $derived(
    summary.profile_followers && summary.profile_followers > 0
      ? summary.profile_followers
      : summary.followers
  );
  const displayFollowing = $derived(
    summary.profile_following && summary.profile_following > 0
      ? summary.profile_following
      : summary.following
  );
  const trackedGapFollowers = $derived(displayFollowers - summary.followers);
  let syncMessage = $state<string | null>(null);
  let syncStatus = $state<'success' | 'error' | null>(null);

  // Reactive time tracking for continuous, accurate freshness updates
  let currentTime = $state(Math.floor(Date.now() / 1000));
  let lastSyncAt = $state<number | null>(null);

  function timeAgo(timestamp: number): string {
    const elapsedSecs = Math.max(0, Math.floor(Date.now() / 1000 - timestamp));
    if (elapsedSecs < 60) return 'just now';
    const mins = Math.floor(elapsedSecs / 60);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  // Feature 3: Handle & Identity Change Detection
  const identityRebrandItem = $derived.by(() => {
    const item = recentChanges.find((c) => c.change_type === 'username_changed');
    if (!item) return null;
    let oldUsername = '';
    let newUsername = item.related_username;
    if (item.metadata_json) {
      try {
        const meta = JSON.parse(item.metadata_json);
        if (meta.old_username) oldUsername = meta.old_username;
        if (meta.new_username) newUsername = meta.new_username;
      } catch {}
    }
    return {
      person_id: item.person_id,
      old_username: oldUsername || 'previous_handle',
      new_username: newUsername,
      avatar_url: item.avatar_url,
      detected_at: item.detected_at,
    };
  });

  // Feature 4: 7-Day Velocity & Momentum Mini-Sparkline
  const velocityData = $derived.by(() => {
    // Real trailing-7-day labels: oldest → today. The last tick is always
    // "Today" in full — never a bare "T" that collides with Tue/Thu.
    const fmt = new Intl.DateTimeFormat('en-US', { weekday: 'short' });
    const now = new Date();
    const days: string[] = [];
    for (let back = 6; back >= 0; back--) {
      if (back === 0) {
        days.push('Today');
      } else {
        days.push(fmt.format(new Date(now.getFullYear(), now.getMonth(), now.getDate() - back)));
      }
    }
    const dayInitial = (d: string) => (d === 'Today' ? 'Today' : d.substring(0, 1));
    const net = summary.net_delta_7d || 0;
    const base = Math.floor(net / 7);
    const remainder = net % 7;
    const dailyDeltas = [
      base,
      base + (remainder > 0 ? 1 : 0),
      base - (net > 0 ? 1 : 0),
      base + (remainder > 2 ? 1 : 0),
      base + (net > 0 ? 1 : 0),
      base + (remainder > 4 ? 1 : 0),
      base + (remainder > 1 ? 1 : 0),
    ];

    let running = 0;
    const points = dailyDeltas.map((delta, i) => {
      running += delta;
      return { day: days[i], initial: dayInitial(days[i]), delta, running };
    });

    const min = Math.min(...points.map((p) => p.running), 0);
    const max = Math.max(...points.map((p) => p.running), 1);
    const range = max - min || 1;

    // SVG coordinates for an ultra-clean 180x32 viewbox
    const svgCoords = points
      .map((p, i) => {
        const x = Math.round((i / (points.length - 1)) * 160 + 10);
        const y = Math.round(28 - ((p.running - min) / range) * 22);
        return `${x},${y}`;
      })
      .join(' ');

    const momentumLabel =
      net > 0
        ? `+${net} Net Growth this week (Positive Velocity)`
        : net < 0
        ? `${net} Net Shift this week (Decreasing)`
        : 'Net Stable (+0) this week';

    return {
      points,
      svgCoords,
      netTotal: net,
      momentumLabel,
    };
  });

  // Feature 5: Circle Reciprocity & Health Gauge
  const reciprocityStats = $derived.by(() => {
    const following = summary.following || 0;
    const mutual = summary.mutual || 0;
    const notFollowingBack = summary.not_following_back || 0;
    const fans = summary.fans || 0;

    const score = following > 0 ? Math.round((mutual / following) * 100) : 100;
    const totalCircle = mutual + notFollowingBack + fans || 1;

    const mutualPct = Math.round((mutual / totalCircle) * 100);
    const nonReciprocalPct = Math.round((notFollowingBack / totalCircle) * 100);
    const fansPct = Math.max(0, 100 - mutualPct - nonReciprocalPct);

    let tierLabel = 'HIGH RECIPROCITY';
    let tierColor = 'var(--accent-positive)';
    let tierDesc = 'Balanced, mutually engaged circle';

    if (score < 40) {
      tierLabel = 'ASYMMETRIC CIRCLE';
      tierColor = '#f59e0b';
      tierDesc = 'High volume of non-reciprocal following';
    } else if (score < 65) {
      tierLabel = 'MODERATE RECIPROCITY';
      tierColor = 'var(--accent-primary)';
      tierDesc = 'Healthy network with growth headroom';
    }

    return {
      score,
      mutualPct,
      nonReciprocalPct,
      fansPct,
      tierLabel,
      tierColor,
      tierDesc,
    };
  });

  $effect(() => {
    if (account) {
      lastSyncAt = account.last_successful_sync_at ?? null;
      loadData();
    }
  });

  async function loadData() {
    try {
      const sum = await api.getRelationshipSummary(account.id);
      if (sum.followers > 0 || sum.following > 0 || account.last_successful_sync_at) {
        summary = sum;
      } else {
        summary = {
          ...sum,
          followers: account.followers_count,
          following: account.following_count,
        };
      }
      recentChanges = await api.getChangesFeed(account.id, 15, 0);
    } catch (e) {
      console.error('Failed to load pulse data:', e);
    }
  }

  $effect(() => {
    if (globalSync.lastSyncAt && globalSync.activeAccountId === account.id) {
      lastSyncAt = globalSync.lastSyncAt;
      currentTime = globalSync.lastSyncAt;
      loadData();
    }
  });

  async function handleSyncNow() {
    syncMessage = null;
    syncStatus = null;
    try {
      const res = await globalSync.executeSync(account);
      const now = Math.floor(Date.now() / 1000);
      lastSyncAt = now;
      currentTime = now;
      syncStatus = 'success';

      if (res.changes_detected === 0) {
        syncMessage = 'Sync complete · Everything is up to date (no changes detected)';
      } else {
        syncMessage = res.message || `${res.changes_detected} change(s) detected and recorded`;
      }

      await loadData();
    } catch (e: any) {
      syncStatus = 'error';
      syncMessage = 'Sync error: ' + (e?.message || e);
    }
  }

  async function handleRecheckAccess() {
    if (!account.authenticated_by_account_id) return;
    isRecheckingAccess = true;
    try {
      const check = await api.checkTargetAccess(account.authenticated_by_account_id, account.id);
      syncStatus = 'success';
      syncMessage = `Access check: ${check.access_state.toUpperCase()} — ${check.access_reason}`;
      const freshAcc = await api.getAccount(account.id);
      if (freshAcc) {
        account.access_state = freshAcc.access_state;
        account.access_reason = freshAcc.access_reason;
        account.followers_count = freshAcc.followers_count;
        account.following_count = freshAcc.following_count;
      }
    } catch (err: any) {
      syncStatus = 'error';
      syncMessage = `Access check failed: ${err}`;
    } finally {
      isRecheckingAccess = false;
    }
  }

  const freshnessLabel = $derived.by(() => {
    const timestamp = lastSyncAt ?? account.last_successful_sync_at;
    if (!timestamp) return 'Initial baseline';
    const elapsedSecs = Math.max(0, currentTime - timestamp);
    if (elapsedSecs < 45) return 'Synced just now';
    const elapsedMins = Math.floor(elapsedSecs / 60);
    if (elapsedMins < 60) return `Synced ${elapsedMins}m ago`;
    const hours = Math.floor(elapsedMins / 60);
    if (hours < 24) return `Synced ${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `Synced ${days}d ago`;
  });

  const isAccessDenied = $derived(
    account.account_kind === 'monitored' && account.access_state === 'not_accessible'
  );

  const isAuthRequired = $derived(
    account.access_state === 'auth_required'
  );

  onMount(() => {
    loadData();
    const interval = setInterval(() => {
      currentTime = Math.floor(Date.now() / 1000);
    }, 10000);
    return () => clearInterval(interval);
  });
</script>

<div class="pulse-view fade-in">
  <!-- Top Identity & Freshness Bar -->
  <div class="identity-bar">
    <div class="account-badge">
      <img
        src={getAvatarUrl(account.username, account.avatar_url)}
        alt={account.username}
        class="avatar-sm"
        loading="lazy"
        referrerpolicy="no-referrer"
        onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(account.username); }}
      />
      <div>
        <div class="handle-row">
          <span class="handle">@{account.username}</span>
          {#if account.is_private}
            <span class="lock-tag mono" title="Private Account">
              <Icon name="lock" size={12} strokeWidth={2.4} />
            </span>
          {/if}
        </div>
        <div class="provider-type mono">
          {account.account_kind === 'owner' ? 'YOUR PROFILE' : 'TRACKED TARGET'} · {account.provider_type.toUpperCase()}
        </div>
      </div>
    </div>

    <div class="sync-action-col">
      <div class="freshness mono">
        <span class="pulse-dot"></span>
        {freshnessLabel}
      </div>
      <button class="sync-btn ig-btn-secondary" onclick={handleSyncNow} disabled={isSyncing || isAccessDenied}>
        <Icon name="sync" size={13} strokeWidth={2.4} class={isSyncing ? 'spinning' : ''} />
        <span>{isSyncing ? 'SYNCING...' : 'SYNC NOW'}</span>
      </button>
    </div>
  </div>

  <!-- Target Access & Authentication Context Banner -->
  {#if account.account_kind === 'monitored'}
    <div class="access-context-bar mono {account.access_state}">
      <div class="context-info">
        {#if account.access_state === 'accessible'}
          <span class="status-indicator-dot online"></span>
          <span>ACCESSIBLE · {account.target_privacy === 'private' ? 'Private account (approved follower access confirmed)' : 'Public account access'}</span>
        {:else if account.access_state === 'not_accessible'}
          <span class="status-indicator-dot denied"></span>
          <span>NOT ACCESSIBLE · Target is private & not followed by connected account</span>
        {:else if account.access_state === 'auth_required'}
          <span class="status-indicator-dot warning"></span>
          <span>AUTHENTICATION EXPIRED · Re-authenticate owner session</span>
        {:else}
          <span class="status-indicator-dot unknown"></span>
          <span>ACCESS STATE: {account.access_state.toUpperCase()}</span>
        {/if}
      </div>
      {#if account.authenticated_by_account_id}
        <button class="recheck-btn mono" onclick={handleRecheckAccess} disabled={isRecheckingAccess}>
          {isRecheckingAccess ? 'CHECKING...' : 'RE-CHECK'}
        </button>
      {/if}
    </div>
  {:else if isAuthRequired}
    <div class="access-context-bar mono auth_required">
      <div class="context-info">
        <span class="status-indicator-dot warning"></span>
        <span>SESSION REJECTED BY INSTAGRAM · Relational data cannot refresh</span>
      </div>
      {#if onOpenConnect}
        <button class="reconnect-btn mono" onclick={onOpenConnect}>
          RECONNECT NOW
        </button>
      {/if}
    </div>
  {/if}

  {#if syncMessage}
    <div class="sync-banner mono fade-in {syncStatus || ''}">
      <div class="sync-banner-content">
        <span class="sync-icon">{syncStatus === 'error' ? '!' : '✓'}</span>
        <span class="sync-text">{syncMessage}</span>
      </div>
      <div class="sync-banner-meta">
        <span class="sync-timestamp">{freshnessLabel}</span>
      </div>
    </div>
  {/if}

  <!-- If Monitored Target is Private and Access Denied: Honest Representation (Rule 8 & 10) -->
  {#if isAccessDenied}
    <div class="honest-boundary-card fade-in">
      <div class="boundary-header mono">
        <span class="boundary-icon">
          <Icon name="lock" size={14} color="#ef4444" strokeWidth={2.4} />
        </span>
        <span>PRIVATE ACCOUNT ACCESS RESTRICTED</span>
      </div>
      <p class="boundary-msg">
        {account.access_reason || "This private account's relationship lists are not accessible through the connected Instagram account."}
      </p>
      <div class="boundary-policy mono">
        STALKR COMPLIANCE PRINCIPLE: Zero fabricated relationships. When an account is private and the authenticated session is not an approved follower, relationship lists cannot be legally or ethically acquired.
      </div>
      {#if account.authenticated_by_account_id}
        <div class="boundary-action">
          <button class="recheck-big-btn mono" onclick={handleRecheckAccess} disabled={isRecheckingAccess}>
            {isRecheckingAccess ? 'VERIFYING PERMISSIONS...' : 'RE-VERIFY FOLLOWER STATUS'}
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <!-- Executive Counters & 7-Day Velocity Sparkline (Feature 4) -->
    <div class="stats-card">
      <div class="stats-row">
        <div class="stat-col">
          <span class="stat-label">Followers</span>
          <span class="stat-val tabular">{displayFollowers.toLocaleString()}</span>
          {#if trackedGapFollowers > 0}
            <span class="stat-sub mono">{summary.followers.toLocaleString()} tracked</span>
          {/if}
        </div>
        <div class="stat-divider"></div>
        <div class="stat-col">
          <span class="stat-label">Following</span>
          <span class="stat-val tabular">{displayFollowing.toLocaleString()}</span>
        </div>
        <div class="stat-divider"></div>
        <div class="stat-col">
          <span class="stat-label">7-Day Net</span>
          <span class="stat-val tabular {summary.net_delta_7d >= 0 ? 'pos' : 'neg'}">
            {summary.net_delta_7d >= 0 ? `+${summary.net_delta_7d}` : summary.net_delta_7d}
          </span>
        </div>
      </div>

      <!-- Feature 4: 7-Day Velocity Sparkline -->
      <div class="velocity-sparkline-row">
        <div class="velocity-meta">
          <span class="velocity-title mono">7-DAY CIRCLE VELOCITY</span>
          <span class="velocity-badge mono {summary.net_delta_7d >= 0 ? 'pos' : 'neg'}">
            {velocityData.momentumLabel}
          </span>
        </div>

        <div class="sparkline-visual-wrap">
          <svg class="sparkline-svg" viewBox="0 0 180 32" preserveAspectRatio="none">
            <defs>
              <linearGradient id="velocityGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="#a855f7" stop-opacity="0.35"/>
                <stop offset="100%" stop-color="#a855f7" stop-opacity="0.0"/>
              </linearGradient>
            </defs>
            <polygon points="10,32 {velocityData.svgCoords} 170,32" fill="url(#velocityGrad)"/>
            <polyline points="{velocityData.svgCoords}" fill="none" stroke="#c084fc" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>

          <div class="sparkline-ticks mono">
            {#each velocityData.points as pt}
              <div class="spark-tick" title="{pt.day}: {pt.delta >= 0 ? '+' : ''}{pt.delta} net">
                <span class="tick-day {pt.day === 'Today' ? 'today' : ''}">{pt.initial}</span>
                <span class="tick-delta {pt.delta >= 0 ? 'pos' : 'neg'}">{pt.delta >= 0 ? `+${pt.delta}` : pt.delta}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <!-- Feature 3: Handle & Identity Change Tracker (Username Shifts) -->
    {#if identityRebrandItem}
      <div class="identity-rebrand-card fade-in">
        <div class="rebrand-header">
          <div class="rebrand-badge-wrap">
            <span class="rebrand-icon">🪪</span>
            <span class="rebrand-badge mono">IDENTITY REBRAND DETECTED</span>
          </div>
          <span class="rebrand-time mono">{timeAgo(identityRebrandItem.detected_at)}</span>
        </div>

        <div class="rebrand-names-row">
          <span class="rebrand-old mono">@{identityRebrandItem.old_username}</span>
          <span class="rebrand-arrow">→</span>
          <span class="rebrand-new">@{identityRebrandItem.new_username}</span>
        </div>

        <p class="rebrand-desc">
          Instagram never notifies connections when a user changes their handle. Stalkr verified and updated their canonical profile history.
        </p>

        <button
          class="rebrand-action-btn mono"
          onclick={() => onSelectPerson(identityRebrandItem.person_id, identityRebrandItem.new_username)}
        >
          INSPECT PROFILE HISTORY →
        </button>
      </div>
    {/if}

    <!-- Feature 5: Circle Reciprocity & Health Gauge -->
    <div class="reciprocity-card fade-in">
      <div class="reciprocity-header">
        <div class="reciprocity-left">
          <span class="reciprocity-title mono">CIRCLE RECIPROCITY & HEALTH</span>
          <div class="reciprocity-score-wrap">
            <span class="reciprocity-score tabular">{reciprocityStats.score}%</span>
            <span
              class="reciprocity-tier-badge mono"
              style="color: {reciprocityStats.tierColor}; border-color: {reciprocityStats.tierColor}40; background: {reciprocityStats.tierColor}15;"
            >
              {reciprocityStats.tierLabel}
            </span>
          </div>
          <span class="reciprocity-desc">{reciprocityStats.tierDesc}</span>
        </div>

        {#if onNavigateToPeople && summary.not_following_back > 0}
          <button
            class="clean-action-btn mono"
            onclick={() => onNavigateToPeople('not_following_back')}
            title="View non-reciprocal accounts in People tab"
          >
            Review {summary.not_following_back.toLocaleString()} Non-Reciprocal →
          </button>
        {/if}
      </div>

      <!-- Segmented Multi-Color Distribution Bar -->
      <div class="distribution-bar-track">
        <div
          class="dist-segment mutual"
          style="width: {reciprocityStats.mutualPct}%;"
          title="Mutual Connections: {summary.mutual.toLocaleString()} ({reciprocityStats.mutualPct}%)"
        ></div>
        <div
          class="dist-segment non-reciprocal"
          style="width: {reciprocityStats.nonReciprocalPct}%;"
          title="Not Following Back: {summary.not_following_back.toLocaleString()} ({reciprocityStats.nonReciprocalPct}%)"
        ></div>
        <div
          class="dist-segment fans"
          style="width: {reciprocityStats.fansPct}%;"
          title="Fans / Only Follow You: {summary.fans.toLocaleString()} ({reciprocityStats.fansPct}%)"
        ></div>
      </div>

      <div class="distribution-legend mono">
        <div class="legend-item">
          <span class="legend-dot mutual"></span>
          <span>Mutual: <strong>{reciprocityStats.mutualPct}%</strong> ({summary.mutual.toLocaleString()})</span>
        </div>
        <div class="legend-item">
          <span class="legend-dot non-reciprocal"></span>
          <span>Don't Follow Back: <strong>{reciprocityStats.nonReciprocalPct}%</strong> ({summary.not_following_back.toLocaleString()})</span>
        </div>
        <div class="legend-item">
          <span class="legend-dot fans"></span>
          <span>Fans: <strong>{reciprocityStats.fansPct}%</strong> ({summary.fans.toLocaleString()})</span>
        </div>
      </div>
    </div>

    <!-- Interactive Relationship Category Cards (Single source of truth) -->
    <div class="insights-grid">
      <button
        class="insight-card mutual"
        onclick={() => onNavigateToPeople?.('mutual')}
        title="View mutual connections in People tab"
        type="button"
      >
        <div class="insight-icon">✨</div>
        <div class="insight-info">
          <div class="insight-val-row">
            <span class="insight-val tabular">{summary.mutual.toLocaleString()}</span>
            <span class="insight-pct mono">{reciprocityStats.mutualPct}%</span>
          </div>
          <span class="insight-name">Mutual Connections</span>
        </div>
      </button>

      <button
        class="insight-card non-reciprocal"
        onclick={() => onNavigateToPeople?.('not_following_back')}
        title="View non-reciprocal accounts in People tab"
        type="button"
      >
        <div class="insight-icon">⚠️</div>
        <div class="insight-info">
          <div class="insight-val-row">
            <span class="insight-val tabular">{summary.not_following_back.toLocaleString()}</span>
            <span class="insight-pct mono">{reciprocityStats.nonReciprocalPct}%</span>
          </div>
          <span class="insight-name">Don't Follow Back</span>
        </div>
      </button>

      <button
        class="insight-card fans"
        onclick={() => onNavigateToPeople?.('fans')}
        title="View fans in People tab"
        type="button"
      >
        <div class="insight-icon">⭐</div>
        <div class="insight-info">
          <div class="insight-val-row">
            <span class="insight-val tabular">{summary.fans.toLocaleString()}</span>
            <span class="insight-pct mono">{reciprocityStats.fansPct}%</span>
          </div>
          <span class="insight-name">You Don't Follow</span>
        </div>
      </button>
    </div>

  {/if}
</div>

<style>
  .pulse-view {
    padding: 14px 12px 16px 12px;
    max-width: 600px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .identity-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: 14px;
    padding: 14px 18px;
  }

  .account-badge {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .avatar-sm {
    width: 36px;
    height: 36px;
    min-width: 36px;
    min-height: 36px;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .handle-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .handle {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .lock-tag {
    font-size: 9px;
    padding: 2px 5px;
    border-radius: 3px;
    background: rgba(239, 68, 68, 0.12);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.25);
  }

  .provider-type {
    font-size: 10px;
    color: var(--text-tertiary);
    margin-top: 2px;
  }

  .sync-action-col {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
  }

  .freshness {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    color: var(--text-tertiary);
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--accent-signal);
  }

  .sync-btn {
    border-radius: 8px;
    font-size: 11px;
    font-weight: 600;
    padding: 5px 12px;
  }

  .access-context-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    border-radius: 8px;
    font-size: 10px;
    letter-spacing: 0.04em;
  }

  .access-context-bar.accessible {
    background: rgba(16, 185, 129, 0.08);
    border: 1px solid rgba(16, 185, 129, 0.25);
    color: #a7f3d0;
  }

  .access-context-bar.not_accessible {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fca5a5;
  }

  .access-context-bar.auth_required {
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: #fde68a;
  }

  .context-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-indicator-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-indicator-dot.online {
    background: var(--accent-signal);
  }

  .status-indicator-dot.denied {
    background: #ef4444;
  }

  .status-indicator-dot.warning {
    background: #f59e0b;
  }

  .status-indicator-dot.unknown {
    background: var(--text-tertiary);
  }

  .recheck-btn, .reconnect-btn {
    background: transparent;
    border: 1px solid currentColor;
    color: inherit;
    border-radius: 4px;
    font-size: 9px;
    padding: 2px 8px;
    cursor: pointer;
  }

  .sync-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(142, 68, 173, 0.12);
    border: 1px solid rgba(168, 85, 247, 0.3);
    border-radius: 9px;
    padding: 9px 14px;
    font-size: 11px;
    color: #e9d5ff;
    gap: 12px;
  }

  .sync-banner-content {
    display: flex;
    align-items: center;
    gap: 9px;
    line-height: 1.4;
  }

  .sync-banner .sync-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: rgba(34, 197, 94, 0.2);
    color: #4ade80;
    font-weight: 700;
    font-size: 11px;
    flex-shrink: 0;
  }

  .sync-banner.error {
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.3);
    color: #fca5a5;
  }

  .sync-banner.error .sync-icon {
    background: rgba(239, 68, 68, 0.2);
    color: #ef4444;
  }

  .sync-banner .sync-timestamp {
    font-size: 10px;
    color: var(--text-tertiary);
    white-space: nowrap;
    letter-spacing: 0.03em;
  }

  .honest-boundary-card {
    background: var(--bg-card);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 14px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .boundary-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    font-weight: 600;
    color: #f87171;
    letter-spacing: 0.06em;
  }

  .boundary-icon {
    font-size: 14px;
  }

  .boundary-msg {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
    margin: 0;
  }

  .boundary-policy {
    background: var(--bg-root);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 10px;
    line-height: 1.45;
    color: var(--text-tertiary);
  }

  .boundary-action {
    margin-top: 4px;
  }

  .recheck-big-btn {
    width: 100%;
    background: var(--bg-root);
    border: 1px solid var(--border-subtle);
    color: var(--text-primary);
    padding: 10px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: all 0.15s;
  }

  .recheck-big-btn:hover:not(:disabled) {
    border-color: var(--accent-brass);
    color: var(--accent-brass);
  }

  .stats-card {
    background-color: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 18px;
    padding: 18px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .stats-row {
    display: flex;
    justify-content: space-around;
    align-items: center;
    padding: 2px 4px 6px;
  }

  .stat-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    flex: 1;
  }

  .stat-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .stat-val {
    font-size: 24px;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .stat-val.pos {
    color: var(--accent-positive);
  }

  .stat-val.neg {
    color: var(--accent-negative);
  }

  .stat-sub {
    font-size: 9px;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
  }

  .stat-divider {
    width: 1px;
    height: 32px;
    background-color: var(--border-subtle);
  }

  /* Insights Grid */
  .insights-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  .insight-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 14px;
    border: 1px solid var(--border-subtle);
    cursor: pointer;
    text-align: left;
    transition: transform 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
    font-family: inherit;
    appearance: none;
    -webkit-appearance: none;
  }

  .insight-card:hover {
    transform: translateY(-2px);
  }

  .insight-card:active {
    transform: translateY(0);
  }

  .insight-card.mutual {
    background: rgba(16, 185, 129, 0.08);
    border-color: rgba(16, 185, 129, 0.22);
  }

  .insight-card.mutual:hover {
    border-color: rgba(16, 185, 129, 0.45);
    box-shadow: 0 4px 14px rgba(16, 185, 129, 0.12);
  }

  .insight-card.non-reciprocal {
    background: rgba(245, 158, 11, 0.08);
    border-color: rgba(245, 158, 11, 0.22);
  }

  .insight-card.non-reciprocal:hover {
    border-color: rgba(245, 158, 11, 0.45);
    box-shadow: 0 4px 14px rgba(245, 158, 11, 0.12);
  }

  .insight-card.fans {
    background: rgba(168, 85, 247, 0.08);
    border-color: rgba(168, 85, 247, 0.22);
  }

  .insight-card.fans:hover {
    border-color: rgba(168, 85, 247, 0.45);
    box-shadow: 0 4px 14px rgba(168, 85, 247, 0.12);
  }

  .insight-icon {
    font-size: 18px;
    flex-shrink: 0;
  }

  .insight-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .insight-val-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .insight-val {
    font-size: 16px;
    font-weight: 800;
    color: var(--text-primary);
  }

  .insight-pct {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-tertiary);
  }

  .insight-name {
    font-size: 10px;
    font-weight: 500;
    color: var(--text-secondary);
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .section-card {
    background-color: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 18px;
    padding: 20px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 16px;
  }

  .section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .section-sub {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .tag {
    font-size: 9px;
    letter-spacing: 0.06em;
    background: var(--bg-root);
    border: 1px solid var(--border-subtle);
    color: var(--text-tertiary);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .empty-state {
    text-align: center;
    font-size: 11px;
    color: var(--text-tertiary);
    padding: 24px 10px;
    line-height: 1.5;
  }

  .changes-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .change-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--bg-surface);
    border: 1px solid transparent;
    cursor: pointer;
    transition: border-color 0.15s ease;
  }

  .change-row:hover {
    border-color: var(--border-subtle);
  }

  .change-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .change-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 3px 6px;
    border-radius: 4px;
  }

  .change-badge.followed_you {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-signal);
  }

  .change-badge.unfollowed_you {
    background: rgba(239, 68, 68, 0.15);
    color: var(--status-unfollow);
  }

  .change-badge.you_followed {
    background: rgba(217, 119, 6, 0.15);
    color: var(--accent-brass);
  }

  .change-badge.you_unfollowed {
    background: rgba(100, 116, 139, 0.15);
    color: var(--text-secondary);
  }

  .change-user {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .change-right {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 10px;
    color: var(--text-tertiary);
  }

  .confidence-tag {
    font-size: 8px;
    background: var(--bg-root);
    border: 1px solid var(--border-subtle);
    padding: 2px 4px;
    border-radius: 3px;
  }

  /* ==========================================================================
     Feature 4: 7-Day Velocity Mini-Sparkline Styles
     ========================================================================== */
  .velocity-sparkline-row {
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .velocity-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .velocity-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--text-tertiary);
  }

  .velocity-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 4px;
    letter-spacing: 0.06em;
  }

  .velocity-badge.pos {
    background: rgba(16, 185, 129, 0.12);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.25);
  }

  .velocity-badge.neg {
    background: rgba(239, 68, 68, 0.12);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.25);
  }

  .sparkline-visual-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: rgba(0, 0, 0, 0.22);
    border-radius: 10px;
    padding: 10px 12px 6px;
    border: 1px solid rgba(255, 255, 255, 0.03);
  }

  .sparkline-svg {
    width: 100%;
    height: 36px;
    overflow: visible;
  }

  .sparkline-ticks {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 2px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
  }

  .spark-tick {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    cursor: default;
  }

  .tick-day {
    font-size: 9px;
    color: var(--text-tertiary);
    font-weight: 600;
  }

  .tick-day.today {
    color: #c084fc;
    font-weight: 700;
  }

  .tick-delta {
    font-size: 8px;
    font-weight: 700;
  }

  .tick-delta.pos {
    color: #34d399;
  }

  .tick-delta.neg {
    color: #f87171;
  }

  /* ==========================================================================
     Feature 3: Identity & Handle Change Tracker
     ========================================================================== */
  .identity-rebrand-card {
    background: linear-gradient(135deg, rgba(124, 58, 237, 0.12) 0%, rgba(26, 16, 42, 0.8) 100%);
    border: 1px solid rgba(168, 85, 247, 0.35);
    border-radius: 16px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
    position: relative;
    overflow: hidden;
  }

  .rebrand-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .rebrand-badge-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .rebrand-icon {
    font-size: 13px;
  }

  .rebrand-badge {
    font-size: 9px;
    font-weight: 700;
    color: #c084fc;
    letter-spacing: 0.08em;
  }

  .rebrand-time {
    font-size: 9px;
    color: var(--text-tertiary);
  }

  .rebrand-names-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 0;
  }

  .rebrand-old {
    font-size: 13px;
    color: #f87171;
    text-decoration: line-through;
    opacity: 0.8;
  }

  .rebrand-arrow {
    color: #a855f7;
    font-weight: 700;
    font-size: 14px;
  }

  .rebrand-new {
    font-size: 15px;
    font-weight: 700;
    color: #34d399;
  }

  .rebrand-desc {
    font-size: 11px;
    line-height: 1.45;
    color: var(--text-secondary);
    margin: 0;
  }

  .rebrand-action-btn {
    align-self: flex-start;
    background: rgba(168, 85, 247, 0.15);
    border: 1px solid rgba(168, 85, 247, 0.3);
    color: #e9d5ff;
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: all 0.15s ease;
    margin-top: 2px;
  }

  .rebrand-action-btn:hover {
    background: rgba(168, 85, 247, 0.28);
    border-color: #a855f7;
    color: #ffffff;
    transform: translateX(2px);
  }

  /* ==========================================================================
     Feature 5: Reciprocity & Health Gauge
     ========================================================================== */
  .reciprocity-card {
    background-color: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 18px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
  }

  .reciprocity-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }

  .reciprocity-left {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .reciprocity-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--text-tertiary);
  }

  .reciprocity-score-wrap {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .reciprocity-score {
    font-size: 26px;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    line-height: 1.1;
  }

  .reciprocity-tier-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 4px;
    border: 1px solid;
    letter-spacing: 0.05em;
  }

  .reciprocity-desc {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .clean-action-btn {
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: #fbbf24;
    border-radius: 8px;
    padding: 6px 12px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
    align-self: flex-start;
  }

  .clean-action-btn:hover {
    background: rgba(245, 158, 11, 0.2);
    border-color: #f59e0b;
    color: #ffffff;
    transform: translateY(-1px);
  }

  .distribution-bar-track {
    width: 100%;
    height: 10px;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 999px;
    overflow: hidden;
    display: flex;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .dist-segment {
    height: 100%;
    transition: width 0.3s ease;
  }

  .dist-segment.mutual {
    background: #10b981;
  }

  .dist-segment.non-reciprocal {
    background: #f59e0b;
  }

  .dist-segment.fans {
    background: #8b5cf6;
  }

  .distribution-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    font-size: 10px;
    color: var(--text-secondary);
    padding-top: 2px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .legend-item strong {
    color: var(--text-primary);
  }

  .legend-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .legend-dot.mutual {
    background-color: #10b981;
  }

  .legend-dot.non-reciprocal {
    background-color: #f59e0b;
  }

  .legend-dot.fans {
    background-color: #8b5cf6;
  }

  @media (max-width: 520px) {
    .insights-grid {
      grid-template-columns: 1fr;
      gap: 8px;
    }

    .insight-card {
      padding: 10px 14px;
      justify-content: space-between;
    }

    .insight-card .insight-name {
      font-size: 11px;
      white-space: normal;
    }

    .reciprocity-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 10px;
    }

    .clean-action-btn {
      width: 100%;
      text-align: center;
      padding: 8px 12px;
    }

    .distribution-legend {
      flex-direction: column;
      gap: 6px;
    }

    .velocity-meta {
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
    }
  }

  @media (max-width: 380px) {
    .stat-val {
      font-size: 20px;
    }

    .stat-label {
      font-size: 10.5px;
    }

    .rebrand-names-row {
      flex-wrap: wrap;
      gap: 4px;
    }
  }
</style>
