<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Account, type PersonListItem, type RelationshipSummary, type TargetOverlapReport } from '../api';
  import PersonCard from '../components/PersonCard.svelte';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';
  import { globalSync } from '../syncState.svelte';
  import Icon from '../components/Icon.svelte';

  let {
    account,
    ownerAccount = null,
    onSelectPerson,
    initialFilter = 'all',
  }: {
    account: Account;
    ownerAccount?: Account | null;
    onSelectPerson: (personId: string, username: string) => void;
    initialFilter?: string;
  } = $props();

  // Active filter, search, and custom sort state
  // svelte-ignore state_referenced_locally
  let filterType = $state<string>(initialFilter || 'all');
  let searchQuery = $state<string>('');
  let sortBy = $state<'mutual_first' | 'name_asc' | 'followers_first'>('mutual_first');
  let isSortOpen = $state(false);
  let isLoading = $state(false);

  // Pagination state
  let currentPage = $state(1);
  let pageSize = $state(50);
  let totalCount = $state(0);
  let jumpPageInput = $state('');

  const totalPages = $derived(Math.max(1, Math.ceil(totalCount / pageSize)));
  const startItem = $derived(totalCount === 0 ? 0 : (currentPage - 1) * pageSize + 1);
  const endItem = $derived(Math.min(totalCount, currentPage * pageSize));

  const paginationRange = $derived.by(() => {
    const total = totalPages;
    const current = currentPage;
    if (total <= 7) {
      return Array.from({ length: total }, (_, i) => i + 1);
    }

    const pages: (number | 'ellipsis')[] = [1];
    if (current > 3) {
      pages.push('ellipsis');
    }

    const start = Math.max(2, current - 1);
    const end = Math.min(total - 1, current + 1);
    for (let i = start; i <= end; i++) {
      pages.push(i);
    }

    if (current < total - 2) {
      pages.push('ellipsis');
    }
    pages.push(total);
    return pages;
  });

  const sortOptions = [
    { id: 'mutual_first', label: 'Mutuals First', icon: '✨' },
    { id: 'name_asc', label: 'A-Z (Alphabetical)', icon: '🔤' },
    { id: 'followers_first', label: 'Followers First', icon: '★' },
  ] as const;

  const currentSort = $derived(
    sortOptions.find((opt) => opt.id === sortBy) || sortOptions[0]
  );

  // Loaded data
  let peopleList = $state<PersonListItem[]>([]);
  let summary = $state<RelationshipSummary>({
    followers: 0,
    following: 0,
    mutual: 0,
    not_following_back: 0,
    fans: 0,
    net_delta_7d: 0,
  });
  let overlapReport = $state<TargetOverlapReport | null>(null);
  let sharedUsernames = $state<Set<string>>(new Set());

  // Track account to avoid duplicate loads
  let loadedAccountId = $state<string>('');

  // Initial load or when active account changes
  $effect(() => {
    const accId = account?.id;
    if (accId) {
      const curFilter = untrack(() => filterType);
      loadData(accId, curFilter, 1);
    }
  });

  $effect(() => {
    if (globalSync.syncVersion > 0 && account?.id) {
      const curFilter = untrack(() => filterType);
      const curPage = untrack(() => currentPage);
      loadData(account.id, curFilter, curPage);
    }
  });

  async function loadData(accId: string, activeFilter: string, targetPage: number = 1) {
    isLoading = true;
    currentPage = targetPage;
    try {
      // 1. Fetch summary metrics for segment cards
      try {
        const sum = await api.getRelationshipSummary(accId);
        summary = sum;
      } catch (err) {
        console.error('Failed to load relationship summary:', err);
      }

      // 2. Fetch shared overlap if monitored target
      if (account.account_kind === 'monitored' && ownerAccount?.id && ownerAccount.id !== account.id) {
        try {
          const overlap = await api.getTargetOverlap(ownerAccount.id, account.id, 50);
          overlapReport = overlap;
          sharedUsernames = new Set(overlap.shared_people.map((p) => p.username.toLowerCase()));
        } catch {
          overlapReport = null;
          sharedUsernames = new Set();
        }
      } else {
        overlapReport = null;
        sharedUsernames = new Set();
      }

      // 3. Fetch people with true database pagination
      const backendFilter = activeFilter === 'bridge' ? 'all' : activeFilter;
      const cleanQuery = searchQuery.trim().replace(/^@/, '');
      const q = cleanQuery.length > 0 ? cleanQuery : null;

      if (activeFilter === 'bridge') {
        let fullOverlap = overlapReport?.shared_people ? [...overlapReport.shared_people] : [];
        if (q) {
          const lq = q.toLowerCase();
          fullOverlap = fullOverlap.filter(
            (p) =>
              p.username.toLowerCase().includes(lq) ||
              (p.display_name && p.display_name.toLowerCase().includes(lq))
          );
        }
        if (sortBy === 'mutual_first') {
          fullOverlap.sort((a, b) => (b.is_mutual ? 1 : 0) - (a.is_mutual ? 1 : 0) || a.username.localeCompare(b.username));
        } else if (sortBy === 'followers_first') {
          fullOverlap.sort((a, b) => (b.is_follower ? 1 : 0) - (a.is_follower ? 1 : 0) || a.username.localeCompare(b.username));
        } else {
          fullOverlap.sort((a, b) => a.username.localeCompare(b.username));
        }
        totalCount = fullOverlap.length;
        const offset = (targetPage - 1) * pageSize;
        peopleList = fullOverlap.slice(offset, offset + pageSize);
      } else {
        const count = await api.getPeopleCount(accId, backendFilter, q);
        totalCount = count;

        const offset = (targetPage - 1) * pageSize;
        const res = await api.getPeople(accId, backendFilter, q, sortBy, pageSize, offset);
        peopleList = res;
      }
    } catch (e) {
      console.error('Failed to load circle members:', e);
      peopleList = [];
      totalCount = 0;
    } finally {
      isLoading = false;
      loadedAccountId = accId;
    }
  }

  function goToPage(page: number) {
    if (page < 1 || page > totalPages || page === currentPage || isLoading) return;
    if (account?.id) {
      loadData(account.id, filterType, page);
      const rosterEl = document.querySelector('.members-container');
      if (rosterEl) {
        rosterEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
    }
  }

  function handleJumpSubmit(e: SubmitEvent | KeyboardEvent) {
    if ('key' in e && e.key !== 'Enter') return;
    if (e.cancelable) e.preventDefault();
    const p = parseInt(jumpPageInput.trim(), 10);
    if (!isNaN(p) && p >= 1 && p <= totalPages) {
      jumpPageInput = '';
      goToPage(p);
    }
  }

  function handlePageSizeChange(newSize: number) {
    pageSize = newSize;
    if (account?.id) {
      loadData(account.id, filterType, 1);
    }
  }

  let searchDebounce: any;
  function handleSearchInput() {
    clearTimeout(searchDebounce);
    searchDebounce = setTimeout(() => {
      if (account?.id) {
        loadData(account.id, filterType, 1);
      }
    }, 200);
  }

  function handleFilterSelect(newFilter: string) {
    filterType = newFilter;
    if (account?.id) {
      loadData(account.id, newFilter, 1);
    }
  }

  // Database-paged and sorted list
  const displayedPeople = $derived(peopleList);

  const totalMembers = $derived(
    summary.mutual + summary.fans + summary.not_following_back || peopleList.length
  );

  const bridgeCount = $derived(
    overlapReport ? overlapReport.mutual_connections_count : sharedUsernames.size
  );
</script>

<div class="circle-view fade-in">
  <!-- Privacy Guard for Inaccessible Target Accounts -->
  {#if account.account_kind === 'monitored' && account.access_state === 'not_accessible'}
    <div class="private-guard-card">
      <div class="guard-icon-wrap">
        <Icon name="lock" size={36} color="#ef4444" strokeWidth={2} />
      </div>
      <h3 class="guard-title">PRIVATE INSTAGRAM ACCOUNT</h3>
      <p class="guard-subtitle mono">
        @{account.username}'s relationship circle is protected by Instagram privacy controls.
      </p>
      <div class="guard-reason-box mono">
        {account.access_reason || "Target account relationship lists are not accessible through the connected Instagram account (requires approved follower)."}
      </div>
      <div class="guard-help-text">
        To view @{account.username}'s full circle, connect an approved follower account via Settings or add target through an authorized owner.
      </div>
    </div>
  {:else}
    <!-- Top Identity & Summary Strip -->
    <header class="circle-header">
      <div class="account-profile-snippet">
        <div class="snippet-avatar-wrap">
          <img
            src={getAvatarUrl(account.username, account.avatar_url)}
            alt={account.username}
            class="snippet-avatar"
            loading="lazy"
            referrerpolicy="no-referrer"
            onerror={(e) => { (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(account.username); }}
          />
        </div>
        <div class="snippet-info">
          <div class="snippet-handle-row">
            <h2 class="snippet-handle">@{account.username}</h2>
            {#if account.is_verified}
              <span class="verified-check" title="Verified Account">✓</span>
            {/if}
            <span class="circle-kind-badge mono">{account.account_kind === 'owner' ? 'OWNER' : 'TARGET'}</span>
          </div>
          <span class="snippet-meta mono">
            {summary.followers.toLocaleString()} followers · {summary.following.toLocaleString()} following · {totalMembers.toLocaleString()} unique in circle
          </span>
        </div>
      </div>
    </header>

    <!-- Interactive Relationship Segment Cards -->
    <div class="segment-cards-grid">
      <!-- 1. All Connections / Unique Directory -->
      <button
        class="segment-card all {filterType === 'all' ? 'active' : ''}"
        onclick={() => handleFilterSelect('all')}
        type="button"
      >
        <div class="card-icon-pill all">🌐</div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">{totalMembers.toLocaleString()}</span>
            <span class="card-status-badge mono" style="background: rgba(168, 85, 247, 0.15); color: #c084fc;">UNIQUE</span>
          </div>
          <span class="card-title">All Directory</span>
          <span class="card-desc mono">{summary.followers} fws + {summary.following} fwg − {summary.mutual} mut = {totalMembers} unique</span>
        </div>
      </button>

      <!-- 2. Mutual Connections -->
      <button
        class="segment-card mutual {filterType === 'mutual' ? 'active' : ''}"
        onclick={() => handleFilterSelect('mutual')}
        type="button"
      >
        <div class="card-icon-pill mutual">✨</div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">{summary.mutual.toLocaleString()}</span>
            <span class="card-status-badge mutual mono">MUTUAL</span>
          </div>
          <span class="card-title">Mutual Circle</span>
          <span class="card-desc mono">Follow each other · 2-way verified</span>
        </div>
      </button>

      <!-- 3. Fans (Followers only) -->
      <button
        class="segment-card fans {filterType === 'fans' ? 'active' : ''}"
        onclick={() => handleFilterSelect('fans')}
        type="button"
      >
        <div class="card-icon-pill fans">★</div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">{summary.fans.toLocaleString()}</span>
            <span class="card-status-badge fans mono">FANS</span>
          </div>
          <span class="card-title">You Don't Follow</span>
          <span class="card-desc mono">Follow @{account.username} · Not followed back</span>
        </div>
      </button>

      <!-- 4. Not Following Back -->
      <button
        class="segment-card not-back {filterType === 'not_following_back' ? 'active' : ''}"
        onclick={() => handleFilterSelect('not_following_back')}
        type="button"
      >
        <div class="card-icon-pill not-back">⚠️</div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">{summary.not_following_back.toLocaleString()}</span>
            <span class="card-status-badge not-back mono">ASYMMETRIC</span>
          </div>
          <span class="card-title">Don't Follow Back</span>
          <span class="card-desc mono">Following · Do not follow back</span>
        </div>
      </button>

      <!-- 5. Mutual Bridge (if monitored target) -->
      {#if bridgeCount > 0 && account.account_kind === 'monitored'}
        <button
          class="segment-card bridge {filterType === 'bridge' ? 'active' : ''}"
          onclick={() => handleFilterSelect('bridge')}
          type="button"
        >
          <div class="card-icon-pill bridge">🔗</div>
          <div class="card-content">
            <div class="card-num-row">
              <span class="card-count tabular">{bridgeCount.toLocaleString()}</span>
              <span class="card-status-badge bridge mono">OVERLAP</span>
            </div>
            <span class="card-title">Mutual Bridges</span>
            <span class="card-desc mono">Shared with @{ownerAccount?.username || 'You'}</span>
          </div>
        </button>
      {/if}
    </div>

    <!-- Search & Quick Sort Ribbon -->
    <div class="search-sort-bar">
      <div class="search-box">
        <span class="search-glyph">⌕</span>
        <input
          type="text"
          placeholder="Search by username or name in {filterType.replace(/_/g, ' ')}..."
          bind:value={searchQuery}
          oninput={handleSearchInput}
          class="search-input"
        />
        {#if searchQuery}
          <button
            class="clear-search-btn"
            onclick={() => {
              searchQuery = '';
              if (account?.id) loadData(account.id, filterType);
            }}
          >
            ✕
          </button>
        {/if}
      </div>

      <div class="sort-box">
        <button
          class="custom-sort-trigger"
          onclick={() => (isSortOpen = !isSortOpen)}
          type="button"
          aria-expanded={isSortOpen}
        >
          <span class="sort-trigger-icon">{currentSort.icon}</span>
          <span class="sort-trigger-label mono">{currentSort.label}</span>
          <span class="sort-chevron-box {isSortOpen ? 'open' : ''}">
            <Icon name="chevron-down" size={12} strokeWidth={2.5} />
          </span>
        </button>

        {#if isSortOpen}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="sort-dropdown-backdrop" onclick={() => (isSortOpen = false)}></div>

          <div class="custom-sort-menu slide-down">
            {#each sortOptions as option}
              <button
                class="sort-menu-item {sortBy === option.id ? 'selected' : ''}"
                onclick={() => {
                  sortBy = option.id;
                  isSortOpen = false;
                  if (account?.id) loadData(account.id, filterType, 1);
                }}
                type="button"
              >
                <span class="item-icon">{option.icon}</span>
                <span class="item-label mono">{option.label}</span>
                {#if sortBy === option.id}
                  <span class="item-check">✓</span>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Filter Pills Strip -->
    <div class="filter-pills-strip">
      <button
        class="filter-pill {filterType === 'all' ? 'active' : ''}"
        onclick={() => handleFilterSelect('all')}
      >
        <span>ALL</span>
        <span class="pill-count mono">{totalMembers}</span>
      </button>

      <button
        class="filter-pill followers {filterType === 'followers' ? 'active' : ''}"
        onclick={() => handleFilterSelect('followers')}
      >
        <span>FOLLOWERS</span>
        <span class="pill-count mono">{summary.followers}</span>
      </button>

      <button
        class="filter-pill following {filterType === 'following' ? 'active' : ''}"
        onclick={() => handleFilterSelect('following')}
      >
        <span>FOLLOWING</span>
        <span class="pill-count mono">{summary.following}</span>
      </button>

      <button
        class="filter-pill not-back {filterType === 'not_following_back' ? 'active' : ''}"
        onclick={() => handleFilterSelect('not_following_back')}
      >
        <span>DON'T FOLLOW BACK</span>
        <span class="pill-count mono">{summary.not_following_back}</span>
      </button>

      <button
        class="filter-pill mutual {filterType === 'mutual' ? 'active' : ''}"
        onclick={() => handleFilterSelect('mutual')}
      >
        <span>MUTUAL</span>
        <span class="pill-count mono">{summary.mutual}</span>
      </button>

      <button
        class="filter-pill fans {filterType === 'fans' ? 'active' : ''}"
        onclick={() => handleFilterSelect('fans')}
      >
        <span>FANS</span>
        <span class="pill-count mono">{summary.fans}</span>
      </button>

      {#if bridgeCount > 0 && account.account_kind === 'monitored'}
        <button
          class="filter-pill bridge {filterType === 'bridge' ? 'active' : ''}"
          onclick={() => handleFilterSelect('bridge')}
        >
          <span>MUTUAL BRIDGES</span>
          <span class="pill-count mono">{bridgeCount}</span>
        </button>
      {/if}
    </div>

    <!-- Results Count Indicator & Page Size Selector -->
    <div class="results-meta-bar mono">
      <div class="meta-left">
        <span class="showing-count">
          SHOWING {startItem}–{endItem} OF {totalCount.toLocaleString()} {filterType.toUpperCase().replace(/_/g, ' ')} MEMBERS
        </span>
        {#if totalPages > 1}
          <span class="page-badge">Page {currentPage} of {totalPages}</span>
        {/if}
      </div>

      <div class="meta-right">
        <span class="per-page-label">Per page:</span>
        <div class="page-size-pills">
          {#each [25, 50, 100, 200] as size}
            <button
              class="page-size-btn {pageSize === size ? 'active' : ''}"
              onclick={() => handlePageSizeChange(size)}
              type="button"
            >
              {size}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Member Roster List -->
    <div class="members-container">
      {#if displayedPeople.length === 0 && !isLoading}
        <div class="empty-state-card mono">
          <div class="empty-icon">⌕</div>
          <div class="empty-title">NO MEMBERS FOUND</div>
          <div class="empty-sub">
            No accounts matched "{filterType.replace(/_/g, ' ')}"
            {#if searchQuery}
              with query "{searchQuery}"
            {/if}
          </div>
          {#if filterType !== 'all' || searchQuery}
            <button
              class="reset-filters-btn mono"
              onclick={() => {
                searchQuery = '';
                handleFilterSelect('all');
              }}
            >
              SHOW ALL CONNECTIONS
            </button>
          {/if}
        </div>
      {:else}
        <div class="members-list">
          {#each displayedPeople as person (person.id)}
            <PersonCard {person} onclick={() => onSelectPerson(person.id, person.username)} />
          {/each}
        </div>
      {/if}
    </div>

    <!-- Pagination Footer Navigation & Page Size Selector -->
    {#if totalCount > 0}
      <nav class="pagination-footer" aria-label="Member pagination">
        <div class="footer-per-page">
          <span class="per-page-label">Per page:</span>
          <div class="page-size-pills">
            {#each [25, 50, 100, 200] as size}
              <button
                class="page-size-btn {pageSize === size ? 'active' : ''}"
                onclick={() => handlePageSizeChange(size)}
                type="button"
              >
                {size}
              </button>
            {/each}
          </div>
        </div>

        {#if totalPages > 1}
          <div class="pagination-nav-group">
            <!-- First Page -->
            <button
              class="page-nav-btn first"
              onclick={() => goToPage(1)}
              disabled={currentPage === 1 || isLoading}
              title="First Page"
              type="button"
            >
              ⪻
            </button>

            <!-- Prev Page -->
            <button
              class="page-nav-btn prev"
              onclick={() => goToPage(currentPage - 1)}
              disabled={currentPage === 1 || isLoading}
              title="Previous Page"
              type="button"
            >
              ← Prev
            </button>

            <!-- Page Numbers -->
            <div class="page-numbers-strip">
              {#each paginationRange as p, i (i)}
                {#if p === 'ellipsis'}
                  <span class="page-ellipsis">…</span>
                {:else}
                  <button
                    class="page-number-btn {currentPage === p ? 'active' : ''}"
                    onclick={() => goToPage(p)}
                    disabled={isLoading}
                    type="button"
                  >
                    {p}
                  </button>
                {/if}
              {/each}
            </div>

            <!-- Next Page -->
            <button
              class="page-nav-btn next"
              onclick={() => goToPage(currentPage + 1)}
              disabled={currentPage === totalPages || isLoading}
              title="Next Page"
              type="button"
            >
              Next →
            </button>

            <!-- Last Page -->
            <button
              class="page-nav-btn last"
              onclick={() => goToPage(totalPages)}
              disabled={currentPage === totalPages || isLoading}
              title="Last Page"
              type="button"
            >
              ⪼
            </button>
          </div>

          <!-- Jump to Page Form -->
          <form class="pagination-jump-form" onsubmit={(e) => { e.preventDefault(); handleJumpSubmit(e); }}>
            <span class="jump-label mono">Go to:</span>
            <input
              type="number"
              min="1"
              max={totalPages}
              placeholder={currentPage.toString()}
              bind:value={jumpPageInput}
              onkeydown={(e) => { if (e.key === 'Enter') handleJumpSubmit(e); }}
              class="jump-input mono"
            />
            <button type="submit" class="jump-btn mono" disabled={!jumpPageInput || isLoading}>
              Go
            </button>
          </form>
        {/if}
      </nav>
    {/if}
  {/if}
</div>

<style>
  .circle-view {
    padding: 14px 12px 16px 12px;
    max-width: 720px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }

  /* Privacy Guard Card */
  .private-guard-card {
    background: rgba(22, 17, 36, 0.95);
    border: 1px solid rgba(239, 68, 68, 0.35);
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.5);
    border-radius: 14px;
    padding: 36px 24px;
    text-align: center;
    max-width: 540px;
    margin: 40px auto;
  }

  .guard-icon-wrap {
    color: #ef4444;
    margin-bottom: 16px;
    display: inline-flex;
    padding: 14px;
    background: rgba(239, 68, 68, 0.1);
    border-radius: 50%;
  }

  .guard-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: 0.08em;
    margin-bottom: 8px;
  }

  .guard-subtitle {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 16px;
  }

  .guard-reason-box {
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 12px 14px;
    font-size: 11px;
    color: #fca5a5;
    margin-bottom: 18px;
    line-height: 1.5;
  }

  .guard-help-text {
    font-size: 12px;
    color: var(--text-tertiary);
    line-height: 1.5;
  }

  /* Top Header */
  .circle-header {
    margin-bottom: 16px;
  }

  .account-profile-snippet {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: rgba(20, 15, 34, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
  }

  .snippet-avatar-wrap {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
  }

  .snippet-avatar {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    object-fit: cover;
    border: 1.5px solid rgba(168, 85, 247, 0.4);
  }

  .snippet-info {
    flex: 1;
    min-width: 0;
  }

  .snippet-handle-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .snippet-handle {
    font-size: 16px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .verified-check {
    color: #38bdf8;
    font-size: 12px;
    font-weight: 700;
  }

  .circle-kind-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(168, 85, 247, 0.18);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.35);
  }

  .snippet-meta {
    font-size: 11px;
    color: var(--text-tertiary);
    display: block;
    margin-top: 2px;
  }

  /* Segment Cards Grid */
  .segment-cards-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
    margin-bottom: 16px;
  }

  @media (min-width: 600px) {
    .segment-cards-grid {
      grid-template-columns: repeat(4, 1fr);
    }
  }

  .segment-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 14px 14px;
    background: rgba(22, 17, 36, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    text-align: left;
    cursor: pointer;
    transition: all 160ms cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
    overflow: hidden;
  }

  .segment-card:hover {
    background: rgba(30, 24, 48, 0.95);
    border-color: rgba(255, 255, 255, 0.16);
    transform: translateY(-1px);
  }

  .segment-card.active {
    background: rgba(32, 24, 52, 0.95);
    border-color: var(--active-border, #a855f7);
    box-shadow: 0 4px 18px var(--active-glow, rgba(168, 85, 247, 0.25));
  }

  .segment-card.all.active {
    --active-border: #94a3b8;
    --active-glow: rgba(148, 163, 184, 0.25);
  }

  .segment-card.mutual.active {
    --active-border: #10b981;
    --active-glow: rgba(16, 185, 129, 0.25);
  }

  .segment-card.fans.active {
    --active-border: #a855f7;
    --active-glow: rgba(168, 85, 247, 0.25);
  }

  .segment-card.not-back.active {
    --active-border: #f59e0b;
    --active-glow: rgba(245, 158, 11, 0.25);
  }

  .segment-card.bridge.active {
    --active-border: #fbbf24;
    --active-glow: rgba(251, 191, 36, 0.3);
  }

  .card-icon-pill {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    margin-bottom: 10px;
    background: rgba(255, 255, 255, 0.06);
  }

  .card-icon-pill.mutual { background: rgba(16, 185, 129, 0.15); color: #10b981; }
  .card-icon-pill.fans { background: rgba(168, 85, 247, 0.15); color: #a855f7; }
  .card-icon-pill.not-back { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
  .card-icon-pill.bridge { background: rgba(251, 191, 36, 0.15); color: #fbbf24; }

  .card-content {
    width: 100%;
  }

  .card-num-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px;
    margin-bottom: 4px;
  }

  .card-count {
    font-size: 22px;
    font-weight: 800;
    color: var(--text-primary);
    line-height: 1;
  }

  .card-status-badge {
    font-size: 8.5px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 4px;
  }

  .card-status-badge.mutual { background: rgba(16, 185, 129, 0.18); color: #34d399; }
  .card-status-badge.fans { background: rgba(168, 85, 247, 0.18); color: #c084fc; }
  .card-status-badge.not-back { background: rgba(245, 158, 11, 0.18); color: #fbbf24; }
  .card-status-badge.bridge { background: rgba(251, 191, 36, 0.18); color: #fde047; }

  .card-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
    display: block;
    margin-bottom: 2px;
  }

  .card-desc {
    font-size: 10.5px;
    color: var(--text-tertiary);
    line-height: 1.3;
    display: block;
  }

  /* Search & Sort */
  .search-sort-bar {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }

  .search-box {
    flex: 1;
    display: flex;
    align-items: center;
    background: rgba(20, 15, 34, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 9px;
    padding: 9px 12px;
    gap: 8px;
  }

  .search-glyph {
    color: var(--text-tertiary);
    font-size: 15px;
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
  }

  .search-input::placeholder {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 400;
    opacity: 0.65;
  }

  .clear-search-btn {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 11px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    flex-shrink: 0;
    transition: all 0.15s ease;
  }

  .clear-search-btn:hover {
    background: rgba(239, 68, 68, 0.25);
    border-color: rgba(239, 68, 68, 0.5);
    color: #ffffff;
    transform: scale(1.08);
  }

  .sort-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .custom-sort-trigger {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: rgba(22, 16, 40, 0.85);
    border: 1px solid var(--border-medium);
    border-radius: 10px;
    padding: 8px 14px;
    color: var(--text-primary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    white-space: nowrap;
  }

  .custom-sort-trigger:hover {
    background: rgba(30, 22, 54, 0.95);
    border-color: rgba(168, 85, 247, 0.5);
    box-shadow: 0 0 14px rgba(168, 85, 247, 0.2);
  }

  .sort-trigger-icon {
    font-size: 13px;
  }

  .sort-trigger-label {
    letter-spacing: 0.3px;
    font-weight: 500;
  }

  .sort-chevron-box {
    display: inline-flex;
    align-items: center;
    color: var(--text-muted);
    transition: transform 0.2s ease, color 0.2s ease;
  }

  .custom-sort-trigger:hover .sort-chevron-box {
    color: var(--text-secondary);
  }

  .sort-chevron-box.open {
    transform: rotate(180deg);
    color: #c084fc;
  }

  .sort-dropdown-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
  }

  .custom-sort-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    width: 200px;
    background: rgba(18, 12, 34, 0.96);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    padding: 6px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.75), 0 0 0 1px rgba(168, 85, 247, 0.2);
    z-index: 110;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .sort-menu-item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 12px;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
    width: 100%;
  }

  .sort-menu-item:hover {
    background: rgba(168, 85, 247, 0.16);
    color: #ffffff;
  }

  .sort-menu-item.selected {
    background: rgba(168, 85, 247, 0.24);
    color: #f3e8ff;
    font-weight: 600;
  }

  .item-icon {
    font-size: 13px;
  }

  .item-label {
    letter-spacing: 0.3px;
  }

  .item-check {
    margin-left: auto;
    color: #c084fc;
    font-weight: 700;
    font-size: 12px;
  }

  /* Filter Pills Strip */
  .filter-pills-strip {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding-bottom: 4px;
    margin-bottom: 12px;
    scrollbar-width: none;
  }

  .filter-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: rgba(20, 15, 34, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: all 120ms ease;
  }

  .filter-pill:hover {
    background: rgba(30, 24, 48, 0.85);
    border-color: rgba(255, 255, 255, 0.15);
  }

  .filter-pill.active {
    background: rgba(168, 85, 247, 0.22);
    border-color: #a855f7;
    color: #ffffff;
    box-shadow: 0 2px 10px rgba(168, 85, 247, 0.25);
  }

  .pill-count {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.09);
  }

  /* Results Meta Bar */
  .results-meta-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 10px;
    color: var(--text-tertiary);
    padding: 0 4px;
    margin-bottom: 8px;
  }



  /* Members Container */
  .members-container {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    overflow: hidden;
  }

  .members-list {
    max-height: 650px;
    overflow-y: auto;
  }

  /* Empty State */
  .empty-state-card {
    padding: 44px 20px;
    text-align: center;
    color: var(--text-tertiary);
  }

  .empty-icon {
    font-size: 28px;
    margin-bottom: 8px;
    opacity: 0.5;
  }

  .empty-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .empty-sub {
    font-size: 11px;
    margin-bottom: 16px;
  }

  .reset-filters-btn {
    padding: 8px 16px;
    background: rgba(168, 85, 247, 0.2);
    border: 1px solid #a855f7;
    border-radius: 6px;
    color: #e9d5ff;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 120ms ease;
  }

  .reset-filters-btn:hover {
    background: rgba(168, 85, 247, 0.35);
  }

  /* Results Meta Bar & Page Size Selector */
  .results-meta-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 11px;
    color: var(--text-tertiary);
    margin-bottom: 12px;
    padding: 0 4px;
    flex-wrap: wrap;
  }

  .meta-left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .showing-count {
    letter-spacing: 0.4px;
    font-weight: 500;
  }

  .page-badge {
    background: rgba(168, 85, 247, 0.12);
    border: 1px solid rgba(168, 85, 247, 0.25);
    color: #d8b4fe;
    padding: 2px 7px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
  }

  .meta-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .per-page-label {
    font-size: 10px;
    color: var(--text-muted);
  }

  .page-size-pills {
    display: flex;
    gap: 4px;
    background: rgba(18, 12, 34, 0.6);
    padding: 2px;
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
  }

  .page-size-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    font-size: 10px;
    font-weight: 600;
    padding: 3px 7px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .page-size-btn:hover {
    color: var(--text-primary);
  }

  .page-size-btn.active {
    background: rgba(168, 85, 247, 0.28);
    color: #f3e8ff;
    border: 1px solid rgba(168, 85, 247, 0.4);
  }

  /* Pagination Footer */
  .pagination-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 20px;
    padding: 12px 16px;
    background: rgba(18, 12, 34, 0.75);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--border-medium);
    border-radius: 14px;
    flex-wrap: wrap;
  }

  .footer-per-page {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .pagination-nav-group {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-wrap: wrap;
  }

  .page-nav-btn {
    background: rgba(26, 18, 48, 0.7);
    border: 1px solid var(--border-medium);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    padding: 6px 10px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .page-nav-btn:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.2);
    border-color: rgba(168, 85, 247, 0.45);
    color: #ffffff;
  }

  .page-nav-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .page-numbers-strip {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .page-number-btn {
    min-width: 30px;
    height: 30px;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: rgba(26, 18, 48, 0.5);
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .page-number-btn:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.2);
    border-color: rgba(168, 85, 247, 0.4);
    color: #ffffff;
  }

  .page-number-btn.active {
    background: rgba(168, 85, 247, 0.3);
    border: 1px solid #c084fc;
    color: #ffffff;
    box-shadow: 0 0 10px rgba(168, 85, 247, 0.35);
  }

  .page-ellipsis {
    color: var(--text-muted);
    padding: 0 4px;
    font-size: 12px;
  }

  .pagination-jump-form {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .jump-label {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .jump-input {
    width: 48px;
    background: rgba(14, 9, 28, 0.85);
    border: 1px solid var(--border-medium);
    border-radius: 6px;
    padding: 5px 8px;
    color: var(--text-primary);
    font-size: 11px;
    text-align: center;
    outline: none;
  }

  .jump-input:focus {
    border-color: #a855f7;
    box-shadow: 0 0 8px rgba(168, 85, 247, 0.25);
  }

  .jump-btn {
    background: rgba(168, 85, 247, 0.25);
    border: 1px solid rgba(168, 85, 247, 0.45);
    border-radius: 6px;
    color: #e9d5ff;
    padding: 5px 10px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .jump-btn:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.4);
    color: #ffffff;
  }

  .jump-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* ==========================================================================
     Mobile & Android Ergonomics (@media max-width: 600px & 360px)
     ========================================================================== */
  @media (max-width: 600px) {
    .account-profile-snippet {
      padding: 10px 12px;
      gap: 10px;
    }

    .snippet-avatar, .snippet-avatar-wrap {
      width: 38px;
      height: 38px;
    }

    .snippet-handle {
      font-size: 14px;
    }

    .segment-cards-grid {
      grid-template-columns: repeat(2, 1fr);
      gap: 8px;
      margin-bottom: 12px;
    }

    .segment-card {
      padding: 10px 10px;
    }

    .card-icon-pill {
      width: 26px;
      height: 26px;
      font-size: 12px;
      margin-bottom: 6px;
    }

    .card-count {
      font-size: 16px;
    }

    .card-title {
      font-size: 11px;
    }

    .filter-pills-strip {
      overflow-x: auto;
      -webkit-overflow-scrolling: touch;
      scrollbar-width: none;
      flex-wrap: nowrap;
      padding-bottom: 4px;
    }

    .results-meta-bar {
      flex-direction: column;
      align-items: flex-start;
      gap: 8px;
    }

    .meta-left, .meta-right {
      width: 100%;
      justify-content: space-between;
    }

    /* Mobile-Optimized Stacked Pagination */
    .pagination-footer {
      flex-direction: column;
      align-items: stretch;
      gap: 12px;
      padding: 12px 10px;
    }

    .footer-per-page {
      justify-content: space-between;
      width: 100%;
    }

    .pagination-nav-group {
      justify-content: center;
      width: 100%;
      flex-wrap: wrap;
      gap: 4px;
    }

    .page-nav-btn {
      padding: 5px 8px;
      font-size: 10px;
    }

    .page-number-btn {
      min-width: 28px;
      height: 28px;
      font-size: 10px;
    }

    .pagination-jump-form {
      justify-content: center;
      width: 100%;
      padding-top: 4px;
      border-top: 1px solid rgba(255, 255, 255, 0.05);
    }
  }

  @media (max-width: 480px) {
    .search-box {
      min-width: 0;
    }
    .custom-sort-trigger {
      padding: 8px 10px;
      gap: 6px;
    }
    .sort-trigger-label {
      font-size: 10px;
    }
  }

  @media (max-width: 370px) {
    .segment-cards-grid {
      grid-template-columns: 1fr;
      gap: 8px;
    }

    .search-sort-bar {
      flex-direction: column;
      gap: 6px;
    }

    .sort-box {
      width: 100%;
    }

    .custom-sort-trigger {
      width: 100%;
      justify-content: space-between;
    }

    .card-title {
      font-size: 10px;
    }

    .card-count {
      font-size: 15px;
    }
  }
</style>
