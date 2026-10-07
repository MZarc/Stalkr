<script lang="ts">
  import type { RelationshipSummary } from '../api';

  let { summary, onSelectFilter }: { summary: RelationshipSummary; onSelectFilter?: (f: string) => void } = $props();

  const total = $derived(summary.followers + summary.following);
  const mutualPct = $derived(total > 0 ? Math.round((summary.mutual / summary.followers) * 100) : 0);
  const notFollowingBackPct = $derived(total > 0 ? Math.round((summary.not_following_back / summary.following) * 100) : 0);
  const fansPct = $derived(total > 0 ? Math.round((summary.fans / summary.followers) * 100) : 0);
</script>

<div class="plain-list-container">
  <div
    class="row"
    onclick={() => onSelectFilter?.('mutual')}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { onSelectFilter?.('mutual'); } }}
    role="button"
    tabindex="0"
  >
    <div class="info">
      <div class="title-line">
        <span class="indicator mutual"></span>
        <span class="label">Mutual Connections</span>
        <span class="badge">R1</span>
      </div>
      <span class="desc">Accounts that follow you, and you follow back</span>
    </div>
    <div class="metric">
      <span class="count tabular">{summary.mutual.toLocaleString()}</span>
      <span class="pct mono">{mutualPct}% of followers</span>
    </div>
  </div>

  <div
    class="row"
    onclick={() => onSelectFilter?.('fans')}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { onSelectFilter?.('fans'); } }}
    role="button"
    tabindex="0"
  >
    <div class="info">
      <div class="title-line">
        <span class="indicator fans"></span>
        <span class="label">Fans (Followers Only)</span>
        <span class="badge">R2</span>
      </div>
      <span class="desc">People following you whom you don't follow back</span>
    </div>
    <div class="metric">
      <span class="count tabular">{summary.fans.toLocaleString()}</span>
      <span class="pct mono">{fansPct}% of followers</span>
    </div>
  </div>

  <div
    class="row"
    onclick={() => onSelectFilter?.('not_following_back')}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { onSelectFilter?.('not_following_back'); } }}
    role="button"
    tabindex="0"
  >
    <div class="info">
      <div class="title-line">
        <span class="indicator following"></span>
        <span class="label">Not Following You Back</span>
        <span class="badge">R3</span>
      </div>
      <span class="desc">Accounts you follow who do not follow back</span>
    </div>
    <div class="metric">
      <span class="count tabular">{summary.not_following_back.toLocaleString()}</span>
      <span class="pct mono">{notFollowingBackPct}% of following</span>
    </div>
  </div>
</div>

<style>
  .plain-list-container {
    background-color: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    overflow: hidden;
    margin-bottom: 20px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px;
    border-bottom: 1px solid var(--border-subtle);
    cursor: pointer;
    transition: background 150ms ease;
  }

  .row:last-child {
    border-bottom: none;
  }

  .row:hover {
    background-color: var(--bg-surface-elevated);
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title-line {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .indicator.mutual { background-color: var(--accent-positive); }
  .indicator.fans { background-color: var(--text-secondary); }
  .indicator.following { background-color: var(--accent-signal); }

  .label {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
  }

  .badge {
    font-family: var(--font-mono);
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--bg-surface-hover);
    color: var(--text-tertiary);
  }

  .desc {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .metric {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }

  .count {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .pct {
    font-size: 10px;
    color: var(--text-tertiary);
  }
</style>
