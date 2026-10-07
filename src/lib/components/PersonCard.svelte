<script lang="ts">
  import type { PersonListItem } from '../api';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';

  let { person, onclick }: { person: PersonListItem; onclick: () => void } = $props();

  const relationshipLabel = $derived(
    person.is_mutual
      ? 'Mutual'
      : person.is_follower
      ? 'Follows You'
      : person.is_following
      ? "Doesn't Follow"
      : 'Observed'
  );

  const relationshipClass = $derived(
    person.is_mutual ? 'badge-mutual' : person.is_follower ? 'badge-follower' : 'badge-following'
  );
</script>

<div
  class="person-card"
  {onclick}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { onclick(); } }}
  role="button"
  tabindex="0"
>
  <div class="avatar-col">
    <img
      src={getAvatarUrl(person.username, person.avatar_url)}
      alt={person.username}
      class="avatar"
      loading="lazy"
      referrerpolicy="no-referrer"
      onerror={(e) => {
        const target = e.currentTarget as HTMLImageElement;
        target.src = getFallbackAvatar(person.username);
      }}
    />
  </div>

  <div class="details-col">
    <div class="name-row">
      <span class="username">@{person.username}</span>
      {#if person.is_verified}
        <span class="ig-verified-badge" title="Verified">✓</span>
      {/if}
      {#if person.has_note}
        <span class="note-indicator" title="Private note attached">✎</span>
      {/if}
    </div>

    {#if person.display_name}
      <span class="display-name">{person.display_name}</span>
    {/if}

    {#if person.tags && person.tags.length > 0}
      <div class="tags-row">
        {#each person.tags as tag}
          <span class="tag-pill" style="border-color: {tag.color || '#33384a'}">{tag.name}</span>
        {/each}
      </div>
    {/if}
  </div>

  <div class="status-col">
    <span class="badge {relationshipClass} mono">{relationshipLabel}</span>
    {#if person.last_change_type}
      <span class="change-hint mono">
        {person.last_change_type === 'followed_you' ? '+ Followed' : person.last_change_type === 'unfollowed_you' ? '− Unfollowed' : ''}
      </span>
    {/if}
  </div>
</div>

<style>
  .person-card {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    background-color: var(--bg-surface);
    cursor: pointer;
    transition: background 120ms ease;
    gap: 12px;
  }

  .person-card:hover {
    background-color: var(--bg-surface-elevated);
  }

  .avatar, .avatar-placeholder {
    width: 40px;
    height: 40px;
    min-width: 40px;
    min-height: 40px;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid var(--border-subtle);
  }

  .avatar-placeholder {
    background-color: var(--bg-surface-elevated);
    border: 1px solid var(--border-strong);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 600;
  }

  .details-col {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .username {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .verified-icon {
    font-size: 10px;
    color: var(--accent-signal);
  }

  .note-indicator {
    font-size: 10px;
    color: var(--accent-positive);
  }

  .display-name {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tags-row {
    display: flex;
    gap: 4px;
    margin-top: 4px;
  }

  .tag-pill {
    font-size: 9px;
    font-family: var(--font-mono);
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid;
    color: var(--text-secondary);
  }

  .status-col {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
  }

  .badge {
    font-size: 10px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 9999px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .badge-mutual {
    background: var(--accent-positive-muted);
    color: var(--accent-positive);
    border: 1px solid rgba(45, 212, 191, 0.3);
  }

  .badge-follower {
    background: rgba(142, 147, 166, 0.12);
    color: var(--text-secondary);
    border: 1px solid var(--border-subtle);
  }

  .badge-following {
    background: var(--accent-signal-muted);
    color: var(--accent-signal);
    border: 1px solid rgba(229, 184, 76, 0.3);
  }

  .change-hint {
    font-size: 9px;
    color: var(--text-tertiary);
  }

  @media (max-width: 480px) {
    .person-card {
      padding: 10px 12px;
      gap: 10px;
    }

    .avatar, .avatar-placeholder {
      width: 38px;
      height: 38px;
      min-width: 38px;
      min-height: 38px;
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
      font-size: 11px;
    }

    .badge {
      font-size: 9px;
      padding: 2px 6px;
    }
  }

  @media (max-width: 360px) {
    .username {
      max-width: 95px;
    }

    .display-name {
      max-width: 85px;
    }
  }
</style>
