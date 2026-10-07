<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Account } from '../api';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';
  import { promptBiometricAuth } from '../biometrics';
  import { openExternalUrl } from '../instagramAuth';
  import Icon from '../components/Icon.svelte';

  let {
    account,
    accounts,
    onSwitchAccount,
    onRefreshAccounts,
    onLogout,
    onAddTarget,
  }: {
    account: Account;
    accounts: Account[];
    onSwitchAccount: (acc: Account) => void;
    onRefreshAccounts: () => void;
    onLogout?: () => void;
    onAddTarget?: () => void;
  } = $props();

  // File import state
  let followersJsonContent = $state('');
  let followingJsonContent = $state('');
  let followersFileName = $state('');
  let followingFileName = $state('');
  let followersFileSize = $state('');
  let followingFileSize = $state('');
  let importStatus = $state<string | null>(null);
  let importSuccess = $state<boolean | null>(null);
  let isImporting = $state(false);

  // Manual Session override (collapsible)
  let showManualSession = $state(false);
  let sessionId = $state('');
  let dsUserId = $state('');
  let csrfToken = $state('');
  let sessionSaveStatus = $state<string | null>(null);

  // Security & App Lock
  let biometricLockEnabled = $state(false);
  let isTogglingBio = $state(false);
  let bioFeedbackMsg = $state<string | null>(null);

  // Delete Target confirmation
  let deleteTargetConfirmOpen = $state(false);
  let isDeletingTarget = $state(false);

  // Nuclear Purge confirmation
  let purgeConfirmOpen = $state(false);
  let isPurging = $state(false);

  async function loadSettings() {
    try {
      const bio = await api.getSetting('biometric_lock');
      biometricLockEnabled = bio === 'true';

      const sid = await api.getSetting(`session_id_${account.id}`);
      sessionId = sid || '';

      const uid = await api.getSetting(`ds_user_id_${account.id}`);
      dsUserId = uid || '';

      const csrf = await api.getSetting(`csrftoken_${account.id}`);
      csrfToken = csrf || '';
    } catch (e) {
      console.error('Failed to load settings:', e);
    }
  }

  $effect(() => {
    if (account?.id) {
      loadSettings();
    }
  });

  function handleFollowersFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) {
      followersFileName = file.name;
      followersFileSize = (file.size / 1024).toFixed(1) + ' KB';
      const reader = new FileReader();
      reader.onload = (event) => {
        followersJsonContent = event.target?.result as string;
      };
      reader.readAsText(file);
    }
  }

  function handleFollowingFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) {
      followingFileName = file.name;
      followingFileSize = (file.size / 1024).toFixed(1) + ' KB';
      const reader = new FileReader();
      reader.onload = (event) => {
        followingJsonContent = event.target?.result as string;
      };
      reader.readAsText(file);
    }
  }

  function clearFollowersFile() {
    followersJsonContent = '';
    followersFileName = '';
    followersFileSize = '';
  }

  function clearFollowingFile() {
    followingJsonContent = '';
    followingFileName = '';
    followingFileSize = '';
  }

  async function runExportImport() {
    if (!followersJsonContent || !followingJsonContent) {
      importStatus = 'Please select both followers and following JSON files first.';
      importSuccess = false;
      return;
    }

    isImporting = true;
    importStatus = 'Parsing official JSON archives & compiling relationship differential...';
    importSuccess = null;
    try {
      const res = await api.importRawExportContent(account.id, followersJsonContent, followingJsonContent);
      importStatus = res.message || 'Import completed successfully.';
      importSuccess = true;
      onRefreshAccounts();
    } catch (e: any) {
      importStatus = 'Import error: ' + (e?.message || e);
      importSuccess = false;
    } finally {
      isImporting = false;
    }
  }

  async function saveSessionCredentials() {
    try {
      await api.setSetting(`session_id_${account.id}`, sessionId);
      await api.setSetting(`ds_user_id_${account.id}`, dsUserId);
      if (csrfToken) {
        await api.setSetting(`csrftoken_${account.id}`, csrfToken);
      }
      sessionSaveStatus = '✓ Session saved in Android Keystore Key A.';
      setTimeout(() => { sessionSaveStatus = null; }, 3500);
      onRefreshAccounts();
    } catch (e: any) {
      sessionSaveStatus = 'Failed to save: ' + (e?.message || e);
    }
  }

  async function toggleBiometricLock() {
    if (isTogglingBio) return;
    isTogglingBio = true;
    bioFeedbackMsg = null;
    try {
      const nextState = !biometricLockEnabled;
      if (nextState) {
        // Authenticate biometrically on phone before enabling lock
        const res = await promptBiometricAuth(
          'Enable Biometric Lock',
          'Scan your fingerprint, face, or enter device PIN to activate'
        );
        if (!res.success) {
          bioFeedbackMsg = res.error || 'Authentication cancelled. Lock not enabled.';
          return;
        }
      }
      await api.setSetting('biometric_lock', nextState ? 'true' : 'false');
      biometricLockEnabled = nextState;
      bioFeedbackMsg = nextState ? '✓ Biometric app lock activated.' : 'Biometric app lock turned off.';
      setTimeout(() => { bioFeedbackMsg = null; }, 3000);
    } catch (e: any) {
      console.error('Failed to toggle biometric lock:', e);
      bioFeedbackMsg = 'Error: ' + (e?.message || e);
    } finally {
      isTogglingBio = false;
    }
  }

  // Connected Targets management
  const rootAccounts = $derived(accounts.filter((a) => a.account_kind === 'owner'));
  const trackedTargets = $derived(accounts.filter((a) => a.account_kind === 'monitored'));
  const activeIsTarget = $derived(account.account_kind === 'monitored');
  const rootOwnerAccount = $derived(
    accounts.find((a) => a.id === account.authenticated_by_account_id) || rootAccounts[0]
  );

  let targetToDeleteId = $state<string | null>(null);
  let isDeletingTargetId = $state<string | null>(null);

  function formatRelativeTime(ts?: number | null): string {
    if (!ts) return 'Pending initial sync';
    const now = Math.floor(Date.now() / 1000);
    const diff = now - ts;
    if (diff < 60) return 'Just now';
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  async function handleUntrackTarget(target: Account) {
    isDeletingTargetId = target.id;
    try {
      await api.deleteAccount(target.id);
      targetToDeleteId = null;
      onRefreshAccounts();
      if (account.id === target.id && rootAccounts.length > 0) {
        onSwitchAccount(rootAccounts[0]);
      }
    } catch (e: any) {
      alert('Failed to untrack target: ' + (e?.message || e));
    } finally {
      isDeletingTargetId = null;
    }
  }

  async function handleDeleteCurrentTarget() {
    if (account.account_kind === 'owner') return;
    isDeletingTarget = true;
    try {
      await api.deleteAccount(account.id);
      deleteTargetConfirmOpen = false;
      onRefreshAccounts();
      if (rootAccounts.length > 0) {
        onSwitchAccount(rootAccounts[0]);
      }
    } catch (e: any) {
      alert('Failed to remove target: ' + (e?.message || e));
    } finally {
      isDeletingTarget = false;
    }
  }

  async function handleNuclearPurge() {
    isPurging = true;
    try {
      await api.deleteAllLocalData();
      purgeConfirmOpen = false;
      if (onLogout) {
        onLogout();
      } else {
        onRefreshAccounts();
      }
    } catch (e: any) {
      alert('Purge error: ' + (e?.message || e));
    } finally {
      isPurging = false;
    }
  }

  onMount(() => {
    loadSettings();
  });
</script>

<div class="settings-view fade-in">
  <!-- 1. Active Account Profile Card -->
  <div class="card profile-card">
    <div class="card-header">
      <span class="card-title mono">ACTIVE PROFILE & CREDENTIALS</span>
      <span class="tag mono {account.account_kind === 'owner' ? 'owner-tag' : 'target-tag'}">
        {account.account_kind === 'owner' ? 'ROOT PROFILE' : 'TRACKED TARGET'}
      </span>
    </div>

    <div class="profile-hero-row">
      <div class="profile-avatar-wrap">
        <img
          src={getAvatarUrl(account.username, account.avatar_url)}
          alt={account.username}
          class="profile-avatar-img"
          onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(account.username); }}
        />
        {#if account.account_kind === 'owner'}
          <span class="owner-star-badge" title="Root Connected Profile">★</span>
        {/if}
      </div>

      <div class="profile-info-col">
        <div class="profile-handle-row">
          <span class="profile-handle">@{account.username}</span>
          {#if account.is_verified}
            <span class="verified-badge" title="Verified">✓</span>
          {/if}
        </div>
        <span class="profile-display-name">{account.display_name || account.username}</span>
        <div class="profile-counts mono">
          <span><strong>{account.followers_count.toLocaleString()}</strong> followers</span>
          <span class="dot-sep">•</span>
          <span><strong>{account.following_count.toLocaleString()}</strong> following</span>
        </div>
      </div>
    </div>

    {#if (activeIsTarget && rootOwnerAccount) || account.account_kind === 'monitored'}
      <div class="profile-actions-bar">
        {#if activeIsTarget && rootOwnerAccount}
          <button
            class="action-pill-btn switch-root-btn mono"
            onclick={() => onSwitchAccount(rootOwnerAccount)}
            type="button"
            title="Switch view back to @{rootOwnerAccount.username}"
          >
            <Icon name="arrow-left" size={12} strokeWidth={2.4} />
            <span>BACK TO @{rootOwnerAccount.username}</span>
          </button>
        {/if}

        {#if account.account_kind === 'monitored'}
          {#if !deleteTargetConfirmOpen}
            <button
              class="action-pill-btn danger-subtle mono"
              onclick={() => (deleteTargetConfirmOpen = true)}
              type="button"
            >
              <Icon name="trash" size={13} strokeWidth={2.4} />
              <span>UNTRACK TARGET</span>
            </button>
          {:else}
            <div class="confirm-inline-strip">
              <span class="confirm-query mono">Stop tracking @{account.username}?</span>
              <div class="confirm-actions">
                <button
                  class="confirm-btn-yes mono"
                  onclick={handleDeleteCurrentTarget}
                  disabled={isDeletingTarget}
                >
                  {isDeletingTarget ? 'REMOVING...' : 'YES, REMOVE'}
                </button>
                <button
                  class="confirm-btn-no mono"
                  onclick={() => (deleteTargetConfirmOpen = false)}
                  disabled={isDeletingTarget}
                >
                  CANCEL
                </button>
              </div>
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  </div>

  <!-- 2. Connected Targets & Hunt List Card -->
  <div class="card targets-card">
    <div class="card-header">
      <div class="card-title-group">
        <span class="card-title mono">CONNECTED TARGETS</span>
        <span class="target-badge-pill mono">{trackedTargets.length} TRACKED</span>
      </div>
      {#if onAddTarget}
        <button
          class="header-action-btn mono"
          onclick={onAddTarget}
          type="button"
        >
          <Icon name="plus" size={12} strokeWidth={2.6} />
          <span>TRACK TARGET</span>
        </button>
      {/if}
    </div>

    <p class="section-desc">
      Instagram accounts being actively monitored for follower gains, unfollowers, and circle shifts.
    </p>

    {#if trackedTargets.length === 0}
      <div class="empty-targets-box">
        <div class="empty-targets-icon">
          <Icon name="users" size={26} strokeWidth={1.8} color="#a855f7" />
        </div>
        <div class="empty-targets-title">No Connected Targets</div>
        <p class="empty-targets-desc">
          Add any private friend you follow or any public creator to monitor relationship additions, drops, and activity pulses.
        </p>
        {#if onAddTarget}
          <button class="track-first-target-btn mono" onclick={onAddTarget} type="button">
            <Icon name="plus" size={14} strokeWidth={2.4} />
            <span>START TARGET HUNT</span>
          </button>
        {/if}
      </div>
    {:else}
      <div class="targets-list-grid">
        {#each trackedTargets as target (target.id)}
          <div class="target-item-card {account.id === target.id ? 'is-active' : ''}">
            <!-- Card Header: Avatar on left, Status + Untrack on right -->
            <div class="target-card-header">
              <div class="target-avatar-wrap">
                <img
                  src={getAvatarUrl(target.username, target.avatar_url)}
                  alt={target.username}
                  class="target-avatar-img"
                  onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(target.username); }}
                />
                {#if target.is_private}
                  <span class="target-lock-badge" title="Private Account">🔒</span>
                {/if}
              </div>

              <div class="target-header-meta">
                {#if account.id === target.id}
                  <span class="badge-active-live mono">ACTIVE</span>
                {:else}
                  <div class="sync-status-pill mono" title="Connection status">
                    <span class="sync-dot {target.access_state === 'accessible' ? 'ok' : 'warn'}"></span>
                    <span>{formatRelativeTime(target.last_successful_sync_at)}</span>
                  </div>
                {/if}

                {#if targetToDeleteId !== target.id}
                  <button
                    class="btn-icon-untrack"
                    onclick={() => (targetToDeleteId = target.id)}
                    title="Untrack @{target.username}"
                    type="button"
                  >
                    <Icon name="trash" size={13} strokeWidth={2.2} />
                  </button>
                {/if}
              </div>
            </div>

            <!-- Identity: Handle & Display Name -->
            <div class="target-identity-block">
              <div class="target-handle-row">
                <span class="target-handle">@{target.username}</span>
                {#if target.is_verified}
                  <span class="verified-dot" title="Verified">✓</span>
                {/if}
              </div>
              <div class="target-name-line">{target.display_name || target.username}</div>
            </div>

            <!-- Dual Stats Architecture Box -->
            <div class="stats-dual-box mono">
              <div class="stat-col">
                <span class="stat-val tabular">{target.followers_count.toLocaleString()}</span>
                <span class="stat-lbl">FOLLOWERS</span>
              </div>
              <div class="stat-divider"></div>
              <div class="stat-col">
                <span class="stat-val tabular">{target.following_count.toLocaleString()}</span>
                <span class="stat-lbl">FOLLOWING</span>
              </div>
            </div>

            <!-- Inline Untrack Confirmation (if triggered) -->
            {#if targetToDeleteId === target.id}
              <div class="target-inline-confirm">
                <span class="confirm-ask mono">Untrack target?</span>
                <div class="confirm-btn-row">
                  <button
                    class="btn-confirm-yes mono"
                    onclick={() => handleUntrackTarget(target)}
                    disabled={isDeletingTargetId === target.id}
                    type="button"
                  >
                    {isDeletingTargetId === target.id ? '...' : 'YES, REMOVE'}
                  </button>
                  <button
                    class="btn-confirm-no mono"
                    onclick={() => (targetToDeleteId = null)}
                    disabled={isDeletingTargetId === target.id}
                    type="button"
                  >
                    CANCEL
                  </button>
                </div>
              </div>
            {/if}

            <!-- Bottom Action Button -->
            <div class="target-bottom-action">
              {#if account.id !== target.id}
                <button
                  class="target-btn-switch mono"
                  onclick={() => onSwitchAccount(target)}
                  type="button"
                  title="Switch active view to @{target.username}"
                >
                  <Icon name="zap" size={12} strokeWidth={2.4} />
                  <span>VIEW PULSE</span>
                </button>
              {:else}
                <div class="target-active-status mono">
                  <span class="active-dot"></span>
                  <span>CURRENTLY ACTIVE</span>
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 2. Official Instagram Data Export Ingestion (100% Offline & Safe) -->
  <div class="card export-ingest-card">
    <div class="card-header">
      <span class="card-title mono">OFFICIAL META DATA ARCHIVE INGESTION</span>
      <span class="safe-badge mono">100% OFFLINE · ZERO RISK</span>
    </div>

    <p class="section-desc">
      Import your official Meta account data export downloaded from Instagram Accounts Center. Completely local, zero credentials used, zero risk of rate limits.
    </p>

    <!-- Guided Steps Pill -->
    <div class="guide-box">
      <div class="guide-step">
        <span class="step-num mono">1</span>
        <span>Instagram App → <strong>Accounts Center</strong> → <strong>Your info & permissions</strong></span>
      </div>
      <div class="guide-step">
        <span class="step-num mono">2</span>
        <span><strong>Download information</strong> → Format: <strong>JSON</strong> → Select <strong>Followers & following</strong></span>
      </div>
    </div>

    <!-- Upload Tap Cards -->
    <div class="file-picker-grid">
      <!-- File 1: Followers -->
      <div class="file-card {followersJsonContent ? 'loaded' : ''}">
        <label class="file-card-label" for="followers-file-picker">
          <div class="file-card-left">
            <div class="file-icon-box">
              {#if followersJsonContent}
                <span class="icon-ok">✓</span>
              {:else}
                <span class="icon-folder">📁</span>
              {/if}
            </div>
            <div class="file-card-info">
              <span class="file-slot-name mono">1. FOLLOWERS JSON</span>
              {#if followersFileName}
                <span class="file-loaded-name">{followersFileName} ({followersFileSize})</span>
              {:else}
                <span class="file-placeholder">Tap to select followers_1.json</span>
              {/if}
            </div>
          </div>
          <input
            id="followers-file-picker"
            type="file"
            accept=".json"
            onchange={handleFollowersFile}
            class="hidden-file-input"
          />
        </label>
        {#if followersJsonContent}
          <button class="clear-file-btn" onclick={clearFollowersFile} title="Clear file" type="button">✕</button>
        {/if}
      </div>

      <!-- File 2: Following -->
      <div class="file-card {followingJsonContent ? 'loaded' : ''}">
        <label class="file-card-label" for="following-file-picker">
          <div class="file-card-left">
            <div class="file-icon-box">
              {#if followingJsonContent}
                <span class="icon-ok">✓</span>
              {:else}
                <span class="icon-folder">📁</span>
              {/if}
            </div>
            <div class="file-card-info">
              <span class="file-slot-name mono">2. FOLLOWING JSON</span>
              {#if followingFileName}
                <span class="file-loaded-name">{followingFileName} ({followingFileSize})</span>
              {:else}
                <span class="file-placeholder">Tap to select following.json</span>
              {/if}
            </div>
          </div>
          <input
            id="following-file-picker"
            type="file"
            accept=".json"
            onchange={handleFollowingFile}
            class="hidden-file-input"
          />
        </label>
        {#if followingJsonContent}
          <button class="clear-file-btn" onclick={clearFollowingFile} title="Clear file" type="button">✕</button>
        {/if}
      </div>
    </div>

    <button
      class="primary-btn mono"
      onclick={runExportImport}
      disabled={isImporting || !followersJsonContent || !followingJsonContent}
      type="button"
    >
      {#if isImporting}
        <span class="spinner-inline"></span>
        <span>INGESTING ARCHIVE...</span>
      {:else}
        <span>INGEST RELATIONSHIP SNAPSHOT</span>
      {/if}
    </button>

    {#if importStatus}
      <div class="status-msg mono {importSuccess === true ? 'success' : importSuccess === false ? 'error' : ''}">
        {importStatus}
      </div>
    {/if}
  </div>

  <!-- 3. Direct Session Credentials (Clean Mobile Accordion) -->
  <div class="card session-card">
    <div class="card-header">
      <span class="card-title mono">AUTHENTICATED INSTAGRAM SESSION</span>
      <span class="tag mono hardware-tag">KEY A ENCRYPTED</span>
    </div>

    <p class="section-desc">
      Used for automatic background sync. Cookies are stored in the Android Hardware Keystore (Key A domain) and never leave your phone.
    </p>

    <!-- Advanced Manual Cookie Override Collapsible -->
    <div class="accordion-section">
      <button
        class="accordion-toggle-btn mono"
        onclick={() => (showManualSession = !showManualSession)}
        type="button"
        aria-expanded={showManualSession}
      >
        <span>MANUAL COOKIE OVERRIDE (ADVANCED)</span>
        <span class="accordion-arrow">{showManualSession ? '▲' : '▼'}</span>
      </button>

      {#if showManualSession}
        <div class="accordion-body slide-down">
          <div class="input-group">
            <label class="input-label mono" for="settings-session-id">SESSION ID (sessionid cookie)</label>
            <input
              id="settings-session-id"
              type="password"
              bind:value={sessionId}
              placeholder="••••••••••••••••••••••••••••••••"
              class="text-input mono"
              autocomplete="off"
            />
          </div>

          <div class="input-group">
            <label class="input-label mono" for="settings-user-id">USER ID (ds_user_id numeric ID)</label>
            <input
              id="settings-user-id"
              type="text"
              bind:value={dsUserId}
              placeholder="e.g. 17841400000000"
              class="text-input mono"
              autocomplete="off"
            />
          </div>

          <div class="input-group">
            <label class="input-label mono" for="settings-csrf-token">CSRF TOKEN (csrftoken cookie, optional)</label>
            <input
              id="settings-csrf-token"
              type="text"
              bind:value={csrfToken}
              placeholder="csrftoken value"
              class="text-input mono"
              autocomplete="off"
            />
          </div>

          <button class="save-creds-btn mono" onclick={saveSessionCredentials} type="button">
            SAVE SESSION CREDENTIALS
          </button>

          {#if sessionSaveStatus}
            <div class="status-msg mono success">{sessionSaveStatus}</div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- 4. Security & Biometrics -->
  <div class="card security-card">
    <div class="card-header">
      <span class="card-title mono">SECURITY & PRIVACY DOMAINS</span>
      <span class="tag mono hardware-tag">HARDWARE BACKED</span>
    </div>

    <div class="setting-toggle-row">
      <div class="toggle-info">
        <div class="setting-name">Biometric App Lock</div>
        <div class="setting-desc">Require Fingerprint or Device PIN when opening or resuming Stalkr.</div>
      </div>
      <button
        class="custom-switch-btn {biometricLockEnabled ? 'active' : ''}"
        onclick={toggleBiometricLock}
        type="button"
        role="switch"
        aria-checked={biometricLockEnabled}
        aria-label="Toggle Biometric App Lock"
      >
        <span class="switch-knob"></span>
      </button>
    </div>

    <div class="keystore-spec mono">
      <div class="keystore-row">
        <span class="key-domain">DOMAIN KEY A:</span>
        <span class="key-desc">Session Storage · AES-256-GCM Hardware Vault</span>
      </div>
      <div class="keystore-row">
        <span class="key-domain">DOMAIN KEY B:</span>
        <span class="key-desc">Biometric Lock & Private Notes · Auth-Bound</span>
      </div>
    </div>

    {#if bioFeedbackMsg}
      <div class="bio-feedback-msg mono">{bioFeedbackMsg}</div>
    {/if}
  </div>

  <!-- 5. About & Architecture Section -->
  <div class="card about-card">
    <div class="card-header">
      <div class="card-header-left">
        <span class="card-title mono">ABOUT THE ARCHITECT</span>
      </div>
      <span class="about-lead-badge mono">LEAD DEVELOPER</span>
    </div>

    <div class="about-hero">
      <div class="about-avatar-frame">
        <img
          src="/profile.png"
          alt="Meet Mistry (@meetzarc)"
          class="about-avatar-img"
          onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar('meetzarc'); }}
        />
        <span class="about-avatar-badge" title="Creator Verified">✦</span>
      </div>
      <div class="about-meta">
        <div class="about-kicker mono">FULL-STACK & MOBILE SYSTEMS</div>
        <h3 class="about-name">Meet Mistry</h3>
        <p class="about-bio">
          Software engineer focused on native mobile architectures, high-performance systems, and privacy-first local applications. Built with Rust, Tauri v2, and Svelte.
        </p>
      </div>
    </div>

    <div class="about-actions-grid">
      <!-- Portfolio Button -->
      <a
        href="https://meetmistry.vercel.app"
        target="_blank"
        rel="noopener noreferrer"
        class="about-action-btn portfolio-btn"
        title="Visit Portfolio: meetmistry.vercel.app"
        onclick={(e) => {
          e.preventDefault();
          openExternalUrl('https://meetmistry.vercel.app');
        }}
      >
        <div class="about-btn-icon-wrap">
          <Icon name="globe" size={17} color="#d8b4fe" strokeWidth={2.2} />
        </div>
        <div class="about-btn-text">
          <span class="btn-sub-label mono">PORTFOLIO</span>
          <span class="btn-main-label">meetmistry.vercel.app</span>
        </div>
        <span class="about-btn-arrow">↗</span>
      </a>

      <!-- Mail Button -->
      <a
        href="mailto:meetzarc@gmail.com"
        class="about-action-btn mail-btn"
        title="Send email: meetzarc@gmail.com"
        onclick={(e) => {
          e.preventDefault();
          openExternalUrl('mailto:meetzarc@gmail.com');
        }}
      >
        <div class="about-btn-icon-wrap">
          <Icon name="mail" size={17} color="#a855f7" strokeWidth={2.2} />
        </div>
        <div class="about-btn-text">
          <span class="btn-sub-label mono">DIRECT INQUIRY</span>
          <span class="btn-main-label">meetzarc@gmail.com</span>
        </div>
        <span class="about-btn-arrow">✉</span>
      </a>
    </div>

    <div class="about-footer mono">
      <span class="about-spec-badge">STALKR v1.0.0 · PRODUCTION STABLE</span>
      <span class="about-copy">CRAFTED BY MEET MISTRY · 2026</span>
    </div>
  </div>

  <!-- 7. Session & Logout Card -->
  <div class="card logout-card">
    <div class="card-header">
      <span class="card-title mono">SESSION & ACCOUNT LOGOUT</span>
      <span class="tag mono">READY</span>
    </div>

    <p class="section-desc">
      Disconnect your active profile session and return to the Stalkr welcome screen. All data remains encrypted on your phone.
    </p>

    <button
      class="logout-card-btn mono"
      onclick={onLogout}
      type="button"
    >
      <Icon name="logout" size={14} strokeWidth={2.4} />
      <span>LOG OUT OF STALKR</span>
    </button>
  </div>

  <!-- 7. Danger Zone / Data Reset -->
  <div class="card danger-card">
    <div class="card-header">
      <span class="card-title mono text-neg">DATA RESET & PURGE</span>
      <span class="danger-tag mono">DESTRUCTIVE</span>
    </div>

    <p class="section-desc">
      Permanently delete all accounts, snapshots, relationship histories, and encryption keys from this phone.
    </p>

    {#if !purgeConfirmOpen}
      <button class="danger-btn mono" onclick={() => (purgeConfirmOpen = true)} type="button">
        PURGE ALL LOCAL DATA
      </button>
    {:else}
      <div class="confirm-box">
        <span class="confirm-text">Are you sure? This will delete all local accounts, relationship diffs, and notes permanently.</span>
        <div class="confirm-btns">
          <button
            class="danger-confirm mono"
            onclick={handleNuclearPurge}
            disabled={isPurging}
            type="button"
          >
            {isPurging ? 'PURGING...' : 'CONFIRM PURGE'}
          </button>
          <button
            class="cancel-btn mono"
            onclick={() => (purgeConfirmOpen = false)}
            disabled={isPurging}
            type="button"
          >
            CANCEL
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-view {
    padding: 14px 12px 16px 12px;
    max-width: 600px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card {
    background: rgba(20, 14, 38, 0.72);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(168, 85, 247, 0.16);
    border-radius: 12px;
    padding: 16px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.35);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .card-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    letter-spacing: 0.06em;
  }

  .tag {
    font-size: 8.5px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-tertiary);
    border: 1px solid rgba(255, 255, 255, 0.1);
    letter-spacing: 0.04em;
  }

  .tag.owner-tag {
    background: rgba(168, 85, 247, 0.18);
    color: #e9d5ff;
    border-color: rgba(168, 85, 247, 0.4);
  }

  .tag.target-tag {
    background: rgba(192, 132, 252, 0.12);
    color: #d8b4fe;
    border-color: rgba(192, 132, 252, 0.3);
  }

  .tag.hardware-tag {
    background: rgba(16, 185, 129, 0.1);
    color: #34d399;
    border-color: rgba(16, 185, 129, 0.28);
  }

  .safe-badge {
    font-size: 8.5px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 4px;
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.35);
    letter-spacing: 0.04em;
  }

  .danger-tag {
    font-size: 8.5px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 4px;
    background: rgba(244, 63, 94, 0.15);
    color: #fca5a5;
    border: 1px solid rgba(244, 63, 94, 0.35);
    letter-spacing: 0.04em;
  }

  .section-desc {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0 0 14px 0;
    line-height: 1.45;
  }

  /* Profile Card */
  .profile-hero-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 6px 0 14px 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .profile-avatar-wrap {
    position: relative;
    width: 52px;
    height: 52px;
    flex-shrink: 0;
  }

  .profile-avatar-img {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    border: 2px solid rgba(168, 85, 247, 0.4);
    background: #140d28;
  }

  .owner-star-badge {
    position: absolute;
    bottom: -2px;
    right: -2px;
    background: linear-gradient(135deg, #a855f7 0%, #ec4899 100%);
    color: #fff;
    font-size: 9px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 0 6px rgba(168, 85, 247, 0.6);
  }

  .profile-info-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .profile-handle-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .profile-handle {
    font-size: 15px;
    font-weight: 700;
    color: #ffffff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .verified-badge {
    font-size: 10px;
    background: #0ea5e9;
    color: #ffffff;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
  }

  .profile-display-name {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .profile-counts {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 3px;
  }

  .profile-counts strong {
    color: #f3e8ff;
  }

  .dot-sep {
    color: var(--text-tertiary);
  }

  .profile-actions-bar {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-top: 14px;
    width: 100%;
  }

  .action-pill-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 10px;
    font-size: 11px;
    font-weight: 700;
    border-radius: 8px;
    cursor: pointer;
    min-height: 42px;
    width: 100%;
    box-sizing: border-box;
    text-align: center;
    touch-action: manipulation;
    transition: all 0.15s ease;
  }

  .action-pill-btn:active {
    transform: scale(0.98);
  }

  .action-pill-btn.danger-subtle {
    background: rgba(244, 63, 94, 0.1);
    border: 1px solid rgba(244, 63, 94, 0.25);
    color: #fca5a5;
  }

  .action-pill-btn.switch-root-btn {
    background: rgba(168, 85, 247, 0.15);
    border: 1px solid rgba(168, 85, 247, 0.35);
    color: #e9d5ff;
  }

  .action-pill-btn.switch-root-btn:hover {
    background: rgba(168, 85, 247, 0.25);
    color: #ffffff;
  }

  .confirm-inline-strip {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(244, 63, 94, 0.12);
    border: 1px solid rgba(244, 63, 94, 0.3);
    border-radius: 8px;
  }

  .confirm-query {
    font-size: 11px;
    color: #fca5a5;
    font-weight: 600;
  }

  .confirm-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .confirm-btn-yes {
    padding: 6px 10px;
    font-size: 10px;
    font-weight: 700;
    border-radius: 6px;
    background: #f43f5e;
    color: #fff;
    border: none;
    cursor: pointer;
  }

  .confirm-btn-no {
    padding: 6px 10px;
    font-size: 10px;
    font-weight: 600;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.1);
    color: #e2e8f0;
    border: 1px solid rgba(255, 255, 255, 0.15);
    cursor: pointer;
  }

  /* Connected Targets Card Header & Actions */
  .targets-card .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .card-title-group {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .target-badge-pill {
    font-size: 8.5px;
    font-weight: 700;
    padding: 2.5px 8px;
    border-radius: 6px;
    background: rgba(168, 85, 247, 0.18);
    color: #d8b4fe;
    border: 1px solid rgba(168, 85, 247, 0.35);
    letter-spacing: 0.04em;
  }

  .header-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 10px;
    font-weight: 700;
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.25) 0%, rgba(126, 34, 206, 0.18) 100%);
    border: 1px solid rgba(192, 132, 252, 0.45);
    color: #f3e8ff;
    border-radius: 8px;
    cursor: pointer;
    margin-left: auto;
    transition: all 0.18s ease;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
  }

  .header-action-btn:hover {
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.38) 0%, rgba(147, 51, 234, 0.28) 100%);
    border-color: rgba(216, 180, 254, 0.7);
    color: #ffffff;
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(168, 85, 247, 0.3);
  }

  .empty-targets-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 24px 16px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px dashed rgba(168, 85, 247, 0.25);
    border-radius: 10px;
    gap: 8px;
  }

  .empty-targets-icon {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: rgba(168, 85, 247, 0.12);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 4px;
  }

  .empty-targets-title {
    font-size: 13px;
    font-weight: 700;
    color: #ffffff;
  }

  .empty-targets-desc {
    font-size: 11.5px;
    color: var(--text-tertiary);
    line-height: 1.45;
    max-width: 380px;
    margin: 0;
  }

  .track-first-target-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
    padding: 8px 16px;
    font-size: 11px;
    font-weight: 700;
    border-radius: 8px;
    background: linear-gradient(135deg, #a855f7 0%, #7e22ce 100%);
    border: 1px solid rgba(216, 180, 254, 0.4);
    color: #ffffff;
    cursor: pointer;
    box-shadow: 0 4px 14px rgba(168, 85, 247, 0.3);
  }

  /* Connected Targets 2x2 Grid */
  /* Connected Targets 2x2 Grid */
  .targets-list-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  @media (max-width: 540px) {
    .targets-list-grid {
      grid-template-columns: 1fr;
    }
  }

  .target-item-card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 14px;
    background: rgba(22, 15, 40, 0.75);
    border: 1px solid rgba(168, 85, 247, 0.16);
    border-radius: 14px;
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    min-width: 0;
    box-sizing: border-box;
    position: relative;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.3);
  }

  .target-item-card:hover {
    border-color: rgba(168, 85, 247, 0.35);
    background: rgba(26, 18, 48, 0.85);
  }

  .target-item-card.is-active {
    background: linear-gradient(145deg, rgba(168, 85, 247, 0.18) 0%, rgba(35, 18, 65, 0.85) 100%);
    border-color: rgba(168, 85, 247, 0.55);
    box-shadow: 0 4px 22px rgba(168, 85, 247, 0.2);
  }

  .target-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .target-avatar-wrap {
    position: relative;
    width: 44px;
    height: 44px;
    flex-shrink: 0;
  }

  .target-avatar-img {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    border: 2px solid rgba(168, 85, 247, 0.4);
    background: #140d28;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.4);
  }

  .target-lock-badge {
    position: absolute;
    bottom: -2px;
    right: -2px;
    font-size: 10px;
    background: #090611;
    border-radius: 50%;
    padding: 2px;
    line-height: 1;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
  }

  .target-header-meta {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .badge-active-live {
    font-size: 8px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(168, 85, 247, 0.3);
    color: #f3e8ff;
    border: 1px solid rgba(168, 85, 247, 0.6);
    letter-spacing: 0.04em;
  }

  .sync-status-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 20px;
    font-size: 9px;
    color: var(--text-tertiary);
  }

  .sync-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .sync-dot.ok {
    background: #10b981;
    box-shadow: 0 0 6px rgba(16, 185, 129, 0.6);
  }

  .sync-dot.warn {
    background: #f59e0b;
    box-shadow: 0 0 6px rgba(245, 158, 11, 0.6);
  }

  .btn-icon-untrack {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-tertiary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-icon-untrack:hover {
    background: rgba(244, 63, 94, 0.15);
    border-color: rgba(244, 63, 94, 0.35);
    color: #fca5a5;
  }

  .target-identity-block {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 10px;
    min-width: 0;
  }

  .target-handle-row {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }

  .target-handle {
    font-size: 13.5px;
    font-weight: 700;
    color: #ffffff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .verified-dot {
    font-size: 8px;
    background: #0ea5e9;
    color: #ffffff;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 800;
    flex-shrink: 0;
  }

  .target-name-line {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Dual Stats Box */
  .stats-dual-box {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    background: rgba(0, 0, 0, 0.28);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    padding: 6px 8px;
    margin-bottom: 12px;
  }

  .stat-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
  }

  .stat-val {
    font-size: 13px;
    font-weight: 700;
    color: #f3e8ff;
  }

  .stat-lbl {
    font-size: 7.5px;
    color: var(--text-tertiary);
    letter-spacing: 0.05em;
  }

  .stat-divider {
    width: 1px;
    height: 20px;
    background: rgba(255, 255, 255, 0.08);
  }

  .target-inline-confirm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 6px 8px;
    background: rgba(244, 63, 94, 0.14);
    border: 1px solid rgba(244, 63, 94, 0.35);
    border-radius: 8px;
    margin-bottom: 10px;
    box-sizing: border-box;
    gap: 6px;
  }

  .confirm-ask {
    font-size: 9.5px;
    color: #fecdd3;
    font-weight: 600;
  }

  .confirm-btn-row {
    display: flex;
    gap: 4px;
  }

  .btn-confirm-yes {
    background: #e11d48;
    color: #ffffff;
    border: none;
    border-radius: 4px;
    padding: 3px 7px;
    font-size: 9px;
    font-weight: 700;
    cursor: pointer;
  }

  .btn-confirm-no {
    background: rgba(255, 255, 255, 0.12);
    color: var(--text-secondary);
    border: none;
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 9px;
    cursor: pointer;
  }

  .target-bottom-action {
    display: flex;
    width: 100%;
  }

  .target-btn-switch {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 700;
    border-radius: 8px;
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.3) 0%, rgba(126, 34, 206, 0.2) 100%);
    border: 1px solid rgba(168, 85, 247, 0.5);
    color: #ffffff;
    cursor: pointer;
    transition: all 0.16s ease;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }

  .target-btn-switch:hover {
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.45) 0%, rgba(126, 34, 206, 0.35) 100%);
    border-color: rgba(168, 85, 247, 0.75);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(168, 85, 247, 0.25);
  }

  .target-active-status {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 12px;
    font-size: 10px;
    font-weight: 700;
    border-radius: 8px;
    background: rgba(168, 85, 247, 0.12);
    border: 1px solid rgba(168, 85, 247, 0.3);
    color: #e9d5ff;
    letter-spacing: 0.04em;
  }

  .active-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #a855f7;
    box-shadow: 0 0 6px #a855f7;
  }



  .confirm-inline-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 10px;
    background: rgba(244, 63, 94, 0.12);
    border: 1px solid rgba(244, 63, 94, 0.3);
    border-radius: 8px;
    gap: 8px;
  }

  .confirm-query {
    font-size: 11px;
    color: #fecdd3;
  }

  .confirm-actions {
    display: flex;
    gap: 6px;
  }

  .confirm-btn-yes {
    background: #e11d48;
    color: #ffffff;
    border: none;
    border-radius: 6px;
    padding: 5px 10px;
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }

  .confirm-btn-no {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-secondary);
    border: none;
    border-radius: 6px;
    padding: 5px 8px;
    font-size: 10px;
    cursor: pointer;
  }

  /* Guide Box */
  .guide-box {
    background: rgba(142, 68, 173, 0.08);
    border: 1px solid rgba(168, 85, 247, 0.2);
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
  }

  .guide-step {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .guide-step strong {
    color: #f3e8ff;
  }

  .step-num {
    background: rgba(168, 85, 247, 0.3);
    color: #e9d5ff;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    font-weight: 800;
    flex-shrink: 0;
    margin-top: 1px;
  }

  /* File Picker Grid */
  .file-picker-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 14px;
  }

  .file-card {
    position: relative;
    background: rgba(15, 10, 28, 0.6);
    border: 1.5px dashed rgba(168, 85, 247, 0.3);
    border-radius: 10px;
    transition: all 0.16s ease;
  }

  .file-card:hover {
    border-color: rgba(168, 85, 247, 0.6);
    background: rgba(24, 16, 46, 0.7);
  }

  .file-card.loaded {
    border-style: solid;
    border-color: rgba(16, 185, 129, 0.4);
    background: rgba(16, 185, 129, 0.06);
  }

  .file-card-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    cursor: pointer;
    min-height: 48px;
    touch-action: manipulation;
  }

  .file-card-left {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .file-icon-box {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.05);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 15px;
    flex-shrink: 0;
  }

  .icon-ok {
    color: #34d399;
    font-weight: 800;
    font-size: 16px;
  }

  .file-card-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .file-slot-name {
    font-size: 9.5px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
  }

  .file-placeholder {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .file-loaded-name {
    font-size: 12px;
    font-weight: 600;
    color: #34d399;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hidden-file-input {
    display: none;
  }

  .clear-file-btn {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-secondary);
    border: none;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 11px;
  }

  /* Buttons */
  .primary-btn {
    width: 100%;
    min-height: 46px;
    padding: 12px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 10px;
    cursor: pointer;
    transition: all 140ms ease;
    background: linear-gradient(135deg, #a855f7 0%, #ec4899 100%);
    border: none;
    color: #ffffff;
    box-shadow: 0 4px 14px rgba(168, 85, 247, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    touch-action: manipulation;
  }

  .primary-btn:active:not(:disabled) {
    transform: scale(0.98);
  }

  .primary-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
    box-shadow: none;
  }

  .spinner-inline {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .status-msg {
    margin-top: 10px;
    font-size: 11px;
    color: var(--text-secondary);
    padding: 8px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
  }

  .status-msg.success {
    color: #34d399;
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.25);
  }

  .status-msg.error {
    color: #fca5a5;
    background: rgba(244, 63, 94, 0.1);
    border: 1px solid rgba(244, 63, 94, 0.25);
  }

  /* Accordion Section */
  .accordion-section {
    margin-top: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    padding-top: 10px;
  }

  .accordion-toggle-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    font-size: 10px;
    font-weight: 700;
    padding: 6px 0;
    cursor: pointer;
    letter-spacing: 0.04em;
    touch-action: manipulation;
  }

  .accordion-toggle-btn:hover {
    color: var(--text-primary);
  }

  .accordion-arrow {
    font-size: 9px;
  }

  .accordion-body {
    padding-top: 10px;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 10px;
  }

  .input-label {
    font-size: 9px;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
  }

  .text-input {
    background: rgba(12, 8, 22, 0.8);
    border: 1px solid rgba(168, 85, 247, 0.25);
    border-radius: 8px;
    padding: 10px 12px;
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
    min-height: 42px;
  }

  .text-input:focus {
    border-color: #a855f7;
    box-shadow: 0 0 0 2px rgba(168, 85, 247, 0.2);
  }

  .save-creds-btn {
    width: 100%;
    min-height: 40px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: var(--text-primary);
    border-radius: 8px;
    font-size: 10.5px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.14s ease;
  }

  .save-creds-btn:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  /* Setting Toggle */
  .setting-toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 0 10px 0;
  }

  .toggle-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    max-width: 80%;
  }

  .setting-name {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .setting-desc {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .custom-switch-btn {
    position: relative;
    width: 44px;
    height: 24px;
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 12px;
    cursor: pointer;
    padding: 0;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    flex-shrink: 0;
    touch-action: manipulation;
  }

  .custom-switch-btn.active {
    background: #10b981;
    border-color: #34d399;
  }

  .switch-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    background: #ffffff;
    border-radius: 50%;
    transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.35);
  }

  .custom-switch-btn.active .switch-knob {
    transform: translateX(20px);
  }

  .keystore-spec {
    font-size: 9.5px;
    color: var(--text-tertiary);
    padding: 10px 12px;
    background: rgba(12, 8, 22, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .keystore-row {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .key-domain {
    color: #34d399;
    font-weight: 700;
  }

  .key-desc {
    color: var(--text-secondary);
  }



  /* Danger Card */
  .danger-card {
    border-color: rgba(244, 63, 94, 0.3);
    background: rgba(28, 12, 22, 0.65);
  }

  .danger-btn {
    width: 100%;
    min-height: 44px;
    padding: 10px;
    font-size: 11.5px;
    font-weight: 700;
    border-radius: 9px;
    cursor: pointer;
    background: rgba(244, 63, 94, 0.15);
    border: 1px solid rgba(244, 63, 94, 0.4);
    color: #fca5a5;
    transition: all 0.14s ease;
    touch-action: manipulation;
  }

  .danger-btn:hover {
    background: rgba(244, 63, 94, 0.25);
    border-color: #f43f5e;
    color: #ffffff;
  }

  .confirm-box {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: rgba(244, 63, 94, 0.1);
    border: 1px solid rgba(244, 63, 94, 0.3);
    border-radius: 8px;
  }

  .confirm-text {
    font-size: 11.5px;
    color: #fecdd3;
    line-height: 1.4;
  }

  .confirm-btns {
    display: flex;
    gap: 8px;
  }

  .danger-confirm {
    flex: 1;
    min-height: 40px;
    padding: 8px;
    background: #e11d48;
    color: #fff;
    border: none;
    border-radius: 7px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 800;
  }

  .cancel-btn {
    flex: 1;
    min-height: 40px;
    padding: 8px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: var(--text-secondary);
    border-radius: 7px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
  }

  .text-neg {
    color: #fca5a5;
  }

  .logout-card {
    border-color: rgba(244, 63, 94, 0.22);
    background: rgba(26, 12, 30, 0.7);
  }

  .logout-card-btn {
    width: 100%;
    min-height: 44px;
    padding: 10px;
    font-size: 11.5px;
    font-weight: 700;
    border-radius: 9px;
    cursor: pointer;
    background: rgba(244, 63, 94, 0.12);
    border: 1px solid rgba(244, 63, 94, 0.35);
    color: #fca5a5;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    touch-action: manipulation;
    transition: all 0.15s ease;
  }

  .logout-card-btn:active {
    transform: scale(0.98);
  }

  .logout-card-btn:hover {
    background: rgba(244, 63, 94, 0.24);
    border-color: #f43f5e;
    color: #ffffff;
  }

  .bio-feedback-msg {
    margin-top: 8px;
    font-size: 10.5px;
    color: #34d399;
    padding: 6px 10px;
    background: rgba(16, 185, 129, 0.08);
    border: 1px solid rgba(16, 185, 129, 0.22);
    border-radius: 6px;
  }

  /* About Developer & Architecture Card */
  .about-card {
    background: linear-gradient(135deg, rgba(26, 17, 48, 0.78) 0%, rgba(18, 12, 36, 0.88) 100%);
    border: 1px solid rgba(168, 85, 247, 0.28);
    box-shadow: 0 8px 32px rgba(6, 3, 14, 0.45);
    position: relative;
    overflow: hidden;
  }

  .about-card::before {
    content: '';
    position: absolute;
    top: -50px;
    right: -50px;
    width: 140px;
    height: 140px;
    background: radial-gradient(circle, rgba(168, 85, 247, 0.18) 0%, transparent 70%);
    pointer-events: none;
  }

  .about-lead-badge {
    background: linear-gradient(135deg, rgba(245, 158, 11, 0.16) 0%, rgba(168, 85, 247, 0.16) 100%);
    border: 1px solid rgba(245, 158, 11, 0.35);
    color: #fbbf24;
    font-size: 9px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 5px;
    letter-spacing: 0.06em;
  }

  .about-hero {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 6px;
    margin-bottom: 14px;
  }

  .about-avatar-frame {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    position: relative;
    flex-shrink: 0;
    padding: 2px;
    background: linear-gradient(135deg, #a855f7 0%, #e1306c 60%, #fbbf24 100%);
    box-shadow: 0 4px 18px rgba(168, 85, 247, 0.4);
  }

  .about-avatar-img {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    display: block;
    background: #18122b;
    border: 2px solid #140e26;
  }

  .about-avatar-badge {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #0f0a1c;
    border: 1.5px solid #fbbf24;
    color: #fbbf24;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.6);
  }

  .about-meta {
    flex: 1;
    min-width: 0;
  }

  .about-kicker {
    font-size: 9px;
    color: var(--accent-primary);
    letter-spacing: 0.08em;
    font-weight: 600;
  }

  .about-name {
    font-size: 16px;
    font-weight: 700;
    color: #ffffff;
    margin: 2px 0 4px 0;
    letter-spacing: 0.01em;
  }

  .about-bio {
    font-size: 11.5px;
    color: var(--text-secondary);
    line-height: 1.45;
    margin: 0;
  }

  .about-actions-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-top: 4px;
    margin-bottom: 12px;
  }

  .about-action-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 10px;
    text-decoration: none;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(168, 85, 247, 0.22);
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    touch-action: manipulation;
  }

  .about-action-btn:hover {
    background: rgba(168, 85, 247, 0.12);
    border-color: rgba(192, 132, 252, 0.55);
    transform: translateY(-1px);
    box-shadow: 0 4px 14px rgba(168, 85, 247, 0.2);
  }

  .about-action-btn:active {
    transform: translateY(0);
  }

  .about-btn-icon-wrap {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: rgba(168, 85, 247, 0.12);
    border: 1px solid rgba(168, 85, 247, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .portfolio-btn .about-btn-icon-wrap {
    background: rgba(216, 180, 254, 0.1);
    border-color: rgba(216, 180, 254, 0.3);
  }

  .about-btn-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .btn-sub-label {
    font-size: 8.5px;
    color: var(--text-tertiary);
    letter-spacing: 0.05em;
  }

  .btn-main-label {
    font-size: 11.5px;
    color: #ffffff;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .about-btn-arrow {
    font-size: 13px;
    color: var(--text-tertiary);
    transition: transform 0.15s ease, color 0.15s ease;
  }

  .about-action-btn:hover .about-btn-arrow {
    color: #ffffff;
    transform: translateX(2px);
  }

  .about-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid rgba(168, 85, 247, 0.12);
    font-size: 9.5px;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
  }

  .about-spec-badge {
    color: var(--accent-primary);
  }

  @media (max-width: 480px) {
    .about-actions-grid {
      grid-template-columns: 1fr;
    }
    .about-footer {
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
    }
  }
</style>
