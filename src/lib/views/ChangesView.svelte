<script lang="ts">
  import { untrack } from 'svelte';
  import { api, type Account, type ChangeFeedItem } from '../api';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';
  import { globalSync } from '../syncState.svelte';
  import Icon from '../components/Icon.svelte';

  let {
    account,
    onSelectPerson,
  }: {
    account: Account;
    onSelectPerson: (personId: string, username: string) => void;
  } = $props();

  // Primary list and loading state
  let changes = $state<ChangeFeedItem[]>([]);
  let isLoading = $state(false);

  // Active filter: 'all' | 'lost' | 'gained' | 'renamed' | 'outbound'
  let filterType = $state('all');

  // Search input with debounce
  let searchQuery = $state('');
  let searchInputEl = $state<HTMLInputElement | null>(null);
  let isSearchFocused = $state(false);

  // Timeline Sorting state
  let sortBy = $state<'recent' | 'oldest' | 'username'>('recent');
  let isSortOpen = $state(false);

  const sortOptions = [
    { id: 'recent' as const, label: 'Newest First', icon: '⏱️' },
    { id: 'oldest' as const, label: 'Oldest First', icon: '⏳' },
    { id: 'username' as const, label: 'Handle (A–Z)', icon: '🔤' },
  ];

  const currentSort = $derived(sortOptions.find((o) => o.id === sortBy) || sortOptions[0]);

  // Overall metric counts across all records for the account
  let counts = $state({
    all: 0,
    lost: 0,
    gained: 0,
    renamed: 0,
    outbound: 0,
  });

  // True database pagination state
  let currentPage = $state(1);
  let pageSize = $state(25);
  let totalCount = $state(0);
  let jumpPageInput = $state('');

  // Snapshot audit inspection modal state
  let selectedAuditChange = $state<ChangeFeedItem | null>(null);

  // Account identity context
  const isOwner = $derived(account.account_kind === 'owner');

  const totalPages = $derived(Math.max(1, Math.ceil(totalCount / pageSize)));
  const pageStart = $derived(totalCount === 0 ? 0 : (currentPage - 1) * pageSize + 1);
  const pageEnd = $derived(Math.min(currentPage * pageSize, totalCount));

  // Dynamic pagination range with ellipsis
  const paginationRange = $derived.by(() => {
    const total = totalPages;
    const cur = currentPage;
    if (total <= 7) {
      return Array.from({ length: total }, (_, i) => i + 1);
    }
    const pages: (number | 'ellipsis')[] = [1];
    let leftBound = Math.max(2, cur - 1);
    let rightBound = Math.min(total - 1, cur + 1);

    if (cur <= 3) {
      rightBound = 4;
    } else if (cur >= total - 2) {
      leftBound = total - 3;
    }

    if (leftBound > 2) {
      pages.push('ellipsis');
    }

    for (let p = leftBound; p <= rightBound; p++) {
      pages.push(p);
    }

    if (rightBound < total - 1) {
      pages.push('ellipsis');
    }

    pages.push(total);
    return pages;
  });

  // Track account to avoid duplicate loads
  let loadedAccountId = $state('');

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
      // 1. Fetch overall metric counts for summary cards
      const [allC, lostC, gainedC, renC, outC] = await Promise.all([
        api.getChangesCount(accId, 'all'),
        api.getChangesCount(accId, 'lost'),
        api.getChangesCount(accId, 'gained'),
        api.getChangesCount(accId, 'renamed'),
        api.getChangesCount(accId, 'outbound'),
      ]);
      counts = {
        all: allC,
        lost: lostC,
        gained: gainedC,
        renamed: renC,
        outbound: outC,
      };

      // 2. Fetch changes with true database pagination and search
      const cleanQuery = searchQuery.trim().replace(/^@/, '');
      const q = cleanQuery.length > 0 ? cleanQuery : null;

      const filteredTotal = await api.getChangesCount(accId, activeFilter, q);
      totalCount = filteredTotal;

      const offset = (targetPage - 1) * pageSize;
      const res = await api.getChangesFeed(accId, activeFilter, q, pageSize, offset);
      changes = res;
    } catch (e) {
      console.error('Failed to load relationship changes feed:', e);
      changes = [];
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
      const listEl = document.querySelector('.changes-roster-container');
      if (listEl) {
        listEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
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
    }, 180);
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      clearSearch();
      searchInputEl?.blur();
    } else if (e.key === 'Enter') {
      clearTimeout(searchDebounce);
      if (account?.id) {
        loadData(account.id, filterType, 1);
      }
    }
  }

  function clearSearch() {
    searchQuery = '';
    clearTimeout(searchDebounce);
    if (account?.id) {
      loadData(account.id, filterType, 1);
    }
  }

  function handleFilterSelect(newFilter: string) {
    filterType = newFilter;
    if (account?.id) {
      loadData(account.id, newFilter, 1);
    }
  }

  // Sorted list based on active sortBy selection
  const displayChanges = $derived.by(() => {
    const list = [...changes];
    if (sortBy === 'oldest') {
      list.sort((a, b) => a.detected_at - b.detected_at);
    } else if (sortBy === 'username') {
      list.sort((a, b) => a.related_username.localeCompare(b.related_username));
    } else {
      list.sort((a, b) => b.detected_at - a.detected_at);
    }
    return list;
  });

  // Chronological Period Grouping with dynamic sort support
  const groupedChanges = $derived.by(() => {
    if (sortBy === 'username') {
      return [{ label: 'ALPHABETICAL (A–Z)', sub: 'Sorted by handle', items: displayChanges }];
    }
    if (sortBy === 'oldest') {
      return [{ label: 'CHRONOLOGICAL (OLDEST FIRST)', sub: 'Earliest records', items: displayChanges }];
    }

    const oneDayMs = 86400 * 1000;
    const startOfToday = new Date().setHours(0, 0, 0, 0);
    const startOfYesterday = startOfToday - oneDayMs;
    const startOfThisWeek = startOfToday - 6 * oneDayMs;

    const today: ChangeFeedItem[] = [];
    const yesterday: ChangeFeedItem[] = [];
    const thisWeek: ChangeFeedItem[] = [];
    const earlier: ChangeFeedItem[] = [];

    for (const c of displayChanges) {
      const ms = c.detected_at * 1000;
      if (ms >= startOfToday) {
        today.push(c);
      } else if (ms >= startOfYesterday) {
        yesterday.push(c);
      } else if (ms >= startOfThisWeek) {
        thisWeek.push(c);
      } else {
        earlier.push(c);
      }
    }

    const groups: { label: string; sub: string; items: ChangeFeedItem[] }[] = [];
    if (today.length > 0) groups.push({ label: 'TODAY', sub: 'Past 24 hours', items: today });
    if (yesterday.length > 0) groups.push({ label: 'YESTERDAY', sub: 'Yesterday', items: yesterday });
    if (thisWeek.length > 0) groups.push({ label: 'THIS WEEK', sub: 'Past 7 days', items: thisWeek });
    if (earlier.length > 0) groups.push({ label: 'EARLIER', sub: 'All-time history', items: earlier });
    return groups;
  });

  // Relative Time Formatter
  function formatRelativeTime(ts: number): string {
    const diffSec = Math.max(0, Math.floor(Date.now() / 1000 - ts));
    if (diffSec < 60) return 'Just now';
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
    if (diffSec < 86400 * 2) return 'Yesterday';
    if (diffSec < 86400 * 7) return `${Math.floor(diffSec / 86400)}d ago`;
    return new Date(ts * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  // Precise Localized Timestamp Formatter
  function formatExactTimestamp(ts: number): string {
    const d = new Date(ts * 1000);
    return d.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  // Context-Aware Phrasing
  function getChangeSemantic(c: ChangeFeedItem) {
    if (c.change_type === 'followed_you') {
      return {
        label: isOwner ? 'Started following you' : `Started following @${account.username}`,
        sub: 'New follower added',
        pill: 'New Follower',
        badgeSymbol: '+',
        badgeClass: 'followed',
      };
    }
    if (c.change_type === 'unfollowed_you') {
      return {
        label: isOwner ? 'Unfollowed you' : `Unfollowed @${account.username}`,
        sub: 'Removed from followers',
        pill: 'Lost Follower',
        badgeSymbol: '−',
        badgeClass: 'unfollowed',
      };
    }
    if (c.change_type === 'username_changed') {
      let desc = 'Changed their username';
      if (c.metadata_json) {
        try {
          const parsed = JSON.parse(c.metadata_json);
          if (parsed.old_username && parsed.new_username) {
            desc = `@${parsed.old_username} → @${parsed.new_username}`;
          }
        } catch {}
      }
      return {
        label: desc,
        sub: 'Username changed',
        pill: 'Renamed',
        badgeSymbol: '✎',
        badgeClass: 'rename',
      };
    }
    if (c.change_type === 'you_followed') {
      return {
        label: isOwner ? 'You started following' : `@${account.username} started following`,
        sub: 'You followed this account',
        pill: 'You Followed',
        badgeSymbol: '↗',
        badgeClass: 'outbound-add',
      };
    }
    if (c.change_type === 'you_unfollowed') {
      return {
        label: isOwner ? 'You stopped following' : `@${account.username} stopped following`,
        sub: 'You unfollowed this account',
        pill: 'You Unfollowed',
        badgeSymbol: '↘',
        badgeClass: 'outbound-remove',
      };
    }
    return {
      label: 'Relationship changed',
      sub: 'Status updated',
      pill: 'Change',
      badgeSymbol: '•',
      badgeClass: 'neutral',
    };
  }
</script>

<div class="changes-view fade-in">
  <!-- Privacy Guard Card (if monitored target is private and unauthorized) -->
  {#if account.account_kind === 'monitored' && account.access_state === 'not_accessible'}
    <div class="private-guard-card">
      <div class="guard-icon-wrap">
        <Icon name="lock" size={36} color="#ef4444" strokeWidth={2} />
      </div>
      <h2 class="guard-title">No Access to Changes</h2>
      <p class="guard-desc">
        @{account.username} is a private account. To track their changes, you need to be an approved follower.
      </p>
      <div class="guard-badge mono">STATUS: {account.access_state.toUpperCase().replace(/_/g, ' ')}</div>
    </div>
  {:else}
    <!-- Top Account Profile Header -->
    <div class="account-profile-snippet">
      <div class="snippet-avatar-wrap">
        <img
          src={getAvatarUrl(account.username, account.avatar_url)}
          alt={account.username}
          class="snippet-avatar"
          onerror={(e) => {
            (e.currentTarget as HTMLImageElement).src = getFallbackAvatar(account.username);
          }}
        />
      </div>

      <div class="snippet-info">
        <div class="snippet-handle-row">
          <h2 class="snippet-handle">@{account.username}</h2>
          {#if account.is_verified}
            <span class="verified-check" title="Verified Account">✓</span>
          {/if}
          <span class="kind-badge mono">{account.account_kind === 'owner' ? 'YOUR ACCOUNT' : 'TRACKED'}</span>
        </div>
        <p class="snippet-desc mono">
          {#if isOwner}
            Your account · Full change history
          {:else}
            Tracked account · Change history
          {/if}
        </p>
      </div>
    </div>

    <!-- Summary Cards -->
    <div class="segment-cards-grid">
      <!-- 1. Lost Followers -->
      <button
        class="segment-card not-back {filterType === 'lost' ? 'active' : ''}"
        onclick={() => handleFilterSelect(filterType === 'lost' ? 'all' : 'lost')}
        type="button"
      >
        <div class="card-icon-pill not-back">
          <Icon name="user-minus" size={18} color="#f87171" strokeWidth={2.4} />
        </div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">{counts.lost.toLocaleString()}</span>
            <span class="card-status-badge not-back mono">LOST</span>
          </div>
          <span class="card-title">Unfollowed</span>
          <span class="card-desc mono">Stopped following you</span>
        </div>
      </button>

      <!-- 2. New Followers -->
      <button
        class="segment-card fans {filterType === 'gained' ? 'active' : ''}"
        onclick={() => handleFilterSelect(filterType === 'gained' ? 'all' : 'gained')}
        type="button"
      >
        <div class="card-icon-pill fans">
          <Icon name="user-plus" size={18} color="#34d399" strokeWidth={2.4} />
        </div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">{counts.gained.toLocaleString()}</span>
            <span class="card-status-badge fans mono">NEW</span>
          </div>
          <span class="card-title">New Followers</span>
          <span class="card-desc mono">Started following you</span>
        </div>
      </button>

      <!-- 3. Net Change -->
      <div class="segment-card bridge {counts.gained - counts.lost >= 0 ? 'pos' : 'neg'}">
        <div class="card-icon-pill bridge">
          <Icon name={counts.gained - counts.lost >= 0 ? 'trend-up' : 'trend-down'} size={18} color="#60a5fa" strokeWidth={2.4} />
        </div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">
              {counts.gained - counts.lost >= 0 ? '+' : ''}{(counts.gained - counts.lost).toLocaleString()}
            </span>
            <span class="card-status-badge bridge mono">NET</span>
          </div>
          <span class="card-title">Net Change</span>
          <span class="card-desc mono">Gained vs lost</span>
        </div>
      </div>

      <!-- 4. Renamed -->
      <button
        class="segment-card mutual {filterType === 'renamed' ? 'active' : ''}"
        onclick={() => handleFilterSelect(filterType === 'renamed' ? 'all' : 'renamed')}
        type="button"
      >
        <div class="card-icon-pill mutual">
          <Icon name="rename" size={18} color="#c084fc" strokeWidth={2.4} />
        </div>
        <div class="card-content">
          <div class="card-num-row">
            <span class="card-count tabular">{counts.renamed.toLocaleString()}</span>
            <span class="card-status-badge mutual mono">RENAMED</span>
          </div>
          <span class="card-title">Renamed</span>
          <span class="card-desc mono">Changed username</span>
        </div>
      </button>
    </div>

    <!-- Search + Sort Bar -->
    <div class="search-sort-bar">
      <div class="search-box {isSearchFocused ? 'focused' : ''}">
        <div class="search-icon-wrap">
          {#if isLoading && searchQuery.trim().length > 0}
            <div class="search-spinner" title="Searching..."></div>
          {:else}
            <Icon name="search" size={15} strokeWidth={2.4} class="search-svg {searchQuery.trim().length > 0 ? 'active' : ''}" />
          {/if}
        </div>

        <input
          bind:this={searchInputEl}
          type="text"
          placeholder="Search changes..."
          bind:value={searchQuery}
          oninput={handleSearchInput}
          onkeydown={handleSearchKeydown}
          onfocus={() => (isSearchFocused = true)}
          onblur={() => (isSearchFocused = false)}
          class="search-input"
          spellcheck="false"
          autocomplete="off"
        />

        {#if searchQuery.trim().length > 0}
          <button
            class="clear-search-btn"
            onclick={clearSearch}
            title="Clear search"
            type="button"
            aria-label="Clear search"
          >
            <Icon name="close" size={12} strokeWidth={2.5} />
          </button>
        {/if}
      </div>

      <div class="sort-box">
        <button
          class="custom-sort-trigger"
          onclick={() => (isSortOpen = !isSortOpen)}
          type="button"
          aria-expanded={isSortOpen}
          title="Sort order"
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

    <!-- Active Search Filter Chip -->
    {#if searchQuery.trim().length > 0}
      <div class="active-query-banner fade-in">
        <div class="query-chip">
          <span class="chip-label mono">SEARCHING</span>
          <span class="chip-query">"{searchQuery.trim()}"</span>
          <button class="chip-clear-btn" onclick={clearSearch} title="Clear search">
            <span>Clear</span>
            <span class="chip-x">✕</span>
          </button>
        </div>
      </div>
    {/if}

    <!-- Filter Pills -->
    <div class="filter-pills-strip">
      <button
        class="filter-pill {filterType === 'all' ? 'active' : ''}"
        onclick={() => handleFilterSelect('all')}
        type="button"
      >
        <span>All</span>
        <span class="pill-count mono">{counts.all}</span>
      </button>

      <button
        class="filter-pill not-back {filterType === 'lost' ? 'active' : ''}"
        onclick={() => handleFilterSelect('lost')}
        type="button"
      >
        <span>Unfollowed</span>
        <span class="pill-count mono">{counts.lost}</span>
      </button>

      <button
        class="filter-pill fans {filterType === 'gained' ? 'active' : ''}"
        onclick={() => handleFilterSelect('gained')}
        type="button"
      >
        <span>New</span>
        <span class="pill-count mono">{counts.gained}</span>
      </button>

      <button
        class="filter-pill mutual {filterType === 'renamed' ? 'active' : ''}"
        onclick={() => handleFilterSelect('renamed')}
        type="button"
      >
        <span>Renamed</span>
        <span class="pill-count mono">{counts.renamed}</span>
      </button>

      <button
        class="filter-pill bridge {filterType === 'outbound' ? 'active' : ''}"
        onclick={() => handleFilterSelect('outbound')}
        type="button"
      >
        <span>By You</span>
        <span class="pill-count mono">{counts.outbound}</span>
      </button>
    </div>

    <!-- Results Meta Bar -->
    <div class="results-meta-bar">
      <div class="meta-left">
        {#if isLoading}
          <span class="loading-tag mono">Loading changes...</span>
        {:else if totalCount > 0}
          <span class="showing-count mono">
            {pageStart}–{pageEnd} of {totalCount.toLocaleString()}
          </span>
          {#if totalPages > 1}
            <span class="page-badge mono">Page {currentPage} / {totalPages}</span>
          {/if}
        {/if}
      </div>

      <div class="meta-right">
        <span class="per-page-label">Per page:</span>
        <div class="page-size-pills">
          {#each [25, 50, 100] as size}
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

    <!-- Changes List -->
    <div class="changes-roster-container">
      {#if changes.length === 0 && !isLoading}
        <div class="empty-state-card mono">
          <div class="empty-icon">⌕</div>
          <div class="empty-title">No changes found</div>
          <div class="empty-sub">
            {#if filterType !== 'all' || searchQuery}
              Nothing matched your filter{searchQuery ? ` for "${searchQuery}"` : ''}.
            {:else}
              Sync your account to start tracking follower changes.
            {/if}
          </div>
          {#if filterType !== 'all' || searchQuery}
            <button
              class="reset-filters-btn mono"
              onclick={() => {
                searchQuery = '';
                handleFilterSelect('all');
              }}
              type="button"
            >
              Show all changes
            </button>
          {/if}
        </div>
      {:else}
        <div class="changes-list">
          {#each groupedChanges as group (group.label)}
            <div class="timeline-period-block">
              <!-- Period Header -->
              <div class="period-divider">
                <div class="period-left">
                  <span class="period-glyph">◆</span>
                  <span class="period-label mono">{group.label}</span>
                  <span class="period-sub mono">· {group.sub}</span>
                </div>
                <span class="period-count mono">{group.items.length} {group.items.length === 1 ? 'event' : 'events'}</span>
              </div>

              <!-- Change Rows -->
              {#each group.items as c (c.id)}
                {@const meta = getChangeSemantic(c)}
                {@const avatarSrc = getAvatarUrl(c.related_username, c.avatar_url)}
                <div
                  class="change-row {meta.badgeClass}"
                  onclick={() => onSelectPerson(c.person_id, c.related_username)}
                  onkeydown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') onSelectPerson(c.person_id, c.related_username);
                  }}
                  role="button"
                  tabindex="0"
                >
                  <!-- Avatar -->
                  <div class="avatar-col">
                    <div class="avatar-wrap">
                      <img
                        src={avatarSrc}
                        alt={c.related_username}
                        class="avatar"
                        loading="lazy"
                        referrerpolicy="no-referrer"
                        onerror={(e) => {
                          const target = e.currentTarget as HTMLImageElement;
                          target.src = getFallbackAvatar(c.related_username);
                        }}
                      />
                      <span class="floating-badge {meta.badgeClass} mono" title={meta.pill}>
                        {meta.badgeSymbol}
                      </span>
                    </div>
                  </div>

                  <!-- Details -->
                  <div class="details-col">
                    <div class="name-row">
                      <span class="username">@{c.related_username}</span>
                      {#if c.display_name}
                        <span class="display-name">· {c.display_name}</span>
                      {/if}
                      <span class="time-ago mono" title={formatExactTimestamp(c.detected_at)}>
                        {formatRelativeTime(c.detected_at)}
                      </span>
                    </div>

                    <div class="semantic-row">
                      <span class="action-pill {meta.badgeClass} mono">{meta.pill}</span>
                      <span class="change-label {meta.badgeClass}">
                        {meta.label}
                      </span>
                    </div>
                  </div>

                  <!-- Tap indicator -->
                  <div class="action-col">
                    <span class="chevron-arrow">›</span>
                  </div>
                </div>
              {/each}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Pagination Footer -->
    {#if totalCount > 0}
      <nav class="pagination-footer" aria-label="Changes pagination">
        <div class="footer-per-page">
          <span class="per-page-label">Per page:</span>
          <div class="page-size-pills">
            {#each [25, 50, 100] as size}
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
            <button
              class="page-nav-btn first"
              onclick={() => goToPage(1)}
              disabled={currentPage === 1 || isLoading}
              title="First Page"
              type="button"
            >
              ⪻
            </button>

            <button
              class="page-nav-btn prev"
              onclick={() => goToPage(currentPage - 1)}
              disabled={currentPage === 1 || isLoading}
              title="Previous Page"
              type="button"
            >
              ← Prev
            </button>

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

            <button
              class="page-nav-btn next"
              onclick={() => goToPage(currentPage + 1)}
              disabled={currentPage === totalPages || isLoading}
              title="Next Page"
              type="button"
            >
              Next →
            </button>

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

    <!-- Change Detail Modal -->
    {#if selectedAuditChange}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal-backdrop" onclick={() => (selectedAuditChange = null)}>
        <div class="modal-card slide-down" onclick={(e) => e.stopPropagation()}>
          <div class="modal-header">
            <div class="modal-title-row">
              <span class="modal-glyph">📋</span>
              <span class="modal-title mono">Change Detail</span>
            </div>
            <button class="modal-close-btn" onclick={() => (selectedAuditChange = null)} type="button">✕</button>
          </div>

          <div class="modal-body mono">
            <div class="audit-row">
              <span class="audit-key">Account:</span>
              <span class="audit-val">@{account.username}</span>
            </div>
            <div class="audit-row">
              <span class="audit-key">Person:</span>
              <span class="audit-val highlight">@{selectedAuditChange.related_username}</span>
            </div>
            <div class="audit-row">
              <span class="audit-key">Event:</span>
              <span class="audit-val">{getChangeSemantic(selectedAuditChange).pill}</span>
            </div>
            <div class="audit-row">
              <span class="audit-key">Detected:</span>
              <span class="audit-val">{formatExactTimestamp(selectedAuditChange.detected_at)}</span>
            </div>
            {#if selectedAuditChange.metadata_json}
              <div class="audit-row metadata">
                <span class="audit-key">Details:</span>
                <pre class="audit-pre">{selectedAuditChange.metadata_json}</pre>
              </div>
            {/if}
          </div>

          <div class="modal-actions">
            <button
              class="primary-modal-btn mono"
              onclick={() => {
                const target = selectedAuditChange;
                selectedAuditChange = null;
                if (target) onSelectPerson(target.person_id, target.related_username);
              }}
              type="button"
            >
              View Profile →
            </button>
            <button class="secondary-modal-btn mono" onclick={() => (selectedAuditChange = null)} type="button">
              Close
            </button>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .changes-view {
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
    background: rgba(239, 68, 68, 0.12);
    border-radius: 50%;
  }

  .guard-title {
    font-size: 14px;
    font-weight: 700;
    color: #f87171;
    margin-bottom: 8px;
    letter-spacing: 0.5px;
  }

  .guard-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: 20px;
  }

  .guard-badge {
    display: inline-block;
    font-size: 10px;
    background: rgba(239, 68, 68, 0.15);
    color: #fca5a5;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  /* Account Profile Snippet (Matching PeopleView circle-header) */
  .account-profile-snippet {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: rgba(20, 15, 34, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    margin-bottom: 16px;
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
    flex-wrap: wrap;
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

  .kind-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(168, 85, 247, 0.18);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.35);
  }

  .audit-badge {
    font-size: 9px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: var(--text-secondary);
  }

  .snippet-desc {
    font-size: 11px;
    color: var(--text-tertiary);
    margin: 2px 0 0 0;
  }

  /* Header Tools: Export */
  .header-tools {
    flex-shrink: 0;
  }

  .export-dropdown-wrapper {
    position: relative;
  }

  .export-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: rgba(26, 18, 48, 0.7);
    border: 1px solid var(--border-medium);
    border-radius: 8px;
    padding: 6px 12px;
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .export-btn:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.2);
    border-color: rgba(168, 85, 247, 0.45);
    color: #ffffff;
  }

  .export-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .chevron {
    font-size: 10px;
    transition: transform 0.2s ease;
  }

  .chevron.open {
    transform: rotate(180deg);
  }

  .dropdown-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }

  .export-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 50;
    width: 220px;
    background: rgba(20, 14, 38, 0.95);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(168, 85, 247, 0.3);
    border-radius: 10px;
    padding: 6px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .export-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 6px;
    background: transparent;
    border: none;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .export-item:hover {
    background: rgba(168, 85, 247, 0.18);
  }

  .export-icon {
    font-size: 16px;
  }

  .export-text {
    display: flex;
    flex-direction: column;
  }

  .export-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .export-sub {
    font-size: 9px;
    color: var(--text-tertiary);
  }

  /* Segment Cards Grid (Matching PeopleView.svelte exactly) */
  .segment-cards-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    margin-bottom: 16px;
  }

  @media (max-width: 640px) {
    .segment-cards-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .segment-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    background: rgba(20, 15, 34, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    cursor: pointer;
    text-align: left;
    transition: all 150ms ease;
  }

  .segment-card:hover {
    background: rgba(30, 24, 48, 0.85);
    border-color: rgba(255, 255, 255, 0.15);
    transform: translateY(-1px);
  }

  .segment-card.active {
    border-color: #a855f7;
    background: rgba(168, 85, 247, 0.14);
    box-shadow: 0 4px 16px rgba(168, 85, 247, 0.2);
  }

  .card-icon-pill {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    flex-shrink: 0;
  }

  .card-icon-pill.not-back {
    background: rgba(239, 68, 68, 0.15);
  }

  .card-icon-pill.fans {
    background: rgba(16, 185, 129, 0.15);
  }

  .card-icon-pill.bridge {
    background: rgba(59, 130, 246, 0.15);
  }

  .card-icon-pill.mutual {
    background: rgba(168, 85, 247, 0.15);
  }

  .card-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .card-num-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .card-count {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .card-status-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 4px;
    letter-spacing: 0.4px;
  }

  .card-status-badge.not-back {
    background: rgba(239, 68, 68, 0.18);
    color: #f87171;
  }

  .card-status-badge.fans {
    background: rgba(16, 185, 129, 0.18);
    color: #34d399;
  }

  .card-status-badge.bridge {
    background: rgba(59, 130, 246, 0.18);
    color: #60a5fa;
  }

  .card-status-badge.mutual {
    background: rgba(168, 85, 247, 0.18);
    color: #c084fc;
  }

  .card-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .card-desc {
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Search & Sort Bar */
  .search-sort-bar {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 12px;
    width: 100%;
    box-sizing: border-box;
  }

  .search-box {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    background: rgba(20, 15, 34, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 0 10px 0 12px;
    height: 42px;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    position: relative;
    box-sizing: border-box;
  }

  .search-box:focus-within,
  .search-box.focused {
    background: rgba(26, 18, 48, 0.95);
    border-color: #a855f7;
    box-shadow: 0 0 16px rgba(168, 85, 247, 0.25), 0 0 0 1px rgba(168, 85, 247, 0.3);
  }

  .search-icon-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-right: 8px;
    flex-shrink: 0;
  }

  .search-svg {
    color: var(--text-muted);
    transition: color 0.15s ease, transform 0.15s ease;
  }

  .search-box:focus-within .search-svg,
  .search-svg.active {
    color: #c084fc;
    transform: scale(1.05);
  }

  .search-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(168, 85, 247, 0.25);
    border-top-color: #c084fc;
    border-radius: 50%;
    animation: spinSearch 0.6s linear infinite;
  }

  @keyframes spinSearch {
    to { transform: rotate(360deg); }
  }

  .search-input {
    flex: 1;
    min-width: 0;
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    outline: none;
    padding-right: 6px;
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
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
    flex-shrink: 0;
    padding: 0;
    margin-left: 4px;
  }

  .clear-search-btn:hover {
    background: rgba(239, 68, 68, 0.25);
    border-color: rgba(239, 68, 68, 0.5);
    color: #ffffff;
    transform: scale(1.08);
  }

  /* Custom Sort Dropdown */
  .sort-box {
    position: relative;
    flex-shrink: 0;
  }

  .custom-sort-trigger {
    height: 42px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    background: rgba(20, 15, 34, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.16s cubic-bezier(0.16, 1, 0.3, 1);
    user-select: none;
    white-space: nowrap;
  }

  .custom-sort-trigger:hover {
    background: rgba(30, 22, 54, 0.95);
    border-color: rgba(168, 85, 247, 0.5);
    box-shadow: 0 0 14px rgba(168, 85, 247, 0.2);
    color: #ffffff;
  }

  .sort-trigger-icon {
    font-size: 13px;
  }

  .sort-trigger-label {
    letter-spacing: 0.3px;
    font-weight: 600;
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
    width: 190px;
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

  /* Active Query Strip Banner */
  .active-query-banner {
    margin-bottom: 12px;
  }

  .query-chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    background: rgba(168, 85, 247, 0.14);
    border: 1px solid rgba(168, 85, 247, 0.3);
    border-radius: 20px;
    font-size: 11px;
  }

  .chip-label {
    font-size: 9px;
    font-weight: 700;
    color: #c084fc;
    letter-spacing: 0.5px;
  }

  .chip-query {
    font-weight: 600;
    color: #ffffff;
  }

  .chip-clear-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    padding: 2px 7px;
    color: #e9d5ff;
    font-size: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .chip-clear-btn:hover {
    background: rgba(239, 68, 68, 0.25);
    border-color: rgba(239, 68, 68, 0.45);
    color: #ffffff;
  }

  .chip-x {
    font-size: 9px;
    font-weight: 700;
  }

  /* Filter Pills Strip (Matching PeopleView.svelte) */
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

  .loading-tag {
    color: #c084fc;
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

  /* Changes Roster Container (Matching .members-container in PeopleView) */
  .changes-roster-container {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    overflow: hidden;
  }

  .changes-list {
    display: flex;
    flex-direction: column;
  }

  .timeline-period-block {
    display: flex;
    flex-direction: column;
  }

  .period-divider {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    background: rgba(14, 10, 24, 0.7);
    border-bottom: 1px solid var(--border-subtle);
  }

  .period-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .period-glyph {
    color: #a855f7;
    font-size: 8px;
  }

  .period-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.6px;
    color: var(--text-secondary);
  }

  .period-sub {
    font-size: 10px;
    color: var(--text-muted);
  }

  .period-count {
    font-size: 10px;
    color: var(--text-tertiary);
  }

  /* Change Row (Matching PersonCard in PeopleView) */
  .change-row {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    background-color: var(--bg-surface);
    cursor: pointer;
    transition: background 120ms ease;
    gap: 12px;
    position: relative;
  }

  .change-row:hover {
    background-color: var(--bg-surface-elevated);
  }

  .change-row:last-child {
    border-bottom: none;
  }

  /* Avatar & Floating Badge */
  .avatar-col {
    flex-shrink: 0;
  }

  .avatar-wrap {
    position: relative;
    width: 40px;
    height: 40px;
  }

  .avatar {
    width: 40px;
    height: 40px;
    min-width: 40px;
    min-height: 40px;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface-elevated);
  }

  .floating-badge {
    position: absolute;
    bottom: -2px;
    right: -2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 800;
    border: 2px solid #0d0915;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
  }

  .floating-badge.unfollowed {
    background: #ef4444;
    color: #ffffff;
  }

  .floating-badge.followed {
    background: #10b981;
    color: #ffffff;
  }

  .floating-badge.rename {
    background: #a855f7;
    color: #ffffff;
  }

  .floating-badge.outbound-add {
    background: #3b82f6;
    color: #ffffff;
  }

  .floating-badge.outbound-remove {
    background: #64748b;
    color: #ffffff;
  }

  /* Details Column */
  .details-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .name-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .username {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .display-name {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .time-ago {
    font-size: 10px;
    color: var(--text-muted);
    margin-left: auto;
  }

  .semantic-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
  }

  .action-pill {
    padding: 2px 7px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.3px;
    flex-shrink: 0;
  }

  .action-pill.unfollowed {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.25);
  }

  .action-pill.followed {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.25);
  }

  .action-pill.rename {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.25);
  }

  .action-pill.outbound-add,
  .action-pill.outbound-remove {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.25);
  }

  .change-label {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Action Right Column */
  .action-col {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding-left: 4px;
  }

  .chevron-arrow {
    font-size: 18px;
    line-height: 1;
    color: var(--text-muted);
    transition: transform 0.15s ease, color 0.15s ease;
  }

  .change-row:hover .chevron-arrow {
    color: #a855f7;
    transform: translateX(2px);
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

  .empty-hint {
    font-size: 11px;
    color: var(--text-muted);
    max-width: 440px;
    margin: 0 auto;
    line-height: 1.5;
  }

  /* Pagination Footer (Matching PeopleView.svelte) */
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
    color: var(--text-muted);
  }

  .jump-input {
    width: 48px;
    height: 30px;
    background: rgba(18, 12, 34, 0.8);
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    color: var(--text-primary);
    text-align: center;
    font-size: 11px;
    outline: none;
  }

  .jump-input:focus {
    border-color: #a855f7;
  }

  .jump-btn {
    height: 30px;
    padding: 0 10px;
    background: rgba(168, 85, 247, 0.25);
    border: 1px solid rgba(168, 85, 247, 0.4);
    border-radius: 7px;
    color: #e9d5ff;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .jump-btn:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.45);
    color: #ffffff;
  }

  .jump-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  /* Snapshot Audit Inspection Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 20px;
  }

  .modal-card {
    background: rgba(22, 16, 42, 0.98);
    border: 1px solid rgba(168, 85, 247, 0.4);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.8), 0 0 30px rgba(168, 85, 247, 0.2);
    border-radius: 16px;
    width: 100%;
    max-width: 520px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 12px;
  }

  .modal-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .modal-glyph {
    font-size: 16px;
  }

  .modal-title {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: #f3e8ff;
  }

  .modal-close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 14px;
    cursor: pointer;
  }

  .modal-close-btn:hover {
    color: #ffffff;
  }

  .modal-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 11px;
    background: rgba(14, 10, 26, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    padding: 14px;
  }

  .audit-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
  }

  .audit-key {
    color: var(--text-muted);
    font-size: 10px;
  }

  .audit-val {
    color: var(--text-primary);
    text-align: right;
  }

  .audit-val.highlight {
    color: #c084fc;
    font-weight: 700;
  }

  .audit-val.verified {
    color: #34d399;
    font-weight: 700;
  }

  .audit-val.code {
    background: rgba(255, 255, 255, 0.06);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
  }

  .audit-row.metadata {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .audit-pre {
    margin: 0;
    width: 100%;
    background: rgba(0, 0, 0, 0.4);
    padding: 8px;
    border-radius: 6px;
    color: #d8b4fe;
    font-size: 10px;
    overflow-x: auto;
  }

  .modal-actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }

  .primary-modal-btn {
    padding: 8px 16px;
    background: #a855f7;
    border: none;
    border-radius: 8px;
    color: #ffffff;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .primary-modal-btn:hover {
    background: #9333ea;
  }

  .secondary-modal-btn {
    padding: 8px 14px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--border-medium);
    border-radius: 8px;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .secondary-modal-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    color: #ffffff;
  }

  /* ==========================================================================
     Mobile & Android Ergonomics (@media max-width: 600px & 380px)
     ========================================================================== */
  @media (max-width: 600px) {
    .account-profile-snippet {
      padding: 10px 12px;
      gap: 10px;
      margin-bottom: 12px;
    }

    .snippet-avatar, .snippet-avatar-wrap {
      width: 38px;
      height: 38px;
    }

    .snippet-handle {
      font-size: 14px;
    }

    .snippet-desc {
      font-size: 10px;
    }

    .export-btn {
      padding: 5px 8px;
      font-size: 10px;
    }

    .export-menu {
      width: min(200px, calc(100vw - 24px));
      right: 0;
    }

    .segment-cards-grid {
      grid-template-columns: repeat(2, 1fr);
      gap: 8px;
      margin-bottom: 12px;
    }

    .segment-card {
      padding: 10px 10px;
      gap: 8px;
    }

    .card-icon-pill {
      width: 30px;
      height: 30px;
      font-size: 14px;
    }

    .card-count {
      font-size: 16px;
    }

    .card-title {
      font-size: 11px;
    }

    .card-desc {
      font-size: 9.5px;
    }

    .results-meta-bar {
      flex-direction: column;
      align-items: flex-start;
      gap: 8px;
    }

    .custom-sort-trigger {
      padding: 0 10px;
      gap: 6px;
    }

    .meta-left {
      width: 100%;
      justify-content: space-between;
    }

    .meta-right {
      width: 100%;
      justify-content: space-between;
    }

    .change-row {
      padding: 10px 10px;
      gap: 10px;
    }

    .username {
      font-size: 12.5px;
      max-width: 130px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .display-name {
      max-width: 110px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .change-phrase {
      font-size: 11px;
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

    /* Modal Mobile Adjustments */
    .modal-backdrop {
      padding: max(16px, env(safe-area-inset-top, 0px)) 10px max(16px, env(safe-area-inset-bottom, 0px)) 10px;
    }

    .modal-card {
      padding: 16px;
      max-height: 85vh;
      overflow-y: auto;
      border-radius: 14px;
    }

    .modal-actions {
      flex-direction: column-reverse;
      gap: 8px;
    }

    .primary-modal-btn, .secondary-modal-btn {
      width: 100%;
      text-align: center;
      padding: 10px;
    }
  }

  @media (max-width: 440px) {
    .sort-trigger-label {
      display: none;
    }

    .search-box {
      padding: 0 9px;
    }

    .search-icon-wrap {
      margin-right: 6px;
    }
  }

  @media (max-width: 360px) {
    .username {
      max-width: 100px;
    }

    .card-title {
      font-size: 10px;
    }

    .card-count {
      font-size: 15px;
    }

    .page-nav-btn span {
      display: none;
    }
  }
</style>
