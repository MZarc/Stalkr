<script lang="ts">
  import { api, type Account } from '../api';
  import { startNativeInstagramLogin, isNativeAuthAvailable } from '../instagramAuth';
  import Icon from './Icon.svelte';

  let {
    isOpen = false,
    isReconnect = false,
    reconnectAccountId = '',
    onClose,
    onSuccess,
  }: {
    isOpen: boolean;
    isReconnect?: boolean;
    reconnectAccountId?: string;
    onClose: () => void;
    onSuccess: (account: Account) => void;
  } = $props();

  let mode = $state<'official' | 'manual' | 'demo'>('official');
  let rawCookieInput = $state('');
  let sessionId = $state('');
  let dsUserId = $state('');
  let csrfToken = $state('');
  let mockUsername = $state('stalkr.editorial');

  let isValidating = $state(false);
  let statusText = $state('');
  let errorMessage = $state<string | null>(null);
  let successAccount = $state<Account | null>(null);

  async function handleNativeLogin() {
    errorMessage = null;
    successAccount = null;

    if (isNativeAuthAvailable()) {
      isValidating = true;
      statusText = 'Opening official Instagram authentication...';

      const result = await startNativeInstagramLogin((status) => {
        statusText = status;
      });

      isValidating = false;
      if (result.success && result.account) {
        successAccount = result.account;
        setTimeout(() => {
          onSuccess(result.account!);
          handleClose();
        }, 1000);
      } else if (result.error) {
        if (!result.error.toLowerCase().includes('cancel')) {
          errorMessage = result.error;
        }
      }
    } else {
      openInstagramWeb();
    }
  }

  // Automatic Cookie String Parser
  function handleRawCookieChange(val: string) {
    rawCookieInput = val;
    if (!val.trim()) return;

    // Detect if full cookie string or key-value pair was pasted
    const sessionMatch = val.match(/sessionid=([^;\s]+)/i);
    const dsUserMatch = val.match(/ds_user_id=([^;\s]+)/i);
    const csrfMatch = val.match(/csrftoken=([^;\s]+)/i);

    if (sessionMatch) sessionId = decodeURIComponent(sessionMatch[1]);
    if (dsUserMatch) dsUserId = decodeURIComponent(dsUserMatch[1]);
    if (csrfMatch) csrfToken = decodeURIComponent(csrfMatch[1]);

    // If bare sessionid string starting with user ID (e.g. 17841400%3A...)
    if (!sessionMatch && val.includes('%3A')) {
      sessionId = val.trim();
      const parts = val.split('%3A');
      if (parts[0] && /^\d+$/.test(parts[0])) {
        dsUserId = parts[0];
      }
    }
  }

  function openInstagramWeb() {
    window.open('https://www.instagram.com/accounts/login/', '_blank');
  }

  async function handleConnectSession() {
    if (!sessionId.trim() || !dsUserId.trim()) {
      errorMessage = 'Both Session ID and User ID (ds_user_id) are required.';
      return;
    }

    isValidating = true;
    errorMessage = null;
    successAccount = null;

    try {
      const account = await api.connectInstagram(
        sessionId.trim(),
        dsUserId.trim(),
        csrfToken.trim() || undefined
      );
      successAccount = account;
      setTimeout(() => {
        onSuccess(account);
        handleClose();
      }, 1000);
    } catch (err: any) {
      errorMessage = typeof err === 'string' ? err : err?.message || 'Authentication failed. Please verify credentials.';
    } finally {
      isValidating = false;
    }
  }

  async function handleConnectMock() {
    if (!mockUsername.trim()) return;
    isValidating = true;
    errorMessage = null;
    try {
      const clean = mockUsername.trim().replace(/^@/, '');
      const account = await api.connectMockOwner(clean, clean, 1842, 936);
      successAccount = account;
      setTimeout(() => {
        onSuccess(account);
        handleClose();
      }, 600);
    } catch (err: any) {
      errorMessage = typeof err === 'string' ? err : 'Failed to create demo account';
    } finally {
      isValidating = false;
    }
  }

  function handleClose() {
    rawCookieInput = '';
    sessionId = '';
    dsUserId = '';
    csrfToken = '';
    errorMessage = null;
    successAccount = null;
    onClose();
  }
</script>

{#if isOpen}
  <div class="modal-backdrop fade-in" onclick={handleClose} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="modal-sheet slide-up"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-labelledby="modal-title"
      tabindex="-1"
    >
      <div class="drag-handle"></div>

      <div class="modal-header">
        <div class="header-titles">
          <div class="badge-row">
            <span class="badge ig-badge">INSTAGRAM AUTHENTICATION</span>
            <span class="badge secure">AES-256 ENCRYPTED</span>
          </div>
          <h2 id="modal-title" class="title">
            {isReconnect ? 'Reconnect Instagram Session' : 'Connect Your Account'}
          </h2>
          <p class="subtitle">
            Private, local-first analytics. Credentials never leave this device.
          </p>
        </div>
        <button class="close-btn" onclick={handleClose} aria-label="Close">✕</button>
      </div>

      <!-- Segmented Mode Navigation -->
      <div class="mode-tabs">
        <button
          class="mode-btn {mode === 'official' ? 'active' : ''}"
          onclick={() => { mode = 'official'; errorMessage = null; }}
        >
          <span>✨ In-App Login</span>
        </button>
        <button
          class="mode-btn {mode === 'manual' ? 'active' : ''}"
          onclick={() => { mode = 'manual'; errorMessage = null; }}
        >
          <span>🔑 Cookie Tokens</span>
        </button>
        <button
          class="mode-btn {mode === 'demo' ? 'active' : ''}"
          onclick={() => { mode = 'demo'; errorMessage = null; }}
        >
          <span>⚡ Demo Mode</span>
        </button>
      </div>

      <!-- OFFICIAL IN-APP / SAFE WEB LOGIN FLOW -->
      {#if mode === 'official'}
        <div class="flow-container fade-in">
          <div class="hero-card">
            <div class="hero-icon-wrap">
              <svg viewBox="0 0 24 24" width="36" height="36" class="ig-svg-icon" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="2" width="20" height="20" rx="5" ry="5"/>
                <path d="M16 11.37A4 4 0 1 1 12.63 8 4 4 0 0 1 16 11.37z"/>
                <line x1="17.5" y1="6.5" x2="17.51" y2="6.5"/>
              </svg>
            </div>
            <div class="hero-content">
              <h3>Official Instagram Web Login</h3>
              <p>Log in securely on Meta's official portal. Eliminates ban risk, supports 2FA, and carries real browser fingerprints.</p>
            </div>
          </div>

          <div class="safe-badges-grid">
            <div class="safe-badge">
              <span class="safe-icon">
                <Icon name="shield" size={18} color="#34d399" strokeWidth={2.4} />
              </span>
              <div>
                <strong>Zero Ban Risk</strong>
                <span>Official Meta webview, no script injection</span>
              </div>
            </div>
            <div class="safe-badge">
              <span class="safe-icon">
                <Icon name="lock" size={18} color="#a855f7" strokeWidth={2.4} />
              </span>
              <div>
                <strong>Local Keystore</strong>
                <span>AES-256 encrypted on this device</span>
              </div>
            </div>
          </div>

          <!-- 1-Tap Official In-App Login Action -->
          <div class="official-cta-card">
            {#if isValidating}
              <div class="auth-loading-state">
                <div class="auth-pulse-spinner"></div>
                <div class="auth-status-title">{statusText || 'Connecting with Instagram...'}</div>
                <p class="auth-status-hint">Please complete login in the Instagram screen. Credentials will be securely captured.</p>
              </div>
            {:else if successAccount}
              <div class="auth-success-state">
                <div class="auth-check-icon">✓</div>
                <div class="auth-status-title">Connected @{successAccount.username}!</div>
                <p class="auth-status-hint">Followers: {successAccount.followers_count.toLocaleString()} · Following: {successAccount.following_count.toLocaleString()}</p>
              </div>
            {:else}
              <button class="official-login-big-btn" onclick={handleNativeLogin} type="button">
                <svg viewBox="0 0 24 24" width="22" height="22" fill="currentColor">
                  <path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zm0-2.163c-3.259 0-3.667.014-4.947.072-4.358.2-6.78 2.618-6.98 6.98-.059 1.281-.073 1.689-.073 4.948 0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98 1.281.058 1.689.072 4.948.072 3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98-1.281-.059-1.69-.073-4.949-.073zm0 5.838c-3.403 0-6.162 2.759-6.162 6.162s2.759 6.163 6.162 6.163 6.162-2.759 6.162-6.163c0-3.403-2.759-6.162-6.162-6.162zm0 10.162c-2.209 0-4-1.79-4-4 0-2.209 1.791-4 4-4s4 1.791 4 4c0 2.21-1.791 4-4 4zm6.406-11.845c-.796 0-1.441.645-1.441 1.44s.645 1.44 1.441 1.44c.795 0 1.439-.645 1.439-1.44s-.644-1.44-1.439-1.44z"/>
                </svg>
                <span>Launch Official In-App Login</span>
                <span class="cta-arrow">→</span>
              </button>
              <p class="cta-subtext">Opens Instagram's official login screen directly. Zero token copying needed.</p>
            {/if}
          </div>

          {#if errorMessage}
            <div class="error-banner fade-in">
              <span class="err-icon">⚠️</span>
              <span>{errorMessage}</span>
            </div>
          {/if}

          <!-- Desktop / manual fallback option -->
          <div class="desktop-fallback-card">
            <span class="fallback-label mono">ALTERNATIVE OPTIONS</span>
            <div class="fallback-links">
              <button class="fallback-link-btn" onclick={() => { mode = 'manual'; errorMessage = null; }}>
                <span>🔑 Enter Cookie Tokens manually</span>
              </button>
              <button class="fallback-link-btn" onclick={() => { mode = 'demo'; errorMessage = null; }}>
                <span>⚡ Try Demo Mode (@meetzarc)</span>
              </button>
            </div>
          </div>
        </div>

      <!-- MANUAL COOKIE ENTRY FLOW -->
      {:else if mode === 'manual'}
        <div class="flow-container fade-in">
          <div class="instruction-box">
            <span class="sec-icon">🔒</span>
            <div>
              <strong>Client-Side Keystore Encryption:</strong>
              Your tokens are protected with AES-256 via hardware Keystore. They are never sent to external servers.
            </div>
          </div>

          <details class="help-accordion" open>
            <summary class="help-summary">
              <span>💡 How to find your cookies in 30 seconds</span>
              <span class="chevron">▾</span>
            </summary>
            <div class="help-content">
              <div class="help-step"><strong>1.</strong> Open <a href="https://www.instagram.com" target="_blank" rel="noreferrer" class="link">instagram.com</a> in your browser (logged in).</div>
              <div class="help-step"><strong>2.</strong> Press <kbd>F12</kbd> (or right click &gt; <em>Inspect</em>).</div>
              <div class="help-step"><strong>3.</strong> Go to <strong>Application</strong> tab (or <strong>Storage</strong> in Firefox).</div>
              <div class="help-step"><strong>4.</strong> In left sidebar, expand <strong>Cookies</strong> &gt; click <code>https://www.instagram.com</code>.</div>
              <div class="help-step"><strong>5.</strong> Copy <strong><code>ds_user_id</code></strong> and <strong><code>sessionid</code></strong> values.</div>
            </div>
          </details>

          <div class="inputs-grid">
            <div class="input-field">
              <label class="input-label" for="ds-user-id">
                INSTAGRAM USER ID (ds_user_id) <span class="required">*</span>
              </label>
              <input
                id="ds-user-id"
                type="text"
                class="text-input mono"
                bind:value={dsUserId}
                placeholder="e.g. 17841405392019"
                disabled={isValidating}
              />
            </div>

            <div class="input-field">
              <label class="input-label" for="session-id">
                SESSION COOKIE (sessionid) <span class="required">*</span>
              </label>
              <input
                id="session-id"
                type="password"
                class="text-input mono"
                bind:value={sessionId}
                placeholder="••••••••••••••••••••••••••••"
                disabled={isValidating}
              />
            </div>

            <div class="input-field">
              <label class="input-label" for="csrf-token">
                CSRF TOKEN (csrftoken) <span class="optional">(optional)</span>
              </label>
              <input
                id="csrf-token"
                type="text"
                class="text-input mono"
                bind:value={csrfToken}
                placeholder="csrftoken value"
                disabled={isValidating}
              />
            </div>
          </div>

          {#if errorMessage}
            <div class="error-banner fade-in">
              <span class="err-icon">⚠️</span>
              <span>{errorMessage}</span>
            </div>
          {/if}

          {#if successAccount}
            <div class="success-banner fade-in">
              <span class="succ-icon">✓</span>
              <span>Connected as @{successAccount.username}! Encrypting session...</span>
            </div>
          {/if}

          <div class="actions-row">
            <button
              class="primary-submit-btn"
              onclick={handleConnectSession}
              disabled={isValidating || !sessionId || !dsUserId}
            >
              {#if isValidating}
                <span class="spinner"></span>
                <span>VERIFYING SESSION...</span>
              {:else}
                <span>SAVE & VALIDATE SESSION</span>
              {/if}
            </button>
          </div>
        </div>

      <!-- DEMO FIXTURE ENVIRONMENT FLOW -->
      {:else}
        <div class="flow-container fade-in">
          <div class="demo-card">
            <div class="demo-icon">⚡</div>
            <div class="demo-body">
              <h3>Simulated Test Environment</h3>
              <p>Explore the full analytics suite instantly with 1,842 followers, mutual relationship graphs, and change history without connecting your personal account.</p>
            </div>
          </div>

          <div class="input-field">
            <label class="input-label" for="demo-username">DEMO HANDLE</label>
            <div class="prefix-input">
              <span class="prefix">@</span>
              <input
                id="demo-username"
                type="text"
                class="text-input"
                bind:value={mockUsername}
                disabled={isValidating}
              />
            </div>
          </div>

          {#if errorMessage}
            <div class="error-banner fade-in">
              <span class="err-icon">⚠️</span>
              <span>{errorMessage}</span>
            </div>
          {/if}

          {#if successAccount}
            <div class="success-banner fade-in">
              <span class="succ-icon">✓</span>
              <span>Demo environment loaded!</span>
            </div>
          {/if}

          <div class="actions-row">
            <button
              class="secondary-submit-btn"
              onclick={handleConnectMock}
              disabled={isValidating || !mockUsername.trim()}
            >
              {#if isValidating}
                <span class="spinner"></span>
                <span>GENERATING FIXTURE...</span>
              {:else}
                <span>LAUNCH DEMO SHOWCASE</span>
              {/if}
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 5, 8, 0.75);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    z-index: 100;
  }

  .modal-sheet {
    background: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-bottom: none;
    border-radius: 24px 24px 0 0;
    width: 100%;
    max-width: 540px;
    max-height: 90vh;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    padding: 20px 20px calc(24px + max(12px, env(safe-area-inset-bottom, 0px))) 20px;
    box-shadow: 0 -8px 40px rgba(0, 0, 0, 0.6);
  }

  .drag-handle {
    width: 38px;
    height: 4px;
    background: var(--border-strong);
    border-radius: 3px;
    margin: 0 auto 18px auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 20px;
  }

  .header-titles .title {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 6px 0 4px 0;
    letter-spacing: -0.02em;
  }

  .header-titles .subtitle {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .badge-row {
    display: flex;
    gap: 8px;
  }

  .badge {
    font-size: 10px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 6px;
    letter-spacing: 0.04em;
  }

  .badge.ig-badge {
    background: var(--ig-gradient);
    color: #ffffff;
  }

  .badge.secure {
    background: rgba(16, 185, 129, 0.12);
    color: var(--accent-positive);
    border: 1px solid rgba(16, 185, 129, 0.25);
  }

  .close-btn {
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    background: var(--bg-surface-hover);
    color: var(--text-primary);
  }

  /* Mode Tabs */
  .mode-tabs {
    display: flex;
    gap: 6px;
    background: var(--bg-root);
    padding: 5px;
    border-radius: 12px;
    border: 1px solid var(--border-subtle);
    margin-bottom: 20px;
  }

  .mode-btn {
    flex: 1;
    background: none;
    border: none;
    padding: 9px 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-tertiary);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.18s ease;
  }

  .mode-btn.active {
    background: var(--bg-surface-elevated);
    color: var(--text-primary);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  .flow-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Hero Card */
  .hero-card {
    display: flex;
    align-items: center;
    gap: 16px;
    background: var(--ig-gradient-subtle);
    border: 1px solid rgba(253, 29, 29, 0.25);
    border-radius: 16px;
    padding: 16px 18px;
  }

  .hero-icon-wrap {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    background: var(--ig-gradient);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ffffff;
    box-shadow: var(--ig-glow);
    flex-shrink: 0;
  }

  .hero-content h3 {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 3px;
  }

  .hero-content p {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  /* Safe badges */
  .safe-badges-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .safe-badge {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    padding: 10px 12px;
  }

  .safe-badge .safe-icon {
    font-size: 18px;
  }

  .safe-badge div {
    display: flex;
    flex-direction: column;
  }

  .safe-badge strong {
    font-size: 12px;
    color: var(--text-primary);
  }

  .safe-badge span {
    font-size: 10px;
    color: var(--text-tertiary);
  }

  /* Action Steps */
  /* Official In-App CTA */
  .official-cta-card {
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 16px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 10px;
  }

  .official-login-big-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    background: var(--ig-gradient);
    color: #ffffff;
    border: none;
    padding: 14px 20px;
    border-radius: 12px;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 16px rgba(225, 48, 108, 0.35);
    transition: transform 0.15s ease, opacity 0.15s ease, box-shadow 0.15s ease;
  }

  .official-login-big-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 6px 20px rgba(225, 48, 108, 0.45);
  }

  .official-login-big-btn:active {
    transform: scale(0.98);
  }

  .cta-arrow {
    font-size: 16px;
    transition: transform 0.15s ease;
  }

  .cta-subtext {
    font-size: 11px;
    color: var(--text-tertiary);
    line-height: 1.4;
    margin: 0;
  }

  /* Auth Loading State */
  .auth-loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
  }

  .auth-pulse-spinner {
    width: 36px;
    height: 36px;
    border: 3px solid rgba(225, 48, 108, 0.2);
    border-top-color: #e1306c;
    border-radius: 50%;
    animation: authSpin 0.8s linear infinite;
  }

  @keyframes authSpin {
    to { transform: rotate(360deg); }
  }

  .auth-status-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .auth-status-hint {
    font-size: 11px;
    color: var(--text-tertiary);
    margin: 0;
  }

  /* Auth Success State */
  .auth-success-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 10px 0;
  }

  .auth-check-icon {
    width: 36px;
    height: 36px;
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: var(--accent-positive);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    font-weight: 700;
  }

  /* Desktop Fallback Card */
  .desktop-fallback-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px dashed var(--border-subtle);
    border-radius: 12px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .fallback-label {
    font-size: 10px;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
  }

  .fallback-links {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .fallback-link-btn {
    background: none;
    border: none;
    text-align: left;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 6px 8px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .fallback-link-btn:hover {
    background: var(--bg-surface-hover);
    color: var(--text-primary);
  }

  /* Manual & Help */
  .instruction-box {
    display: flex;
    gap: 12px;
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    padding: 12px 14px;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .help-accordion {
    background: var(--bg-root);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    overflow: hidden;
    font-size: 12px;
  }

  .help-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 14px;
    cursor: pointer;
    font-weight: 600;
    color: var(--text-primary);
    list-style: none;
    user-select: none;
  }

  .help-summary::-webkit-details-marker {
    display: none;
  }

  .help-summary .chevron {
    color: var(--text-tertiary);
    transition: transform 0.2s;
  }

  .help-accordion[open] .help-summary .chevron {
    transform: rotate(180deg);
  }

  .help-content {
    padding: 10px 14px 14px 14px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .help-step strong {
    color: var(--text-primary);
  }

  .help-step code {
    background: rgba(255, 255, 255, 0.08);
    padding: 1px 5px;
    border-radius: 4px;
    color: var(--accent-signal);
  }

  .help-step kbd {
    background: rgba(255, 255, 255, 0.1);
    padding: 1px 6px;
    border-radius: 4px;
    border: 1px solid var(--border-subtle);
  }

  .help-step .link {
    color: var(--accent-primary);
    text-decoration: underline;
  }

  .inputs-grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .input-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.04em;
  }

  .required {
    color: var(--accent-negative);
  }

  .optional {
    color: var(--text-tertiary);
    font-weight: normal;
  }

  .text-input {
    width: 100%;
    background: var(--bg-root);
    border: 1px solid var(--border-strong);
    color: var(--text-primary);
    font-size: 13px;
    padding: 10px 14px;
    border-radius: 10px;
    outline: none;
    transition: border-color 0.15s;
  }

  .text-input:focus {
    border-color: var(--accent-primary);
  }

  .prefix-input {
    display: flex;
    align-items: center;
    background: var(--bg-root);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    padding-left: 12px;
  }

  .prefix-input .prefix {
    color: var(--text-tertiary);
    font-weight: 600;
  }

  .prefix-input .text-input {
    border: none;
    background: transparent;
  }

  /* Demo Card */
  .demo-card {
    display: flex;
    gap: 14px;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.25);
    border-radius: 14px;
    padding: 16px;
  }

  .demo-icon {
    font-size: 24px;
  }

  .demo-body h3 {
    font-size: 14px;
    font-weight: 700;
    color: var(--accent-signal);
    margin-bottom: 4px;
  }

  .demo-body p {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  /* Banners */
  .error-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(244, 63, 94, 0.1);
    border: 1px solid rgba(244, 63, 94, 0.3);
    color: var(--accent-negative);
    padding: 10px 14px;
    border-radius: 10px;
    font-size: 12px;
    font-weight: 500;
  }

  .success-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: var(--accent-positive);
    padding: 10px 14px;
    border-radius: 10px;
    font-size: 12px;
    font-weight: 600;
  }

  /* Actions */
  .actions-row {
    margin-top: 6px;
  }

  .primary-submit-btn {
    width: 100%;
    background: var(--ig-gradient);
    color: #ffffff;
    border: none;
    padding: 13px;
    border-radius: 12px;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.02em;
    cursor: pointer;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: opacity 0.15s, transform 0.15s;
  }

  .primary-submit-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    opacity: 0.95;
  }

  .primary-submit-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .secondary-submit-btn {
    width: 100%;
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-strong);
    color: var(--text-primary);
    padding: 13px;
    border-radius: 12px;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: background 0.15s;
  }

  .secondary-submit-btn:hover:not(:disabled) {
    background: var(--bg-surface-hover);
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 700ms linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
