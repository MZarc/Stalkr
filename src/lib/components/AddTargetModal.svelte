<script lang="ts">
  import { api, type Account, type PersonListItem, type TargetAccessCheckResult } from '../api';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';
  import Icon from './Icon.svelte';

  let {
    isOpen = false,
    ownerAccounts = [],
    activeOwnerId = '',
    onClose,
    onSuccess,
  }: {
    isOpen: boolean;
    ownerAccounts: Account[];
    activeOwnerId: string;
    onClose: () => void;
    onSuccess: (targetAccount: Account) => void;
  } = $props();

  let activeTab = $state<'following' | 'public'>('following');
  let selectedOwnerId = $state('');

  // Tab 1: From Following (with Pagination)
  let followingPeople = $state<PersonListItem[]>([]);
  let followingSearch = $state('');
  let isLoadingFollowing = $state(false);
  let currentPage = $state(1);
  let pageSize = $state(15);
  let totalCount = $state(0);

  const totalPages = $derived(Math.max(1, Math.ceil(totalCount / pageSize)));
  const startItem = $derived(totalCount === 0 ? 0 : (currentPage - 1) * pageSize + 1);
  const endItem = $derived(Math.min(totalCount, currentPage * pageSize));

  // Tab 2: Public Account
  let publicUsername = $state('');
  const publicSuggestions = ['cristiano', 'instagram', 'apple', 'nasa', 'zuck'];

  // Shared state
  let isSubmitting = $state(false);
  let addingTargetUsername = $state<string | null>(null);
  let errorMessage = $state<string | null>(null);
  let successFeedback = $state<string | null>(null);
  let accessResult = $state<TargetAccessCheckResult | null>(null);

  $effect(() => {
    if (activeOwnerId && !selectedOwnerId) {
      selectedOwnerId = activeOwnerId;
    } else if (ownerAccounts.length > 0 && !selectedOwnerId) {
      selectedOwnerId = ownerAccounts[0].id;
    }
  });

  $effect(() => {
    if (isOpen && selectedOwnerId) {
      loadFollowingList(1);
    }
  });

  async function loadFollowingList(targetPage: number = 1) {
    if (!selectedOwnerId) return;
    isLoadingFollowing = true;
    currentPage = targetPage;
    try {
      const q = followingSearch.trim() || null;
      // Global search across all followers and people in the account
      const filterType = q ? 'all' : 'following';
      const [count, people] = await Promise.all([
        api.getPeopleCount(selectedOwnerId, filterType, q),
        api.getPeople(selectedOwnerId, filterType, q, 'mutual_first', pageSize, (targetPage - 1) * pageSize)
      ]);
      totalCount = count;
      followingPeople = people;
    } catch (e) {
      console.error('Failed to load following/people list:', e);
    } finally {
      isLoadingFollowing = false;
    }
  }

  function goToPage(targetPage: number) {
    if (targetPage < 1 || targetPage > totalPages || targetPage === currentPage || isLoadingFollowing) return;
    loadFollowingList(targetPage);
  }

  let searchDebounce: any;
  function handleFollowingSearch() {
    clearTimeout(searchDebounce);
    searchDebounce = setTimeout(() => {
      loadFollowingList(1);
    }, 200);
  }

  const cleanSearch = $derived(followingSearch.trim().replace(/^@/, ''));
  const cleanPublicInput = $derived(publicUsername.trim().replace(/^@/, ''));

  const filteredFollowing = $derived(followingPeople);

  const hasExactMatchInFollowing = $derived.by(() => {
    if (!cleanSearch) return true;
    return followingPeople.some(
      (p) => p.username.toLowerCase() === cleanSearch.toLowerCase()
    );
  });

  async function handleAddTarget(username: string) {
    const cleanHandle = username.trim().replace(/^@/, '');
    if (!cleanHandle) {
      errorMessage = 'Please enter a valid Instagram username.';
      return;
    }
    if (!selectedOwnerId) {
      errorMessage = 'Please connect an Instagram account first.';
      return;
    }

    isSubmitting = true;
    addingTargetUsername = cleanHandle;
    errorMessage = null;
    successFeedback = null;
    accessResult = null;

    try {
      const res = await api.addTargetAccount(selectedOwnerId, cleanHandle);
      accessResult = res.access_check;

      if (res.access_check.access_state === 'not_accessible') {
        errorMessage = res.access_check.access_reason || 'This account is private and not followed by your connected account.';
        isSubmitting = false;
        addingTargetUsername = null;
        return;
      }

      successFeedback = `Target @${cleanHandle} locked & active`;
      setTimeout(() => {
        onSuccess(res.account);
        handleClose();
      }, 900);
    } catch (err: any) {
      errorMessage = typeof err === 'string' ? err : err?.message || 'Failed to inspect target account.';
    } finally {
      isSubmitting = false;
      addingTargetUsername = null;
    }
  }

  function handleClose() {
    followingSearch = '';
    publicUsername = '';
    errorMessage = null;
    successFeedback = null;
    accessResult = null;
    currentPage = 1;
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    }
  }

  const selectedOwner = $derived(
    ownerAccounts.find((o) => o.id === selectedOwnerId) || ownerAccounts[0]
  );
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop fade-in" onclick={handleClose} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="modal-window scale-in"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby="target-modal-title"
      tabindex="-1"
    >
      <!-- Top Accent Bar -->
      <div class="modal-top-accent"></div>

      <!-- Header -->
      <div class="modal-header">
        <div class="header-main">
          <div class="header-icon-badge">
            <Icon name="plus" size={20} strokeWidth={2.4} />
          </div>
          <div class="header-titles">
            <div class="meta-row">
              <span class="category-tag mono">TARGET HUNT</span>
              {#if selectedOwner}
                <div class="auth-pill mono" title="Queries authenticated through your connected profile">
                  <span class="auth-dot"></span>
                  <span>via @{selectedOwner.username}</span>
                </div>
              {/if}
            </div>
            <h2 id="target-modal-title" class="title">Instagram Target Hunt</h2>
            <p class="subtitle">Monitor follower gains, unfollowers & relationship shifts</p>
          </div>
        </div>

        <button class="close-btn" onclick={handleClose} aria-label="Close modal" title="Close (Esc)">
          <Icon name="close" size={16} strokeWidth={2.4} />
        </button>
      </div>

      <!-- Multi-Owner Account Switcher (if user connected >1 account) -->
      {#if ownerAccounts.length > 1}
        <div class="owner-selector-bar">
          <span class="owner-label mono">Query session:</span>
          <div class="owner-chips">
            {#each ownerAccounts as owner}
              <button
                class="owner-chip-btn {selectedOwnerId === owner.id ? 'active' : ''}"
                onclick={() => { selectedOwnerId = owner.id; loadFollowingList(); }}
              >
                @{owner.username}
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Custom Segmented Control -->
      <div class="segmented-control" role="tablist">
        <button
          role="tab"
          aria-selected={activeTab === 'following'}
          class="segment-tab {activeTab === 'following' ? 'active' : ''}"
          onclick={() => { activeTab = 'following'; errorMessage = null; }}
        >
          <div class="tab-icon-wrap">
            <Icon name="lock" size={16} strokeWidth={2.2} />
          </div>
          <div class="tab-label-stack">
            <span class="tab-title">Private Friend / Target</span>
            <span class="tab-caption">Accounts you follow</span>
          </div>
          <span class="tab-pill lock">Access-Gated</span>
        </button>

        <button
          role="tab"
          aria-selected={activeTab === 'public'}
          class="segment-tab {activeTab === 'public' ? 'active' : ''}"
          onclick={() => { activeTab = 'public'; errorMessage = null; }}
        >
          <div class="tab-icon-wrap">
            <Icon name="users" size={16} strokeWidth={2.2} />
          </div>
          <div class="tab-label-stack">
            <span class="tab-title">Any Public Profile</span>
            <span class="tab-caption">Creators & public accounts</span>
          </div>
          <span class="tab-pill open">No Follow Needed</span>
        </button>
      </div>

      <!-- Tab Content Area -->
      <div class="tab-content-container">
        <!-- Tab 1: From Following (Private Target / Friends) -->
        {#if activeTab === 'following'}
          <div class="tab-pane fade-in">
            <!-- Strategic Insight Banner -->
            <div class="principle-banner">
              <div class="principle-icon">
                <Icon name="shield" size={16} strokeWidth={2.2} />
              </div>
              <div class="principle-text">
                <strong>Private Target Access:</strong> Instagram strictly hides follower lists of private accounts. Because <em>you already follow them</em>, Stalkr utilizes your approved access to monitor their follower additions & drops.
              </div>
            </div>

            <!-- Smart Unified Search Bar -->
            <div class="search-input-shell">
              <span class="at-prefix">@</span>
              <input
                id="following-search-input"
                type="text"
                class="search-text-input"
                placeholder="Search globally across all followers & people..."
                bind:value={followingSearch}
                oninput={handleFollowingSearch}
                onkeydown={(e) => {
                  if (e.key === 'Enter' && cleanSearch) {
                    handleAddTarget(cleanSearch);
                  }
                }}
              />
              {#if followingSearch}
                <button
                  class="clear-btn"
                  onclick={() => { followingSearch = ''; loadFollowingList(1); }}
                  aria-label="Clear search"
                >✕</button>
              {/if}
            </div>

            <!-- Direct Entry Action (When user types a handle not yet matched in cached circle) -->
            {#if cleanSearch && !hasExactMatchInFollowing}
              <div class="direct-track-card fade-in">
                <div class="direct-meta">
                  <div class="direct-avatar mono">
                    {cleanSearch.substring(0, 1).toUpperCase()}
                  </div>
                  <div class="direct-info">
                    <div class="direct-handle">
                      <span>Track @{cleanSearch} directly</span>
                      <span class="direct-chip mono">Direct Hunt</span>
                    </div>
                    <p class="direct-sub">Not seen in initial cache? Stalkr will verify your follower permissions directly.</p>
                  </div>
                </div>

                <button
                  class="direct-track-btn"
                  onclick={() => handleAddTarget(cleanSearch)}
                  disabled={isSubmitting}
                >
                  {#if isSubmitting && addingTargetUsername === cleanSearch}
                    <span class="spinner-small"></span>
                  {:else}
                    <span>Inspect & Track →</span>
                  {/if}
                </button>
              </div>
            {/if}

            <!-- Following & People Count & Pagination Summary -->
            {#if totalCount > 0}
              <div class="following-count-row mono">
                <span class="count-total">{totalCount.toLocaleString()} {cleanSearch ? 'MATCHING PEOPLE & FOLLOWERS' : 'ACCOUNTS IN CIRCLE'}</span>
                <span class="count-range">Page {currentPage} of {totalPages} ({startItem}–{endItem})</span>
              </div>
            {/if}

            <!-- Scrollable Following List -->
            <div class="people-list-container">
              {#if isLoadingFollowing}
                <div class="empty-state-card">
                  <span class="spinner"></span>
                  <span class="empty-title">Retrieving your following circle...</span>
                </div>
              {:else if filteredFollowing.length === 0 && !cleanSearch}
                <div class="empty-state-card">
                  <div class="empty-icon-circle">👥</div>
                  <h4 class="empty-title">No cached following records yet</h4>
                  <p class="empty-sub">
                    Enter any friend or private target's username in the search bar above to start tracking immediately.
                  </p>
                </div>
              {:else if filteredFollowing.length === 0 && cleanSearch}
                <div class="empty-state-card">
                  <div class="empty-icon-circle">🔍</div>
                  <h4 class="empty-title">No matches found</h4>
                  <p class="empty-sub">
                    No accounts matching "@{cleanSearch}". Use the "Direct Hunt" card above to inspect & track directly.
                  </p>
                </div>
              {:else}
                <div class="following-grid">
                  {#each filteredFollowing as person}
                    <div class="person-item-card">
                      <div class="person-avatar-wrap">
                        <img
                          src={getAvatarUrl(person.username, person.avatar_url)}
                          alt={person.username}
                          class="person-avatar-img"
                          loading="lazy"
                          referrerpolicy="no-referrer"
                          onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(person.username); }}
                        />
                        {#if person.is_private}
                          <span class="lock-indicator" title="Private Account">🔒</span>
                        {/if}
                      </div>

                      <div class="person-details">
                        <div class="person-row-top">
                          <span class="person-handle">@{person.username}</span>
                          {#if person.is_verified}
                            <span class="verified-dot" title="Verified">✓</span>
                          {/if}
                          {#if person.is_mutual}
                            <span class="badge-mutual mono">Mutual</span>
                          {/if}
                        </div>
                        <span class="person-name">{person.display_name || person.username}</span>
                      </div>

                      <button
                        class="track-target-btn"
                        onclick={() => handleAddTarget(person.username)}
                        disabled={isSubmitting}
                        title="Start tracking @{person.username}"
                      >
                        {#if addingTargetUsername === person.username}
                          <span class="spinner-small"></span>
                        {:else}
                          <span class="reticle-mini">⌖</span>
                          <span>Track Target</span>
                        {/if}
                      </button>
                    </div>
                  {/each}
                </div>

                <!-- Pagination Footer Navigation -->
                {#if totalPages > 1}
                  <div class="modal-pagination-bar">
                    <button
                      class="modal-page-nav-btn prev"
                      onclick={() => goToPage(currentPage - 1)}
                      disabled={currentPage <= 1 || isLoadingFollowing}
                      type="button"
                    >
                      <Icon name="chevron-left" size={13} strokeWidth={2.4} />
                      <span>Prev</span>
                    </button>

                    <div class="modal-pages-strip">
                      {#each Array.from({ length: totalPages }, (_, i) => i + 1) as p}
                        {#if totalPages <= 6 || p === 1 || p === totalPages || (p >= currentPage - 1 && p <= currentPage + 1)}
                          <button
                            class="modal-page-pill {currentPage === p ? 'active' : ''}"
                            onclick={() => goToPage(p)}
                            disabled={isLoadingFollowing}
                            type="button"
                          >
                            {p}
                          </button>
                        {:else if (p === 2 && currentPage > 3) || (p === totalPages - 1 && currentPage < totalPages - 2)}
                          <span class="modal-page-ellipsis">…</span>
                        {/if}
                      {/each}
                    </div>

                    <button
                      class="modal-page-nav-btn next"
                      onclick={() => goToPage(currentPage + 1)}
                      disabled={currentPage >= totalPages || isLoadingFollowing}
                      type="button"
                    >
                      <span>Next</span>
                      <Icon name="chevron-right" size={13} strokeWidth={2.4} />
                    </button>
                  </div>
                {/if}
              {/if}
            </div>
          </div>
        {/if}

        <!-- Tab 2: Any Public Account -->
        {#if activeTab === 'public'}
          <div class="tab-pane fade-in">
            <!-- Strategic Insight Banner -->
            <div class="principle-banner public">
              <div class="principle-icon">
                <Icon name="users" size={16} strokeWidth={2.2} />
              </div>
              <div class="principle-text">
                <strong>Public Audience Tracking:</strong> Track any public creator, brand, competitor, or influencer. Relationship lists are openly accessible with no following or approval required.
              </div>
            </div>

            <!-- Public Handle Input Card -->
            <div class="public-input-card">
              <label class="input-field-label mono" for="public-target-input">
                TARGET INSTAGRAM USERNAME
              </label>

              <div class="public-input-row">
                <span class="public-at-badge">@</span>
                <input
                  id="public-target-input"
                  type="text"
                  class="public-text-input"
                  placeholder="creator, brand, or public account"
                  bind:value={publicUsername}
                  disabled={isSubmitting}
                  onkeydown={(e) => {
                    if (e.key === 'Enter' && cleanPublicInput) {
                      handleAddTarget(cleanPublicInput);
                    }
                  }}
                />
                {#if publicUsername}
                  <button
                    class="clear-btn"
                    onclick={() => { publicUsername = ''; errorMessage = null; accessResult = null; }}
                    aria-label="Clear input"
                  >✕</button>
                {/if}
              </div>

              <!-- Quick-Pick Popular Suggestions -->
              <div class="suggestions-row">
                <span class="sugg-label mono">Quick pick:</span>
                <div class="sugg-chips">
                  {#each publicSuggestions as sugg}
                    <button
                      type="button"
                      class="sugg-chip"
                      onclick={() => { publicUsername = sugg; handleAddTarget(sugg); }}
                    >
                      @{sugg}
                    </button>
                  {/each}
                </div>
              </div>
            </div>

            <!-- Inspect & Track Action Button -->
            <button
              class="primary-hunt-btn"
              onclick={() => handleAddTarget(cleanPublicInput)}
              disabled={isSubmitting || !cleanPublicInput}
            >
              {#if isSubmitting}
                <span class="spinner"></span>
                <span>Verifying Profile Access...</span>
              {:else}
                <Icon name="plus" size={16} strokeWidth={2.4} />
                <span>Track Public Profile →</span>
              {/if}
            </button>
          </div>
        {/if}
      </div>

      <!-- Feedback Banners -->
      {#if errorMessage}
        <div class="feedback-card error fade-in">
          <div class="feedback-icon error">⚠️</div>
          <div class="feedback-body">
            <span class="feedback-title mono">TRACKING RESTRICTED</span>
            <p class="feedback-msg">{errorMessage}</p>
          </div>
        </div>
      {/if}

      {#if successFeedback}
        <div class="feedback-card success fade-in">
          <div class="feedback-icon success">✓</div>
          <div class="feedback-body">
            <span class="feedback-title mono">TARGET ACQUIRED</span>
            <p class="feedback-msg">{successFeedback}</p>
          </div>
        </div>
      {/if}

      <!-- Target Access Pre-Flight Preview -->
      {#if accessResult && !errorMessage}
        <div class="access-preview-card fade-in">
          <div class="preview-header">
            <span class="preview-badge {accessResult.access_state} mono">
              {accessResult.access_state === 'accessible' ? '✓ FULL ACCESS CONFIRMED' : 'RESTRICTED ACCESS'}
            </span>
            <span class="privacy-tag mono">{accessResult.privacy_state.toUpperCase()} PROFILE</span>
          </div>
          <p class="preview-reason">{accessResult.access_reason}</p>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(4, 2, 9, 0.86);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    z-index: 1000;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 16px;
  }

  .modal-window {
    width: 100%;
    max-width: 560px;
    background: linear-gradient(180deg, #130d22 0%, #0d0817 100%);
    border: 1px solid rgba(168, 85, 247, 0.28);
    border-radius: 20px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.8), 0 0 0 1px rgba(168, 85, 247, 0.12);
    display: flex;
    flex-direction: column;
    max-height: 90vh;
    overflow: hidden;
    position: relative;
    outline: none;
  }

  .modal-top-accent {
    height: 3px;
    width: 100%;
    background: linear-gradient(90deg, #7c3aed 0%, #a855f7 50%, #ec4899 100%);
  }

  /* Header */
  .modal-header {
    padding: 18px 22px 14px;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(255, 255, 255, 0.015);
  }

  .header-main {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }

  .header-icon-badge {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.22) 0%, rgba(126, 34, 206, 0.12) 100%);
    border: 1px solid rgba(168, 85, 247, 0.35);
    color: var(--accent-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .header-titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .category-tag {
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    background: rgba(168, 85, 247, 0.18);
    color: #e9d5ff;
    padding: 2px 7px;
    border-radius: 4px;
    border: 1px solid rgba(168, 85, 247, 0.3);
  }

  .auth-pill {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    color: var(--text-tertiary);
    background: rgba(255, 255, 255, 0.04);
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid var(--border-subtle);
  }

  .auth-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #34d399;
  }

  .title {
    font-size: 19px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text-primary);
    margin: 0;
  }

  .subtitle {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 0;
  }

  .close-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.15s;
    flex-shrink: 0;
  }

  .close-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
  }

  /* Multi-Owner Selector */
  .owner-selector-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 22px;
    background: rgba(20, 13, 38, 0.6);
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .owner-label {
    font-size: 10px;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
  }

  .owner-chips {
    display: flex;
    gap: 6px;
  }

  .owner-chip-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 6px;
    cursor: pointer;
  }

  .owner-chip-btn.active {
    background: rgba(168, 85, 247, 0.2);
    border-color: var(--accent-primary);
    color: #ffffff;
    font-weight: 600;
  }

  /* Custom Segmented Control */
  .segmented-control {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 14px 22px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(0, 0, 0, 0.15);
  }

  .segment-tab {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    cursor: pointer;
    text-align: left;
    transition: all 0.16s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
  }

  .segment-tab:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(168, 85, 247, 0.25);
  }

  .segment-tab.active {
    background: linear-gradient(135deg, rgba(142, 68, 173, 0.24) 0%, rgba(107, 33, 168, 0.15) 100%);
    border: 1px solid rgba(168, 85, 247, 0.5);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
  }

  .tab-icon-wrap {
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .segment-tab.active .tab-icon-wrap {
    color: var(--accent-primary);
  }

  .tab-label-stack {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .tab-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .segment-tab.active .tab-title {
    color: #ffffff;
  }

  .tab-caption {
    font-size: 10px;
    color: var(--text-tertiary);
    white-space: nowrap;
  }

  .tab-pill {
    position: absolute;
    top: -5px;
    right: 8px;
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 1px 5px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .tab-pill.lock {
    background: rgba(245, 158, 11, 0.2);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.4);
  }

  .tab-pill.open {
    background: rgba(16, 185, 129, 0.2);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.4);
  }

  /* Tab Content */
  .tab-content-container {
    padding: 18px 22px;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .tab-pane {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Principle Banner */
  .principle-banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: rgba(142, 68, 173, 0.08);
    border: 1px solid rgba(168, 85, 247, 0.22);
    border-radius: 10px;
    padding: 10px 12px;
    font-size: 11px;
    line-height: 1.45;
    color: #d8b4fe;
  }

  .principle-banner.public {
    background: rgba(16, 185, 129, 0.08);
    border-color: rgba(16, 185, 129, 0.22);
    color: #a7f3d0;
  }

  .principle-icon {
    flex-shrink: 0;
    margin-top: 1px;
  }

  /* Unified Search Bar */
  .search-input-shell {
    display: flex;
    align-items: center;
    background: rgba(15, 10, 28, 0.85);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    padding: 2px 10px;
    transition: all 0.16s ease;
  }

  .search-input-shell:focus-within {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px rgba(168, 85, 247, 0.2);
    background: rgba(20, 14, 38, 0.95);
  }

  .at-prefix {
    font-size: 14px;
    font-weight: 700;
    color: var(--accent-primary);
    margin-right: 6px;
  }

  .search-text-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    padding: 8px 0;
    outline: none;
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    font-size: 12px;
    padding: 4px;
  }

  .clear-btn:hover {
    color: #ffffff;
  }

  /* Direct Track Card */
  .direct-track-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: rgba(168, 85, 247, 0.12);
    border: 1px solid rgba(168, 85, 247, 0.35);
    border-radius: 10px;
  }

  .direct-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .direct-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: rgba(168, 85, 247, 0.25);
    color: #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .direct-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .direct-handle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
    color: #ffffff;
  }

  .direct-chip {
    font-size: 8px;
    background: rgba(168, 85, 247, 0.3);
    color: #f3e8ff;
    padding: 1px 4px;
    border-radius: 3px;
  }

  .direct-sub {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0;
  }

  .direct-track-btn {
    background: var(--accent-primary);
    border: none;
    color: #ffffff;
    font-size: 11px;
    font-weight: 600;
    padding: 7px 12px;
    border-radius: 8px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
  }

  .direct-track-btn:hover:not(:disabled) {
    background: var(--accent-purple-light);
    transform: translateY(-1px);
  }

  /* Following Count & Pagination Bar */
  .following-count-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 10px;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
    padding: 0 4px;
    margin-bottom: 4px;
  }

  .following-count-row .count-total {
    color: var(--accent-purple-light);
    font-weight: 600;
  }

  .following-count-row .count-range {
    color: var(--text-secondary);
  }

  /* People List */
  .people-list-container {
    max-height: 300px;
    overflow-y: auto;
    padding-right: 4px;
  }

  .modal-pagination-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 10px;
    padding: 10px 4px 4px 4px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .modal-page-nav-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .modal-page-nav-btn:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.15);
    border-color: rgba(168, 85, 247, 0.35);
    color: #ffffff;
  }

  .modal-page-nav-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .modal-pages-strip {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .modal-page-pill {
    min-width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    color: var(--text-tertiary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .modal-page-pill:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .modal-page-pill.active {
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.35) 0%, rgba(139, 92, 246, 0.25) 100%);
    border-color: rgba(168, 85, 247, 0.6);
    color: #ffffff;
    font-weight: 700;
  }

  .modal-page-ellipsis {
    color: var(--text-tertiary);
    font-size: 11px;
    padding: 0 2px;
  }

  .following-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .person-item-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    transition: all 0.14s ease;
  }

  .person-item-card:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(168, 85, 247, 0.25);
  }

  .person-avatar-wrap {
    position: relative;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
  }

  .person-avatar-img, .person-avatar-fallback {
    width: 36px;
    height: 36px;
    aspect-ratio: 1 / 1;
    object-fit: cover;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 700;
  }

  .lock-indicator {
    position: absolute;
    bottom: -2px;
    right: -2px;
    font-size: 10px;
    background: #0d0817;
    border-radius: 50%;
    padding: 1px;
    line-height: 1;
  }

  .person-details {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .person-row-top {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .person-handle {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .verified-dot {
    color: #38bdf8;
    font-size: 11px;
    font-weight: 800;
  }

  .badge-mutual {
    font-size: 8px;
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    padding: 1px 4px;
    border-radius: 3px;
  }

  .person-name {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .track-target-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: rgba(168, 85, 247, 0.14);
    border: 1px solid rgba(168, 85, 247, 0.35);
    color: #e9d5ff;
    font-size: 11px;
    font-weight: 600;
    padding: 6px 12px;
    border-radius: 8px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.14s ease;
  }

  .track-target-btn:hover:not(:disabled) {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    color: #ffffff;
    transform: translateY(-1px);
  }

  .reticle-mini {
    font-size: 13px;
    line-height: 1;
    color: var(--accent-primary);
  }

  .track-target-btn:hover .reticle-mini {
    color: #ffffff;
  }

  /* Public Tab Inputs */
  .public-input-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .input-field-label {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.06em;
  }

  .public-input-row {
    display: flex;
    align-items: center;
    background: rgba(15, 10, 28, 0.9);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    padding: 3px 12px;
    transition: all 0.16s ease;
  }

  .public-input-row:focus-within {
    border-color: var(--accent-primary);
    box-shadow: 0 0 0 2px rgba(168, 85, 247, 0.2);
  }

  .public-at-badge {
    font-size: 16px;
    font-weight: 700;
    color: var(--accent-primary);
    margin-right: 8px;
  }

  .public-text-input {
    flex: 1;
    background: transparent;
    border: none;
    color: #ffffff;
    font-size: 14px;
    font-weight: 500;
    padding: 10px 0;
    outline: none;
  }

  .suggestions-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 4px;
  }

  .sugg-label {
    font-size: 10px;
    color: var(--text-tertiary);
  }

  .sugg-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .sugg-chip {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--text-secondary);
    font-size: 10px;
    padding: 3px 8px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.14s ease;
  }

  .sugg-chip:hover {
    background: rgba(168, 85, 247, 0.2);
    border-color: var(--accent-primary);
    color: #ffffff;
  }

  .primary-hunt-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    background: linear-gradient(135deg, #9d4edd 0%, #7b2cbf 100%);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #ffffff;
    font-size: 13px;
    font-weight: 700;
    padding: 12px;
    border-radius: 11px;
    cursor: pointer;
    box-shadow: 0 4px 16px rgba(123, 44, 191, 0.35);
    transition: all 0.16s ease;
  }

  .primary-hunt-btn:hover:not(:disabled) {
    background: linear-gradient(135deg, #b05fe6 0%, #8a38d1 100%);
    transform: translateY(-1px);
    box-shadow: 0 6px 20px rgba(123, 44, 191, 0.45);
  }

  .primary-hunt-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Feedback & Previews */
  .feedback-card {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 10px;
    margin: 0 22px 14px;
  }

  .feedback-card.error {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.35);
    color: #fca5a5;
  }

  .feedback-card.success {
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.35);
    color: #a7f3d0;
  }

  .feedback-icon {
    font-size: 14px;
    font-weight: 700;
  }

  .feedback-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .feedback-title {
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.06em;
  }

  .feedback-msg {
    font-size: 12px;
    margin: 0;
    line-height: 1.4;
  }

  .access-preview-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    padding: 10px 14px;
    margin: 0 22px 14px;
  }

  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .preview-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .preview-badge.accessible {
    background: rgba(16, 185, 129, 0.18);
    color: #34d399;
  }

  .preview-badge.not_accessible {
    background: rgba(239, 68, 68, 0.18);
    color: #f87171;
  }

  .privacy-tag {
    font-size: 9px;
    color: var(--text-tertiary);
  }

  .preview-reason {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0;
  }

  .empty-state-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 28px 16px;
    text-align: center;
    gap: 8px;
  }

  .empty-icon-circle {
    font-size: 28px;
    margin-bottom: 2px;
  }

  .empty-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .empty-sub {
    font-size: 11px;
    color: var(--text-tertiary);
    margin: 0;
    max-width: 320px;
    line-height: 1.45;
  }

  .spinner-small {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    display: inline-block;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes scaleInModal {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(6px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .scale-in {
    animation: scaleInModal 180ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  /* Responsive Mobile Adaptation */
  @media (max-width: 600px) {
    .modal-backdrop {
      align-items: flex-end;
      padding: 0;
    }

    .modal-window {
      max-width: 100%;
      border-radius: 22px 22px 0 0;
      max-height: 88vh;
      padding-bottom: max(16px, env(safe-area-inset-bottom, 0px));
    }

    .segmented-control {
      grid-template-columns: 1fr;
    }
  }
</style>
