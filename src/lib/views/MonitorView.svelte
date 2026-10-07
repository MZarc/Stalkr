<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Account, type ProviderHealthStatus, type SyncReport } from '../api';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';
  import { globalSync } from '../syncState.svelte';

  let {
    account,
    ownerAccount = null,
  }: {
    account: Account;
    ownerAccount?: Account | null;
  } = $props();

  let health = $state<ProviderHealthStatus>({
    provider_name: 'Authenticated Session',
    is_connected: true,
    status_text: 'Verified and operational',
    follower_retrieval_ok: true,
    following_retrieval_ok: true,
    pagination_ok: true,
    completeness_check_ok: true,
    provider_version: 'Session Provider 1.0 (Oct 2026)',
  });

  let selectedInterval = $state('60');
  const isSyncing = $derived(globalSync.isSyncing && globalSync.activeAccountId === account.id);
  const syncState = $derived(globalSync.activeAccountId === account.id ? globalSync.syncState : 'IDLE');
  const syncReport = $derived(globalSync.activeAccountId === account.id ? globalSync.lastReport : null);

  const intervals = [
    { id: 'off', label: 'Off (Manual only)', tag: 'OFF' },
    { id: '15', label: 'Every 15–25 min (Randomized jitter)', tag: '15M' },
    { id: '30', label: 'Every 25–40 min (Balanced jitter)', tag: '30M' },
    { id: '60', label: 'Every 50–90 min (Recommended, stealth anti-bot)', tag: '1H', isRec: true },
    { id: '360', label: 'Every 5–7 hours (Battery saver)', tag: '6H' },
    { id: '1440', label: 'Daily (Randomized 20–28h window)', tag: '24H' },
  ];

  async function loadHealth() {
    try {
      health = await api.getProviderHealth(account.id);
      const saved = await api.getSetting(`interval_${account.id}`);
      if (saved) selectedInterval = saved;
    } catch (e) {
      console.error('Failed to load health:', e);
    }
  }

  $effect(() => {
    if (account?.id) {
      loadHealth();
    }
  });

  async function handleIntervalChange(val: string) {
    selectedInterval = val;
    try {
      await api.setSetting(`interval_${account.id}`, val);
      const minutes = val === 'off' ? 0 : parseInt(val, 10);
      if (!isNaN(minutes)) {
        await api.scheduleBackgroundSync(account.id, minutes);
      }
    } catch (e) {
      console.error('Failed to save interval:', e);
    }
  }

  $effect(() => {
    if (globalSync.lastSyncAt && globalSync.activeAccountId === account.id) {
      loadHealth();
    }
  });

  async function runManualSync() {
    if (isSyncing) return;
    try {
      await globalSync.executeSync(account);
      await loadHealth();
    } catch (e) {
      console.error('Manual sync failed:', e);
    }
  }

  function formatRelativeTime(ts?: number | null): string {
    if (!ts) return 'Never';
    const diffSec = Math.max(0, Math.floor(Date.now() / 1000 - ts));
    if (diffSec < 45) return 'Just now';
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
    return new Date(ts * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  onMount(() => {
    loadHealth();
  });
</script>

<div class="monitor-view fade-in">
  <!-- Target Account Context Strip -->
  <header class="account-context-strip">
    <div class="avatar-wrap">
      <img
        src={getAvatarUrl(account.username, account.avatar_url)}
        alt={account.username}
        class="account-avatar"
        loading="lazy"
        referrerpolicy="no-referrer"
        onerror={(e) => {
          (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(account.username);
        }}
      />
      <span class="status-dot {health.is_connected ? 'online' : 'offline'}"></span>
    </div>

    <div class="account-details">
      <div class="handle-row">
        <h3 class="account-handle">@{account.username}</h3>
        {#if account.is_verified}
          <span class="verified-glyph" title="Verified">✓</span>
        {/if}
        <span class="account-role-tag mono">
          {account.account_kind === 'owner' ? 'OWNER' : 'TARGET'}
        </span>
      </div>

      <div class="account-stats-row mono">
        <span>{account.followers_count.toLocaleString()} followers</span>
        <span class="sep">·</span>
        <span>{account.following_count.toLocaleString()} following</span>
        {#if ownerAccount && account.account_kind === 'monitored'}
          <span class="sep">·</span>
          <span class="monitored-by">via @{ownerAccount.username}</span>
        {/if}
      </div>
    </div>

    <div class="sync-last-time mono">
      <span class="last-label">LAST SYNC</span>
      <span class="last-val">{formatRelativeTime(account.last_successful_sync_at)}</span>
    </div>
  </header>

  <!-- 1. State Machine Visualizer Card -->
  <div class="card state-machine-card">
    <div class="card-header">
      <span class="card-title mono">SYNC ENGINE STATE MACHINE</span>
      <span class="state-badge {syncState.toLowerCase()} mono">
        {#if isSyncing}
          <span class="badge-dot-pulse"></span>
        {/if}
        {syncState}
      </span>
    </div>

    <!-- Interactive Horizontal Flow Chart -->
    <div class="flow-chart mono">
      <div class="node {syncState === 'IDLE' ? 'active' : ''}">IDLE</div>
      <span class="arrow">→</span>
      <div class="node {syncState === 'CONNECTING' ? 'active' : ''}">CONNECT</div>
      <span class="arrow">→</span>
      <div class="node {syncState === 'FETCHING' ? 'active' : ''}">FETCH</div>
      <span class="arrow">→</span>
      <div class="node {syncState === 'VALIDATING' ? 'active' : ''}">VERIFY</div>
      <span class="arrow">→</span>
      <div class="node {syncState === 'DIFFING' ? 'active' : ''}">DIFF</div>
      <span class="arrow">→</span>
      <div class="node {syncState === 'COMPLETE' ? 'active' : ''}">COMMIT</div>
    </div>

    <div class="sync-action-row">
      <button
        class="primary-sync-btn mono"
        onclick={runManualSync}
        disabled={isSyncing}
        type="button"
      >
        {#if isSyncing}
          <span class="spinner-circle"></span>
          <span>EXECUTING SYNC CYCLE...</span>
        {:else}
          <span class="sync-glyph">⟳</span>
          <span>TRIGGER IMMEDIATE SYNC</span>
        {/if}
      </button>
    </div>

    {#if syncReport}
      <div class="report-box mono {syncReport.success ? 'success' : 'warn'} fade-in">
        <div class="report-status">
          <span>STATUS: {syncReport.status.toUpperCase()}</span>
          <span class="report-time">JUST NOW</span>
        </div>
        <div class="report-msg">{syncReport.message}</div>
        {#if syncReport.changes_detected > 0}
          <div class="report-changes">✓ {syncReport.changes_detected} confirmed change(s) committed</div>
        {:else if syncReport.success}
          <div class="report-changes">✓ Baseline updated · No new changes detected</div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- 2. Android WorkManager Schedule Card -->
  <div class="card schedule-card">
    <div class="card-header">
      <span class="card-title mono">ANDROID WORKMANAGER SCHEDULE</span>
      <span class="tag jitter mono">🛡️ STEALTH JITTER ACTIVE</span>
    </div>

    <div class="interval-options">
      {#each intervals as item}
        {@const isChecked = selectedInterval === item.id}
        <button
          class="interval-option-row {isChecked ? 'selected' : ''}"
          onclick={() => handleIntervalChange(item.id)}
          type="button"
          role="radio"
          aria-checked={isChecked}
        >
          <div class="custom-radio-circle {isChecked ? 'checked' : ''}">
            {#if isChecked}
              <div class="radio-core-dot"></div>
            {/if}
          </div>
          <span class="interval-label">{item.label}</span>
          {#if item.isRec}
            <span class="rec-badge mono">REC</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="stealth-info-box">
      <div class="stealth-header">
        <span class="shield-icon">🛡️</span>
        <span class="stealth-title mono">ANTI-BOT RANDOMIZED TIMING</span>
      </div>
      <p class="stealth-desc">
        To prevent Instagram heuristics from detecting static heartbeat patterns, sync intervals are dynamically randomized (±20%–35%). For example, a 1-hour schedule triggers randomly between <strong>50 and 90 minutes</strong>.
      </p>
    </div>

    <div class="footnote mono">
      * Note: Android WorkManager enforces a 15-minute minimum periodic interval. Execution occurs according to system power scheduling and Android Doze heuristics.
    </div>
  </div>
</div>

<style>
  .monitor-view {
    padding: 14px 12px 16px 12px;
    max-width: 640px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* Target Account Context Strip */
  .account-context-strip {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
  }

  .avatar-wrap {
    position: relative;
    width: 40px;
    height: 40px;
    flex-shrink: 0;
  }

  .account-avatar {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    border: 1.5px solid rgba(168, 85, 247, 0.35);
  }

  .status-dot {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid var(--bg-surface);
  }

  .status-dot.online {
    background-color: var(--accent-positive);
    box-shadow: 0 0 6px var(--accent-positive);
  }

  .status-dot.offline {
    background-color: var(--accent-negative);
  }

  .account-details {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }

  .handle-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .account-handle {
    font-size: 14.5px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .verified-glyph {
    color: #38bdf8;
    font-size: 11px;
    font-weight: 700;
  }

  .account-role-tag {
    font-size: 8.5px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(168, 85, 247, 0.16);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.3);
  }

  .account-stats-row {
    font-size: 11px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
  }

  .sep {
    color: var(--text-tertiary);
  }

  .monitored-by {
    color: #c084fc;
  }

  .sync-last-time {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 1px;
    flex-shrink: 0;
  }

  .last-label {
    font-size: 8.5px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.4px;
  }

  .last-val {
    font-size: 11px;
    font-weight: 600;
    color: #6ee7b7;
  }

  /* General Luxury Obsidian Card */
  .card {
    background-color: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    padding: 15px;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 13px;
    gap: 8px;
    flex-wrap: wrap;
  }

  .card-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    letter-spacing: 0.05em;
  }

  .title-with-badge {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-indicator {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }

  .status-indicator.healthy {
    background-color: var(--accent-positive);
    box-shadow: 0 0 6px var(--accent-positive);
  }

  .status-indicator.degraded {
    background-color: var(--accent-signal);
  }

  .state-badge {
    font-size: 9.5px;
    padding: 3px 8px;
    border-radius: 6px;
    font-weight: 700;
    letter-spacing: 0.4px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .state-badge.idle {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .state-badge.connecting,
  .state-badge.fetching,
  .state-badge.validating,
  .state-badge.diffing {
    background: rgba(168, 85, 247, 0.22);
    color: #f3e8ff;
    border: 1px solid #a855f7;
  }

  .state-badge.complete {
    background: rgba(16, 185, 129, 0.2);
    color: #6ee7b7;
    border: 1px solid rgba(16, 185, 129, 0.35);
  }

  .badge-dot-pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #c084fc;
    box-shadow: 0 0 6px #c084fc;
    animation: badgePulse 1s ease-in-out infinite alternate;
  }

  @keyframes badgePulse {
    from { opacity: 0.5; transform: scale(0.9); }
    to { opacity: 1; transform: scale(1.2); }
  }

  .version-tag, .tag {
    font-size: 9px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-subtle);
    padding: 2px 7px;
    border-radius: 4px;
    color: var(--text-tertiary);
    font-weight: 600;
  }

  /* Sleek Flow Chart */
  .flow-chart {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background-color: var(--bg-root);
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
    font-size: 9.5px;
    margin-bottom: 14px;
    overflow-x: auto;
    scrollbar-width: none;
    gap: 4px;
  }

  .node {
    padding: 4px 8px;
    border-radius: 5px;
    color: var(--text-tertiary);
    background: transparent;
    transition: all 0.2s ease;
    white-space: nowrap;
    font-weight: 600;
  }

  .node.active {
    background: rgba(168, 85, 247, 0.25);
    color: #f3e8ff;
    border: 1px solid #a855f7;
    box-shadow: 0 0 10px rgba(168, 85, 247, 0.3);
    font-weight: 700;
  }

  .arrow {
    color: rgba(168, 85, 247, 0.4);
    font-size: 11px;
    user-select: none;
  }

  .sync-action-row {
    display: flex;
    justify-content: flex-end;
  }

  /* Primary Trigger Button */
  .primary-sync-btn {
    width: 100%;
    min-height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.24) 0%, rgba(126, 34, 206, 0.36) 100%);
    border: 1px solid #a855f7;
    color: #ffffff;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.5px;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.16s ease;
    user-select: none;
  }

  .primary-sync-btn:not(:disabled):hover {
    background: linear-gradient(135deg, rgba(192, 132, 252, 0.35) 0%, rgba(147, 51, 234, 0.48) 100%);
    box-shadow: 0 4px 16px rgba(168, 85, 247, 0.3);
    transform: translateY(-1px);
  }

  .primary-sync-btn:not(:disabled):active {
    transform: translateY(1px);
  }

  .primary-sync-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .sync-glyph {
    font-size: 15px;
    color: #e9d5ff;
    line-height: 1;
  }

  .spinner-circle {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Report Box */
  .report-box {
    margin-top: 12px;
    padding: 12px 14px;
    border-radius: 8px;
    font-size: 11px;
    border: 1px solid;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .report-box.success {
    background-color: rgba(16, 185, 129, 0.08);
    border-color: rgba(16, 185, 129, 0.35);
    color: var(--text-primary);
  }

  .report-box.warn {
    background-color: rgba(245, 158, 11, 0.08);
    border-color: rgba(245, 158, 11, 0.35);
    color: var(--text-primary);
  }

  .report-status {
    display: flex;
    justify-content: space-between;
    font-weight: 700;
    font-size: 10px;
    color: var(--text-primary);
  }

  .report-time {
    color: var(--text-tertiary);
  }

  .report-msg {
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .report-changes {
    color: #6ee7b7;
    font-weight: 600;
    font-size: 10.5px;
    margin-top: 2px;
  }

  /* Provider Health */
  .health-grid {
    display: flex;
    flex-direction: column;
    gap: 9px;
  }

  .health-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11.5px;
    padding-bottom: 7px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .health-row:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .h-label {
    color: var(--text-secondary);
  }

  .h-val {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .h-val.ok {
    color: #6ee7b7;
  }

  /* Schedule Options */
  .interval-options {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }

  .interval-option-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    background: rgba(14, 10, 24, 0.6);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition: all 0.14s ease;
    user-select: none;
    width: 100%;
  }

  .interval-option-row:hover {
    background: rgba(24, 17, 44, 0.85);
    border-color: rgba(168, 85, 247, 0.3);
  }

  .interval-option-row.selected {
    background: rgba(168, 85, 247, 0.16);
    border-color: #a855f7;
  }

  .custom-radio-circle {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: all 0.15s ease;
  }

  .custom-radio-circle.checked {
    border-color: #a855f7;
  }

  .radio-core-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #c084fc;
    box-shadow: 0 0 6px #c084fc;
  }

  .interval-label {
    font-size: 12px;
    color: var(--text-primary);
    font-weight: 500;
    flex: 1;
  }

  .interval-option-row.selected .interval-label {
    color: #f3e8ff;
    font-weight: 600;
  }

  .rec-badge {
    font-size: 8px;
    font-weight: 800;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(168, 85, 247, 0.25);
    color: #e9d5ff;
    border: 1px solid rgba(168, 85, 247, 0.4);
  }

  .footnote {
    font-size: 9.5px;
    color: var(--text-tertiary);
    line-height: 1.45;
  }

  .tag.jitter {
    background: rgba(16, 185, 129, 0.12);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .stealth-info-box {
    background: rgba(16, 185, 129, 0.05);
    border: 1px solid rgba(16, 185, 129, 0.18);
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .stealth-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .shield-icon {
    font-size: 13px;
  }

  .stealth-title {
    font-size: 10px;
    font-weight: 700;
    color: #34d399;
    letter-spacing: 0.05em;
  }

  .stealth-desc {
    margin: 0;
    font-size: 10.5px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .stealth-desc strong {
    color: #f3e8ff;
    font-weight: 600;
  }
</style>
