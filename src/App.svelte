<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Account, type PersonListItem } from './lib/api';
  import { getAvatarUrl, getFallbackAvatar } from './lib/avatar';
  import Navbar from './lib/components/Navbar.svelte';
  import ProfileSheet from './lib/components/ProfileSheet.svelte';
  import ConnectInstagramModal from './lib/components/ConnectInstagramModal.svelte';
  import AddTargetModal from './lib/components/AddTargetModal.svelte';
  import PulseView from './lib/views/PulseView.svelte';
  import PeopleView from './lib/views/PeopleView.svelte';
  import ChangesView from './lib/views/ChangesView.svelte';
  import MonitorView from './lib/views/MonitorView.svelte';
  import SettingsView from './lib/views/SettingsView.svelte';
  import { globalSync } from './lib/syncState.svelte';
  import { promptBiometricAuth } from './lib/biometrics';
  import { startNativeInstagramLogin, isNativeAuthAvailable, openExternalUrl } from './lib/instagramAuth';
  import Icon from './lib/components/Icon.svelte';

  let activeTab = $state('pulse');
  let accounts = $state<Account[]>([]);
  let activeAccount = $state<Account | null>(null);

  // Modals state
  let isConnectModalOpen = $state(false);
  let isReconnectMode = $state(false);
  let isAddTargetModalOpen = $state(false);

  // Selected person for profile sheet
  let selectedPerson = $state<PersonListItem | null>(null);

  // App Lock
  let isLocked = $state(false);
  let lockError = $state<string | null>(null);

  // Owner accounts derived list
  const ownerAccounts = $derived(accounts.filter((a) => a.account_kind === 'owner'));

  // Active owner account associated with the current active account
  const activeOwnerAccount = $derived.by(() => {
    if (!activeAccount) return null;
    if (activeAccount.account_kind === 'owner') return activeAccount;
    return accounts.find((a) => a.id === activeAccount?.authenticated_by_account_id) || null;
  });

  // Custom Account Picker Dropdown state
  let isAccountDropdownOpen = $state(false);
  const connectedOwner = $derived(accounts.find((a) => a.account_kind === 'owner') || accounts[0]);
  const trackedTargets = $derived(accounts.filter((a) => a.account_kind === 'monitored'));

  async function handleLogout() {
    try {
      await api.deleteAllLocalData();
      accounts = [];
      activeAccount = null;
      isAccountDropdownOpen = false;
    } catch (e) {
      console.error('Failed to log out:', e);
      accounts = [];
      activeAccount = null;
      isAccountDropdownOpen = false;
    }
  }

  function handleSelectAccount(acc: Account) {
    activeAccount = acc;
    isAccountDropdownOpen = false;
  }

  let loginErrorMessage = $state<string | null>(null);

  async function loadAccounts() {
    try {
      accounts = await api.getAccounts();
      if (accounts.length > 0) {
        const owner = accounts.find((a) => a.account_kind === 'owner');
        if (!activeAccount || !accounts.some((a) => a.id === activeAccount?.id)) {
          activeAccount = owner || accounts[0];
        } else {
          // Refresh current active account object
          const currentId = activeAccount?.id;
          const fresh = accounts.find((a) => a.id === currentId);
          if (fresh) activeAccount = fresh;
        }
      } else {
        activeAccount = null;
      }

      const bioSetting = await api.getSetting('biometric_lock');
      if (bioSetting === 'true') {
        isLocked = true;
      }
    } catch (e) {
      console.error('Failed to load accounts:', e);
    }
  }

  onMount(async () => {
    await loadAccounts();
  });

  $effect(() => {
    if (globalSync.syncVersion > 0) {
      loadAccounts();
    }
  });

  let isEnteringDemo = $state(false);

  async function handleEnterDemoMode() {
    isEnteringDemo = true;
    try {
      const demoAccount = await api.enterDemoMode();
      await loadAccounts();
      activeAccount = demoAccount;
      activeTab = 'pulse';
    } catch (e) {
      console.error('Failed to enter demo mode:', e);
    } finally {
      isEnteringDemo = false;
    }
  }

  let isNativeLoggingIn = $state(false);
  let nativeLoginStatus = $state('');

  async function handlePrimaryLoginClick() {
    loginErrorMessage = null;
    if (isNativeAuthAvailable()) {
      isNativeLoggingIn = true;
      nativeLoginStatus = 'Opening official Instagram authentication...';
      const result = await startNativeInstagramLogin((status) => {
        nativeLoginStatus = status;
      });
      isNativeLoggingIn = false;
      if (result.success && result.account) {
        handleAccountConnected(result.account);
      } else if (result.error) {
        if (!result.error.toLowerCase().includes('cancel') && !result.error.toLowerCase().includes('dismiss')) {
          loginErrorMessage = result.error;
        }
      }
    } else {
      isReconnectMode = false;
      isConnectModalOpen = true;
    }
  }

  function handleAccountConnected(account: Account) {
    import('./lib/api').then(({ setDemoMode }) => setDemoMode(false));
    isConnectModalOpen = false;
    isReconnectMode = false;
    loginErrorMessage = null;
    loadAccounts();
    activeAccount = account;
    activeTab = 'pulse';
  }

  let peopleInitialFilter = $state<string>('all');

  function handleNavigateToPeople(filter = 'all') {
    peopleInitialFilter = filter;
    activeTab = 'people';
  }

  function handleTargetAdded(targetAccount: Account) {
    loadAccounts();
    activeAccount = targetAccount;
    activeTab = 'pulse';
  }

  async function handleSelectPerson(personId: string, username: string) {
    if (!activeAccount) return;
    try {
      const items = await api.getPeople(activeAccount.id, 'all', username, 1, 0);
      if (items.length > 0) {
        selectedPerson = items[0];
      } else {
        selectedPerson = {
          id: personId,
          username,
          display_name: username,
          is_verified: false,
          is_private: false,
          is_follower: true,
          is_following: false,
          is_mutual: false,
          last_seen_at: Date.now() / 1000,
          tags: [],
          has_note: false,
        };
      }
    } catch {
      selectedPerson = {
        id: personId,
        username,
        display_name: username,
        is_verified: false,
        is_private: false,
        is_follower: true,
        is_following: false,
        is_mutual: false,
        last_seen_at: Date.now() / 1000,
        tags: [],
        has_note: false,
      };
    }
  }

  async function checkBiometricLockOnResume() {
    try {
      const bio = await api.getSetting('biometric_lock');
      if (bio === 'true') {
        isLocked = true;
        await unlockApp();
      }
    } catch (e) {
      console.error('Failed to check biometric lock:', e);
    }
  }

  async function unlockApp() {
    lockError = null;
    const res = await promptBiometricAuth(
      'Stalkr Privacy Lock',
      'Confirm fingerprint, face, or device PIN to continue'
    );
    if (res.success) {
      isLocked = false;
      lockError = null;
    } else {
      lockError = res.error || 'Authentication required';
    }
  }

  onMount(() => {
    loadAccounts();
    checkBiometricLockOnResume();

    const handleVisibility = () => {
      if (document.visibilityState === 'visible') {
        checkBiometricLockOnResume();
      }
    };
    document.addEventListener('visibilitychange', handleVisibility);
    return () => {
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  });
</script>

<main class="app-root">
  {#if isLocked}
    <div class="lock-overlay">
      <div class="lock-card">
        <div class="lock-icon">
          <Icon name="shield" size={44} color="#a855f7" strokeWidth={2} />
        </div>
        <div class="lock-title">Stalkr Privacy Lock</div>
        <div class="lock-desc">Biometric authentication or device passcode required</div>
        {#if lockError}
          <div class="lock-err-pill mono">{lockError}</div>
        {/if}
        <button class="unlock-btn" onclick={unlockApp} type="button">
          <Icon name="fingerprint" size={18} color="#ffffff" strokeWidth={2.4} />
          <span>Authenticate with Biometrics</span>
        </button>
      </div>
    </div>
  {:else if activeAccount}
    <!-- Top Identity & Multi-Account Navigation Header -->
    <header class="top-nav-bar">
      <div class="top-nav-left">
        <!-- Account Avatar Bubble (Clean circle, no fake story ring) -->
        <div class="account-avatar-bubble">
          <img
            src={getAvatarUrl(activeAccount.username, activeAccount.avatar_url)}
            alt={activeAccount.username}
            class="avatar-bubble-img"
            loading="lazy"
            referrerpolicy="no-referrer"
            onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(activeAccount?.username || 'user'); }}
          />
          {#if activeAccount.account_kind === 'owner'}
            <span class="owner-star" title="Connected Profile">★</span>
          {/if}
        </div>

        <div class="account-details">
          <div class="account-picker-wrap">
            <button
              class="account-picker-btn"
              onclick={() => (isAccountDropdownOpen = !isAccountDropdownOpen)}
              type="button"
              aria-expanded={isAccountDropdownOpen}
            >
              <span class="account-handle">@{activeAccount.username}</span>
              <span class="dropdown-chevron-box {isAccountDropdownOpen ? 'open' : ''}">
                <Icon name="chevron-down" size={13} strokeWidth={2.4} />
              </span>
            </button>

            {#if isAccountDropdownOpen}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="dropdown-backdrop" onclick={() => (isAccountDropdownOpen = false)}></div>

              <div class="custom-dropdown-panel slide-down">
                <!-- Connected Profile Section -->
                <div class="dropdown-category-title mono">YOUR CONNECTED PROFILE</div>
                {#if connectedOwner}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <div
                    class="dropdown-account-row {activeAccount.id === connectedOwner.id ? 'selected' : ''}"
                    onclick={() => handleSelectAccount(connectedOwner)}
                    role="button"
                    tabindex="0"
                  >
                    <div class="row-avatar">
                      <img
                        src={getAvatarUrl(connectedOwner.username, connectedOwner.avatar_url)}
                        alt={connectedOwner.username}
                        class="row-avatar-img"
                        loading="lazy"
                        referrerpolicy="no-referrer"
                        onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(connectedOwner.username); }}
                      />
                    </div>
                    <div class="row-info">
                      <div class="row-handle">
                        <span>@{connectedOwner.username}</span>
                        <span class="connected-tag">You</span>
                      </div>
                      <span class="row-desc mono">Root Instagram Account</span>
                    </div>
                    {#if activeAccount.id === connectedOwner.id}
                      <span class="row-check">✓</span>
                    {/if}
                  </div>
                {/if}

                <!-- Tracked Targets Section -->
                <div class="dropdown-category-title mono">
                  <span>TRACKED TARGETS</span>
                  <span class="target-count-badge mono">{trackedTargets.length}</span>
                </div>

                {#if trackedTargets.length === 0}
                  <div class="no-targets-box mono">
                    No external targets tracked yet.
                  </div>
                {:else}
                  <div class="targets-scroll-list">
                    {#each trackedTargets as target}
                      <!-- svelte-ignore a11y_click_events_have_key_events -->
                      <div
                        class="dropdown-account-row {activeAccount.id === target.id ? 'selected' : ''}"
                        onclick={() => handleSelectAccount(target)}
                        role="button"
                        tabindex="0"
                      >
                        <div class="row-avatar target">
                          <img
                            src={getAvatarUrl(target.username, target.avatar_url)}
                            alt={target.username}
                            class="row-avatar-img"
                            loading="lazy"
                            referrerpolicy="no-referrer"
                            onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(target.username); }}
                          />
                        </div>
                        <div class="row-info">
                          <div class="row-handle">
                            <span>@{target.username}</span>
                            {#if target.is_private}
                              <span class="target-lock" title="Private Target">🔒</span>
                            {/if}
                          </div>
                          <span class="row-desc mono">
                            {target.followers_count ? `${target.followers_count.toLocaleString()} followers` : 'Tracked Profile'}
                          </span>
                        </div>
                        {#if activeAccount.id === target.id}
                          <span class="row-check">✓</span>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {/if}

                <div class="dropdown-action-footer">
                  <button
                    class="add-target-menu-btn"
                    onclick={() => {
                      isAccountDropdownOpen = false;
                      isAddTargetModalOpen = true;
                    }}
                  >
                    <span class="plus-icon">+</span>
                    <span>Track New Target</span>
                  </button>
                </div>
              </div>
            {/if}
          </div>

          <div class="role-row">
            {#if activeAccount.account_kind === 'owner'}
              <span class="role-pill owner">Connected Profile</span>
            {:else}
              <span class="role-pill target">
                Tracked Target {activeOwnerAccount ? `(via @${activeOwnerAccount.username})` : ''}
              </span>
            {/if}
          </div>
        </div>
      </div>

      <div class="top-nav-right">
        <!-- Luxury 2-Line Developer Badge (Beside Logout Button on Right) -->
        <a
          href="https://meetmistry.vercel.app"
          target="_blank"
          rel="noopener noreferrer"
          class="nav-developer-badge"
          title="Developed by Meet Mistry (https://meetmistry.vercel.app)"
          onclick={(e) => {
            e.preventDefault();
            openExternalUrl('https://meetmistry.vercel.app');
          }}
        >
          <div class="badge-sheen-sweep"></div>
          <div class="nav-badge-medallion">
            <span class="medallion-gem">✦</span>
          </div>
          <div class="nav-badge-text">
            <span class="nav-badge-line1 mono">DEVELOPED BY</span>
            <span class="nav-badge-line2">MEET MISTRY</span>
          </div>
          <span class="nav-badge-arrow">›</span>
        </a>

        {#if activeAccount.access_state === 'auth_required'}
          <button
            class="health-badge expired"
            onclick={() => { isReconnectMode = true; isConnectModalOpen = true; }}
          >
            <span class="pulse-warn"></span>
            Re-Auth
          </button>
        {/if}

        <!-- Logout as Icon-Only Button -->
        <button
          class="nav-icon-btn logout-btn"
          title="Log out and return to fresh start screen"
          aria-label="Logout"
          onclick={handleLogout}
        >
          <Icon name="logout" size={15} strokeWidth={2.4} />
        </button>
      </div>
    </header>

    <!-- Active view container -->
    <div class="content-container">
      {#if activeTab === 'pulse'}
        <PulseView
          account={activeAccount}
          ownerAccount={activeOwnerAccount}
          onSelectPerson={handleSelectPerson}
          onOpenConnect={() => { isReconnectMode = true; isConnectModalOpen = true; }}
          onNavigateToPeople={handleNavigateToPeople}
        />
      {:else if activeTab === 'people'}
        <PeopleView
          account={activeAccount}
          ownerAccount={activeOwnerAccount}
          onSelectPerson={handleSelectPerson}
          initialFilter={peopleInitialFilter}
        />
      {:else if activeTab === 'changes'}
        <ChangesView account={activeAccount} onSelectPerson={handleSelectPerson} />
      {:else if activeTab === 'monitor'}
        <MonitorView
          account={activeAccount}
          ownerAccount={activeOwnerAccount}
        />
      {:else if activeTab === 'settings'}
        <SettingsView
          account={activeAccount}
          {accounts}
          onSwitchAccount={(acc: Account) => (activeAccount = acc)}
          onRefreshAccounts={loadAccounts}
          onLogout={handleLogout}
          onAddTarget={() => (isAddTargetModalOpen = true)}
        />
      {/if}
    </div>

    <!-- Profile Slide-Over Sheet -->
    {#if selectedPerson}
      <ProfileSheet
        person={selectedPerson}
        accountId={activeAccount.id}
        onClose={() => (selectedPerson = null)}
      />
    {/if}

    <!-- Bottom Navigation Bar -->
    <Navbar {activeTab} onChangeTab={(tab) => { activeTab = tab; if (tab !== 'people') peopleInitialFilter = 'all'; }} />
  {:else}
    <!-- First-Time Onboarding Screen — Consumer Welcome Experience -->
    <div class="onboarding-container fade-in">
      <div class="onboarding-glow"></div>
      <div class="onboarding-card">
        <div class="logo-mark">
          <img src="/icon.png" alt="Stalkr" class="logo-img" />
        </div>

        <h1 class="brand-title">Stalkr</h1>
        <p class="brand-tagline">Your Private Circle, Unfiltered.</p>
        <p class="brand-sub">
          Monitor follower changes, track mutual connections, and inspect relationship dynamics privately. Zero cloud servers, 100% on-device Keystore encryption.
        </p>

        <div class="action-stack">
          <!-- Primary Instagram Connect Action -->
          <button
            class="primary-connect-btn"
            onclick={handlePrimaryLoginClick}
            disabled={isNativeLoggingIn}
          >
            {#if isNativeLoggingIn}
              <span class="native-login-spinner"></span>
              <span>{nativeLoginStatus || 'Connecting Instagram...'}</span>
            {:else}
              <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
                <path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zm0-2.163c-3.259 0-3.667.014-4.947.072-4.358.2-6.78 2.618-6.98 6.98-.059 1.281-.073 1.689-.073 4.948 0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98 1.281.058 1.689.072 4.948.072 3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98-1.281-.059-1.69-.073-4.949-.073zm0 5.838c-3.403 0-6.162 2.759-6.162 6.162s2.759 6.163 6.162 6.163 6.162-2.759 6.162-6.163c0-3.403-2.759-6.162-6.162-6.162zm0 10.162c-2.209 0-4-1.79-4-4 0-2.209 1.791-4 4-4s4 1.791 4 4c0 2.21-1.791 4-4 4zm6.406-11.845c-.796 0-1.441.645-1.441 1.44s.645 1.44 1.441 1.44c.795 0 1.439-.645 1.439-1.44s-.644-1.44-1.439-1.44z"/>
              </svg>
              <span>Log in with Instagram</span>
              <span class="btn-arrow">→</span>
            {/if}
          </button>

          {#if loginErrorMessage}
            <div class="login-err-banner">
              <span>{loginErrorMessage}</span>
            </div>
          {/if}

          <div class="divider-row">
            <span>OR PREVIEW DEMO</span>
          </div>

          <!-- Enter Demo Mode Action -->
          <button
            class="demo-mode-btn"
            onclick={handleEnterDemoMode}
            disabled={isEnteringDemo}
          >
            <div class="demo-btn-left">
              <span class="demo-sparkle">✨</span>
              <div class="demo-btn-text">
                <span class="demo-btn-title">Enter Demo Mode</span>
                <span class="demo-btn-sub">Realistic simulated profile (@meetzarc)</span>
              </div>
            </div>
            {#if isEnteringDemo}
              <span class="demo-spinner"></span>
            {:else}
              <span class="demo-arrow">→</span>
            {/if}
          </button>
        </div>

        <div class="guarantee-box">
          <div class="guar-item">
            <span class="guar-icon">🔒</span>
            <span>Hardware Keystore AES-256 Encryption</span>
          </div>
          <div class="guar-item">
            <span class="guar-icon">🛡️</span>
            <span>Zero Third-Party Cloud Uploads</span>
          </div>
          <div class="guar-item">
            <span class="guar-icon">✨</span>
            <span>100% Ban-Safe Official Web View</span>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Connect Instagram Modal -->
  <ConnectInstagramModal
    isOpen={isConnectModalOpen}
    isReconnect={isReconnectMode}
    reconnectAccountId={activeAccount?.id || ''}
    onClose={() => (isConnectModalOpen = false)}
    onSuccess={handleAccountConnected}
  />

  <!-- Add Target Modal -->
  <AddTargetModal
    isOpen={isAddTargetModalOpen}
    {ownerAccounts}
    activeOwnerId={activeOwnerAccount?.id || ''}
    onClose={() => (isAddTargetModalOpen = false)}
    onSuccess={handleTargetAdded}
  />
</main>

<style>
  .app-root {
    min-height: 100vh;
    background-color: var(--bg-root);
    color: var(--text-primary);
    position: relative;
  }

  /* Fixed Top Navigation Bar */
  .top-nav-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: max(8px, env(safe-area-inset-top, 0px)) max(12px, env(safe-area-inset-right, 0px)) 8px max(12px, env(safe-area-inset-left, 0px));
    background: rgba(18, 13, 34, 0.96);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-bottom: 1px solid var(--border-subtle);
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    width: 100%;
    z-index: 100;
    box-shadow: 0 4px 20px rgba(6, 3, 14, 0.5);
  }

  .top-nav-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .account-avatar-bubble {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--ig-gradient);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ffffff;
    font-weight: 700;
    font-size: 13px;
    position: relative;
    flex-shrink: 0;
  }

  .avatar-bubble-img {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
  }

  .row-avatar-img {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
  }

  .owner-star {
    position: absolute;
    bottom: -2px;
    right: -2px;
    width: 14px;
    height: 14px;
    background: var(--accent-signal);
    color: #000;
    border-radius: 50%;
    font-size: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--bg-surface);
  }

  .account-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .account-picker-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .account-picker-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px 0;
    text-align: left;
    outline: none;
  }

  .account-handle {
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .dropdown-chevron-box {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
    transition: transform 200ms cubic-bezier(0.16, 1, 0.3, 1), color 160ms ease;
  }

  .dropdown-chevron-box.open {
    transform: rotate(180deg);
    color: var(--accent-primary);
  }

  .dropdown-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
  }

  .custom-dropdown-panel {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    width: 260px;
    background: rgba(20, 14, 38, 0.96);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    padding: 8px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.65), 0 0 0 1px rgba(168, 85, 247, 0.15);
    z-index: 110;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .slide-down {
    animation: slideDownMenu 180ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
    transform-origin: top left;
  }

  @keyframes slideDownMenu {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .dropdown-category-title {
    font-size: 9px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.08em;
    padding: 6px 8px 2px 8px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .target-count-badge {
    background: rgba(168, 85, 247, 0.15);
    color: var(--accent-primary);
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 4px;
  }

  .dropdown-account-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    cursor: pointer;
    transition: background-color 140ms ease;
    user-select: none;
  }

  .dropdown-account-row:hover {
    background: var(--bg-surface-elevated);
  }

  .dropdown-account-row.selected {
    background: rgba(168, 85, 247, 0.12);
  }

  .row-avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--ig-gradient);
    color: #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .row-avatar.target {
    background: #2a2048;
    color: var(--accent-primary);
    border: 1px solid rgba(168, 85, 247, 0.3);
  }

  .row-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .row-handle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .connected-tag {
    font-size: 8px;
    font-weight: 700;
    text-transform: uppercase;
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    padding: 1px 4px;
    border-radius: 4px;
    letter-spacing: 0.04em;
  }

  .target-lock {
    font-size: 10px;
  }

  .row-desc {
    font-size: 10px;
    color: var(--text-tertiary);
  }

  .row-check {
    color: var(--accent-primary);
    font-size: 12px;
    font-weight: 800;
  }

  .no-targets-box {
    padding: 10px;
    font-size: 11px;
    color: var(--text-tertiary);
    text-align: center;
  }

  .targets-scroll-list {
    max-height: 180px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .dropdown-action-footer {
    border-top: 1px solid var(--border-subtle);
    padding-top: 6px;
    margin-top: 4px;
  }

  .add-target-menu-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px;
    background: rgba(168, 85, 247, 0.1);
    border: 1px dashed rgba(168, 85, 247, 0.35);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 140ms ease;
  }

  .add-target-menu-btn:hover {
    background: rgba(168, 85, 247, 0.2);
    border-color: var(--accent-primary);
    color: #ffffff;
  }

  .plus-icon {
    font-size: 14px;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .role-row {
    display: flex;
    align-items: center;
  }

  .role-pill {
    font-size: 10px;
    font-weight: 500;
    padding: 1px 6px;
    border-radius: 6px;
  }

  .role-pill.owner {
    background: rgba(245, 158, 11, 0.12);
    color: var(--accent-signal);
  }

  .role-pill.target {
    background: rgba(99, 102, 241, 0.12);
    color: var(--accent-indigo);
  }

  /* Ultra-Luxury Gradient Developer Badge */
  .nav-developer-badge {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 3px 9px 3px 4px;
    border-radius: 9999px;
    border: 1px solid transparent;
    background-image:
      linear-gradient(135deg, rgba(28, 14, 48, 0.94), rgba(14, 8, 28, 0.98)),
      linear-gradient(135deg, #fbbf24 0%, #d946ef 50%, #8b5cf6 100%);
    background-origin: border-box;
    background-clip: padding-box, border-box;
    box-shadow:
      0 0 14px rgba(217, 70, 239, 0.22),
      0 2px 10px rgba(0, 0, 0, 0.5),
      inset 0 1px 1px rgba(255, 255, 255, 0.18);
    text-decoration: none;
    transition: all 0.22s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
    user-select: none;
    touch-action: manipulation;
    flex-shrink: 0;
    overflow: hidden;
  }

  .nav-developer-badge:hover {
    transform: translateY(-1px) scale(1.02);
    box-shadow:
      0 0 22px rgba(217, 70, 239, 0.4),
      0 4px 16px rgba(0, 0, 0, 0.6),
      inset 0 1px 2px rgba(255, 255, 255, 0.28);
  }

  .nav-developer-badge:active {
    transform: scale(0.98);
  }

  .badge-sheen-sweep {
    position: absolute;
    top: 0;
    left: -100%;
    width: 60%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.22), transparent);
    transform: skewX(-20deg);
    animation: badgeSheen 4.5s cubic-bezier(0.16, 1, 0.3, 1) infinite;
    pointer-events: none;
  }

  @keyframes badgeSheen {
    0%, 70% { left: -100%; opacity: 0; }
    80% { opacity: 1; }
    100% { left: 200%; opacity: 0; }
  }

  .nav-badge-medallion {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: linear-gradient(135deg, #f59e0b 0%, #ec4899 50%, #8b5cf6 100%);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 6px rgba(245, 158, 11, 0.35);
    flex-shrink: 0;
  }

  .medallion-gem {
    font-size: 10.5px;
    color: #ffffff;
    filter: drop-shadow(0 0 3px rgba(255, 255, 255, 0.85));
    transition: transform 0.25s ease;
  }

  .nav-developer-badge:hover .medallion-gem {
    transform: rotate(90deg) scale(1.15);
  }

  .nav-badge-text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    line-height: 1.1;
  }

  .nav-badge-line1 {
    font-size: 6.8px;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: #fbbf24;
    text-shadow: 0 0 8px rgba(251, 191, 36, 0.4);
  }

  .nav-badge-line2 {
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.04em;
    background: linear-gradient(90deg, #ffffff 0%, #fed7aa 25%, #f472b6 65%, #c084fc 100%);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    filter: drop-shadow(0 0 6px rgba(244, 114, 182, 0.35));
  }

  .nav-badge-arrow {
    font-size: 12px;
    font-weight: 700;
    color: rgba(216, 180, 254, 0.65);
    margin-left: -2px;
    transition: transform 0.2s ease, color 0.2s ease;
  }

  .nav-developer-badge:hover .nav-badge-arrow {
    color: #ffffff;
    transform: translateX(2px);
  }

  @media (max-width: 480px) {
    .top-nav-bar {
      padding: max(8px, env(safe-area-inset-top, 0px)) 10px 8px 10px;
    }
    .top-nav-left {
      gap: 7px;
    }
    .account-picker-btn {
      max-width: 95px;
    }
    .nav-developer-badge {
      padding: 2px 7px 2px 3.5px;
      gap: 5px;
    }
    .nav-badge-medallion {
      width: 17px;
      height: 17px;
    }
    .medallion-gem {
      font-size: 9px;
    }
    .nav-badge-line1 {
      font-size: 5.8px;
      letter-spacing: 0.08em;
    }
    .nav-badge-line2 {
      font-size: 8.8px;
    }
    .nav-badge-arrow {
      font-size: 10px;
    }
    .logout-btn {
      width: 28px;
      height: 28px;
    }
  }

  @media (max-width: 430px) {
    .account-picker-btn {
      max-width: 100px;
    }
    .account-handle {
      max-width: 80px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      display: inline-block;
    }
    .logout-btn {
      width: 26px;
      height: 26px;
    }
    .top-nav-right {
      gap: 6px;
    }
    .top-nav-left {
      gap: 6px;
    }
    .role-row {
      display: none;
    }
  }

  @media (max-width: 360px) {
    .nav-developer-badge {
      padding: 2px 5px 2px 3px;
      gap: 4px;
    }
    .nav-badge-line2 {
      font-size: 8px;
    }
    .nav-badge-arrow {
      display: none;
    }
    .top-nav-bar {
      padding: max(6px, env(safe-area-inset-top, 0px)) 6px 6px 6px;
    }
  }

  .top-nav-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .health-badge {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 600;
    padding: 4px 8px;
    border-radius: 8px;
  }

  .health-badge.expired {
    background: rgba(244, 63, 94, 0.15);
    color: var(--accent-negative);
    border: 1px solid rgba(244, 63, 94, 0.3);
    cursor: pointer;
  }

  .pulse-warn {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-negative);
  }

  .nav-icon-btn {
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--text-primary);
    width: 32px;
    height: 32px;
    border-radius: 9px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .nav-icon-btn.logout-btn {
    background: rgba(244, 63, 94, 0.08);
    border: 1px solid rgba(244, 63, 94, 0.25);
    color: #fca5a5;
  }

  .nav-icon-btn.logout-btn:hover {
    background: rgba(244, 63, 94, 0.22);
    border-color: rgba(244, 63, 94, 0.55);
    color: #ffffff;
    transform: translateY(-1px);
    box-shadow: 0 2px 10px rgba(244, 63, 94, 0.25);
  }

  .content-container {
    position: fixed;
    top: calc(54px + max(8px, env(safe-area-inset-top, 0px)));
    bottom: 0;
    left: 0;
    right: 0;
    width: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    -webkit-overflow-scrolling: touch;
    padding-bottom: calc(72px + max(12px, env(safe-area-inset-bottom, 0px)));
    box-sizing: border-box;
  }

  @media (max-width: 560px) {
    .content-container {
      top: calc(50px + max(8px, env(safe-area-inset-top, 0px)));
    }

    .top-nav-bar {
      padding: max(8px, env(safe-area-inset-top, 0px)) 10px 8px 10px;
    }

    .top-nav-left {
      gap: 8px;
      min-width: 0;
      flex: 1;
    }

    .account-handle {
      max-width: 115px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 13px;
    }

    .custom-dropdown-panel {
      width: min(280px, calc(100vw - 20px));
      left: 0;
    }

    .role-row .role-pill {
      font-size: 8px;
      padding: 1px 4px;
      max-width: 120px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .top-nav-right {
      gap: 6px;
      flex-shrink: 0;
    }
  }

  /* Privacy Lock Screen */
  .lock-overlay {
    position: fixed;
    inset: 0;
    background-color: var(--bg-root);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 999;
  }

  .lock-card {
    background-color: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: 20px;
    padding: 36px 28px;
    text-align: center;
    max-width: 320px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    box-shadow: var(--card-shadow);
  }

  .lock-icon {
    font-size: 36px;
  }

  .lock-title {
    font-size: 16px;
    font-weight: 700;
  }

  .lock-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .unlock-btn {
    background: var(--ig-gradient);
    color: #ffffff;
    font-weight: 600;
    font-size: 12px;
    padding: 12px;
    border-radius: 10px;
    border: none;
    cursor: pointer;
    touch-action: manipulation;
  }

  .lock-err-pill {
    font-size: 10.5px;
    color: #fca5a5;
    background: rgba(244, 63, 94, 0.12);
    border: 1px solid rgba(244, 63, 94, 0.3);
    border-radius: 6px;
    padding: 6px 10px;
    line-height: 1.35;
  }

  /* Consumer Onboarding Welcome Screen — Strictly Non-Scrollable & Perfectly Fitted */
  .onboarding-container {
    height: 100vh;
    height: 100dvh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: max(16px, env(safe-area-inset-top, 16px)) 16px max(16px, env(safe-area-inset-bottom, 16px)) 16px;
    position: relative;
    overflow: hidden;
    box-sizing: border-box;
  }

  .onboarding-glow {
    position: absolute;
    top: 25%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 320px;
    height: 320px;
    background: radial-gradient(circle, rgba(168, 85, 247, 0.3) 0%, rgba(124, 58, 237, 0.15) 45%, transparent 72%);
    pointer-events: none;
    filter: blur(44px);
  }

  .onboarding-card {
    background-color: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: 24px;
    padding: 28px 22px;
    max-width: 400px;
    width: 100%;
    max-height: 94vh;
    max-height: 94dvh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
    box-shadow: 0 16px 50px rgba(0, 0, 0, 0.65);
    position: relative;
    z-index: 10;
    box-sizing: border-box;
  }

  .logo-mark {
    width: 58px;
    height: 58px;
    margin: 0 auto 10px auto;
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5), 0 0 18px rgba(168, 85, 247, 0.25);
    overflow: hidden;
    flex-shrink: 0;
  }

  .logo-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 16px;
  }

  .brand-title {
    font-size: 26px;
    font-weight: 800;
    letter-spacing: -0.03em;
    color: var(--text-primary);
    margin-bottom: 2px;
    line-height: 1.1;
  }

  .brand-tagline {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--accent-primary);
    margin-bottom: 8px;
  }

  .brand-sub {
    font-size: 11.5px;
    color: var(--text-secondary);
    line-height: 1.4;
    margin-bottom: 18px;
  }

  .action-stack {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 16px;
  }

  .primary-connect-btn {
    background: var(--ig-story-gradient);
    color: #ffffff;
    font-weight: 700;
    font-size: 13.5px;
    padding: 12px 18px;
    border-radius: 11px;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
    transition: transform 0.15s, opacity 0.15s;
  }

  .primary-connect-btn:hover {
    transform: translateY(-1px);
    opacity: 0.95;
  }

  .native-login-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: nativeSpin 0.75s linear infinite;
  }

  @keyframes nativeSpin {
    to { transform: rotate(360deg); }
  }

  .btn-arrow {
    font-size: 16px;
    transition: transform 0.15s;
  }

  .primary-connect-btn:hover .btn-arrow {
    transform: translateX(3px);
  }

  .login-err-banner {
    padding: 10px 14px;
    border-radius: 12px;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: #fca5a5;
    font-size: 12px;
    text-align: center;
    line-height: 1.4;
  }

  .divider-row {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--text-tertiary);
    padding: 2px 0;
  }

  .demo-mode-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    padding: 12px 16px;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    text-align: left;
    width: 100%;
  }

  .demo-mode-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.2);
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }

  .demo-mode-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
  }

  .demo-btn-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .demo-sparkle {
    font-size: 20px;
    line-height: 1;
    filter: drop-shadow(0 0 8px rgba(225, 48, 108, 0.5));
  }

  .demo-btn-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .demo-btn-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.01em;
  }

  .demo-btn-sub {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .demo-arrow {
    font-size: 16px;
    color: var(--text-tertiary);
    transition: transform 0.15s, color 0.15s;
  }

  .demo-mode-btn:hover .demo-arrow {
    transform: translateX(3px);
    color: var(--text-primary);
  }

  .demo-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }


  .guarantee-box {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    font-size: 10px;
    color: var(--text-tertiary);
    text-align: left;
  }

  .guar-item {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .guar-icon {
    font-size: 11px;
  }
</style>
