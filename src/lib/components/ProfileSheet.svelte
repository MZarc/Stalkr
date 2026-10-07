<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type NoteDto, type PersonListItem } from '../api';
  import { getAvatarUrl, getFallbackAvatar } from '../avatar';

  let {
    person,
    accountId,
    onClose,
  }: {
    person: PersonListItem;
    accountId: string;
    onClose: () => void;
  } = $props();

  let noteText = $state('');
  let isSavingNote = $state(false);
  let noteSaveSuccess = $state(false);
  let initialNoteLoaded = $state(false);

  async function loadNote() {
    try {
      const note = await api.getNote(accountId, person.id);
      noteText = note ? note.content : '';
    } catch (e) {
      console.error('Failed to load note:', e);
      noteText = '';
    } finally {
      initialNoteLoaded = true;
    }
  }

  $effect(() => {
    if (person?.id && accountId) {
      loadNote();
    }
  });

  async function handleSaveNote() {
    isSavingNote = true;
    try {
      await api.saveNote(accountId, person.id, noteText);
      noteSaveSuccess = true;
      setTimeout(() => {
        noteSaveSuccess = false;
      }, 2000);
    } catch (e) {
      alert('Failed to encrypt and save note: ' + e);
    } finally {
      isSavingNote = false;
    }
  }

  const firstSeenDate = $derived(
    new Date(person.last_seen_at * 1000).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  );
</script>

<div
  class="overlay"
  onclick={onClose}
  onkeydown={(e) => { if (e.key === 'Escape') onClose(); }}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="sheet" onclick={(e) => e.stopPropagation()} role="document">
    <div class="sheet-handle"></div>

    <div class="header">
      <div class="profile-info">
        <img
          src={getAvatarUrl(person.username, person.avatar_url)}
          alt={person.username}
          class="avatar-large"
          loading="lazy"
          referrerpolicy="no-referrer"
          onerror={(e) => {
            const target = e.currentTarget as HTMLImageElement;
            target.src = getFallbackAvatar(person.username);
          }}
        />
        <div>
          <div class="title-row">
            <span class="user-handle">@{person.username}</span>
            {#if person.is_verified}
              <span class="ig-verified-badge" title="Verified">✓</span>
            {/if}
          </div>
          {#if person.display_name}
            <div class="user-sub">{person.display_name}</div>
          {/if}
        </div>
      </div>
      <button class="close-btn" onclick={onClose}>✕</button>
    </div>

    <!-- Relationship Matrix (Fixed Clean Mobile Layout) -->
    <div class="relationship-matrix-wrapper">
      <div class="matrix-cards-row">
        <!-- Card 1: They Follow You -->
        <div class="matrix-dual-card {person.is_follower ? 'positive' : 'negative'}">
          <div class="matrix-card-head">
            <span class="matrix-icon">{person.is_follower ? '✓' : '✕'}</span>
            <span class="matrix-label mono">THEY FOLLOW YOU</span>
          </div>
          <div class="matrix-state-pill mono {person.is_follower ? 'yes' : 'no'}">
            {person.is_follower ? 'FOLLOWING' : 'NOT FOLLOWING'}
          </div>
        </div>

        <!-- Card 2: You Follow Them -->
        <div class="matrix-dual-card {person.is_following ? 'positive' : 'negative'}">
          <div class="matrix-card-head">
            <span class="matrix-icon">{person.is_following ? '✓' : '✕'}</span>
            <span class="matrix-label mono">YOU FOLLOW THEM</span>
          </div>
          <div class="matrix-state-pill mono {person.is_following ? 'yes' : 'no'}">
            {person.is_following ? 'FOLLOWING' : 'NOT FOLLOWING'}
          </div>
        </div>
      </div>

      <!-- Mutual Connection Banner -->
      <div class="matrix-mutual-banner {person.is_mutual ? 'is-mutual' : 'is-asymmetric'}">
        <div class="mutual-banner-left">
          <span class="mutual-badge-icon">{person.is_mutual ? '✦' : '↷'}</span>
          <div class="mutual-text-col">
            <span class="mutual-banner-title mono">
              {person.is_mutual ? 'MUTUAL CONNECTION' : 'ONE-WAY CONNECTION'}
            </span>
            <span class="mutual-banner-sub">
              {person.is_mutual
                ? 'Both accounts follow each other'
                : person.is_follower
                  ? 'They follow you, but you do not follow back'
                  : 'You follow them, but they do not follow back'}
            </span>
          </div>
        </div>
        <span class="mutual-status-tag mono {person.is_mutual ? 'confirmed' : 'oneway'}">
          {person.is_mutual ? 'CONFIRMED' : 'ASYMMETRIC'}
        </span>
      </div>
    </div>

    <!-- Metadata Timeline -->
    <div class="meta-section">
      <div class="meta-row">
        <span class="meta-label">Observation Record</span>
        <span class="meta-val mono">Observed on {firstSeenDate}</span>
      </div>
      {#if person.instagram_user_id}
        <div class="meta-row">
          <span class="meta-label">Persistent Identity ID</span>
          <span class="meta-val mono">{person.instagram_user_id}</span>
        </div>
      {/if}
    </div>

    <!-- Encrypted Note Section (Key B) -->
    <div class="note-section">
      <div class="note-header">
        <div class="note-title">
          <span>PRIVATE NOTE</span>
          <span class="badge-enc mono">AES-256-GCM ENCRYPTED AT REST</span>
        </div>
        {#if noteSaveSuccess}
          <span class="save-status mono">✓ Saved Encrypted</span>
        {/if}
      </div>

      <textarea
        bind:value={noteText}
        placeholder="Add private observations (work, context, mutual acquaintances). Encrypted locally with hardware Keystore Key B..."
        rows="4"
        class="note-textarea"
      ></textarea>

      <button class="save-note-btn ig-btn-primary" onclick={handleSaveNote} disabled={isSavingNote}>
        {isSavingNote ? 'Encrypting & Saving...' : 'Save Encrypted Note'}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    z-index: 100;
    display: flex;
    justify-content: center;
    align-items: flex-end;
  }

  .sheet {
    width: 100%;
    max-width: 540px;
    max-height: calc(90vh - var(--safe-top));
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    background-color: var(--bg-surface);
    border-top: 1px solid var(--border-strong);
    border-radius: 16px 16px 0 0;
    padding: 16px 20px calc(24px + max(12px, env(safe-area-inset-bottom, 0px))) 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    animation: slideUp 200ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes slideUp {
    from { transform: translateY(100%); }
    to { transform: translateY(0); }
  }

  .sheet-handle {
    width: 36px;
    height: 4px;
    border-radius: 2px;
    background-color: var(--border-strong);
    align-self: center;
    margin-bottom: 4px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .profile-info {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .avatar-large {
    width: 56px;
    height: 56px;
    min-width: 56px;
    min-height: 56px;
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    background-color: var(--bg-surface-elevated);
    border: 1.5px solid var(--border-strong);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .user-handle {
    font-size: 16px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .user-sub {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 18px;
    cursor: pointer;
    padding: 4px 8px;
  }

  .relationship-matrix-wrapper {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
  }

  .matrix-cards-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .matrix-dual-card {
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: all 0.15s ease;
  }

  .matrix-dual-card.positive {
    border-color: rgba(16, 185, 129, 0.25);
    background: rgba(16, 185, 129, 0.04);
  }

  .matrix-dual-card.negative {
    border-color: rgba(244, 63, 94, 0.15);
    background: rgba(244, 63, 94, 0.02);
  }

  .matrix-card-head {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .matrix-icon {
    font-size: 11px;
    font-weight: 800;
  }

  .matrix-dual-card.positive .matrix-icon {
    color: var(--accent-positive);
  }

  .matrix-dual-card.negative .matrix-icon {
    color: var(--accent-negative);
  }

  .matrix-label {
    font-size: 9px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .matrix-state-pill {
    font-size: 10.5px;
    font-weight: 700;
    padding: 3px 6px;
    border-radius: 6px;
    text-align: center;
    letter-spacing: 0.03em;
  }

  .matrix-state-pill.yes {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-positive);
    border: 1px solid rgba(16, 185, 129, 0.25);
  }

  .matrix-state-pill.no {
    background: rgba(244, 63, 94, 0.1);
    color: var(--accent-negative);
    border: 1px solid rgba(244, 63, 94, 0.2);
  }

  .matrix-mutual-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 10px;
    gap: 8px;
    border: 1px solid var(--border-subtle);
  }

  .matrix-mutual-banner.is-mutual {
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.12) 0%, rgba(124, 58, 237, 0.06) 100%);
    border-color: rgba(168, 85, 247, 0.35);
  }

  .matrix-mutual-banner.is-asymmetric {
    background: rgba(255, 255, 255, 0.02);
    border-color: var(--border-subtle);
  }

  .mutual-banner-left {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }

  .mutual-badge-icon {
    font-size: 14px;
    color: var(--accent-primary);
    flex-shrink: 0;
  }

  .mutual-text-col {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .mutual-banner-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--text-primary);
  }

  .mutual-banner-sub {
    font-size: 9.5px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mutual-status-tag {
    font-size: 9px;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 6px;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .mutual-status-tag.confirmed {
    background: rgba(168, 85, 247, 0.2);
    color: #e9d5ff;
    border: 1px solid rgba(168, 85, 247, 0.4);
  }

  .mutual-status-tag.oneway {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .meta-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background-color: var(--bg-surface-elevated);
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
  }

  .meta-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }

  .meta-label { color: var(--text-secondary); }
  .meta-val { color: var(--text-primary); font-size: 11px; }

  .note-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .note-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .note-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .badge-enc {
    font-size: 8px;
    background: var(--bg-surface-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--text-tertiary);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .save-status {
    font-size: 11px;
    color: var(--accent-positive);
  }

  .note-textarea {
    width: 100%;
    background-color: var(--bg-root);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 12px;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 13px;
    resize: none;
    outline: none;
  }

  .note-textarea:focus {
    border-color: var(--border-strong);
  }

  .save-note-btn {
    align-self: flex-end;
  }
</style>
