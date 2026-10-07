<script lang="ts">
  type IconName =
    | 'home'
    | 'users'
    | 'changes'
    | 'sync'
    | 'settings'
    | 'search'
    | 'sort'
    | 'close'
    | 'chevron-down'
    | 'chevron-right'
    | 'chevron-left'
    | 'arrow-left'
    | 'arrow-right'
    | 'plus'
    | 'lock'
    | 'unlock'
    | 'shield'
    | 'fingerprint'
    | 'check'
    | 'trend-up'
    | 'trend-down'
    | 'user-plus'
    | 'user-minus'
    | 'user-check'
    | 'rename'
    | 'trash'
    | 'logout'
    | 'export'
    | 'info'
    | 'warning'
    | 'zap'
    | 'star'
    | 'clock'
    | 'mail'
    | 'globe';

  let {
    name,
    size = 20,
    color = 'currentColor',
    strokeWidth = 2.2,
    class: className = '',
  }: {
    name: IconName;
    size?: number;
    color?: string;
    strokeWidth?: number;
    class?: string;
  } = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke={color}
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class="stalkr-icon stalkr-icon-{name} {className}"
  aria-hidden="true"
>
  {#if name === 'home'}
    <!-- Solar Curved Home -->
    <path d="M2 12.2c0-2.3 1.1-4.4 2.9-5.7l4.5-3.3c1.5-1.1 3.7-1.1 5.2 0l4.5 3.3c1.8 1.3 2.9 3.4 2.9 5.7V18c0 2.2-1.8 4-4 4h-2.5c-.8 0-1.5-.7-1.5-1.5V17c0-.8-.7-1.5-1.5-1.5s-1.5.7-1.5 1.5v3.5c0 .8-.7 1.5-1.5 1.5H6c-2.2 0-4-1.8-4-4v-5.8z" />
  {:else if name === 'users'}
    <!-- Solar Smooth Community / Users -->
    <path d="M16 21v-1c0-2.2-1.8-4-4-4H8c-2.2 0-4 1.8-4 4v1" />
    <circle cx="10" cy="8" r="4" />
    <path d="M19 21v-1c0-1.7-1.1-3.2-2.6-3.8" />
    <path d="M15 4.3a4 4 0 0 1 0 7.4" />
  {:else if name === 'changes'}
    <!-- Solar Curved Activity Pulse / Changes -->
    <path d="M3 12h4l2.5-6.5 4.5 13 3-8.5 2.5 4H21" />
    <circle cx="14" cy="18.5" r="1" fill={color} />
  {:else if name === 'sync'}
    <!-- Solar Circular Refresh / Sync -->
    <path d="M20.5 8A9 9 0 0 0 5.2 5.2L3 7.5" />
    <path d="M3 3v4.5H7.5" />
    <path d="M3.5 16a9 9 0 0 0 15.3 2.8l2.2-2.3" />
    <path d="M21 21v-4.5h-4.5" />
  {:else if name === 'settings'}
    <!-- Solar Modern Widget / Settings Gear -->
    <circle cx="12" cy="12" r="3.2" />
    <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.2a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.2a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3 1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.2a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8 1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.2a1.7 1.7 0 0 0-1.4 1z" />
  {:else if name === 'search'}
    <!-- Solar Curved Search Lens -->
    <circle cx="11" cy="11" r="7" />
    <path d="M16.5 16.5 21 21" />
  {:else if name === 'sort'}
    <!-- Solar Arrow Sort Swap -->
    <path d="M7 4v16M7 4l-3 3M7 4l3 3M17 20V4M17 20l-3-3M17 20l3-3" />
  {:else if name === 'close'}
    <!-- Clean Rounded Cross -->
    <path d="M18 6 6 18M6 6l12 12" />
  {:else if name === 'chevron-down'}
    <!-- Smooth Soft Chevron Down -->
    <path d="m6 9 6 6 6-6" />
  {:else if name === 'chevron-right'}
    <!-- Smooth Soft Chevron Right -->
    <path d="m9 18 6-6-6-6" />
  {:else if name === 'chevron-left'}
    <!-- Smooth Soft Chevron Left -->
    <path d="m15 18-6-6 6-6" />
  {:else if name === 'arrow-left'}
    <!-- Clean Arrow Left -->
    <path d="m12 19-7-7 7-7" />
    <path d="M19 12H5" />
  {:else if name === 'arrow-right'}
    <!-- Clean Arrow Right -->
    <path d="m12 5 7 7-7 7" />
    <path d="M5 12h14" />
  {:else if name === 'plus'}
    <!-- Clean Bold Plus -->
    <path d="M12 5v14M5 12h14" />
  {:else if name === 'lock'}
    <!-- Solar Security Padlock -->
    <rect x="4" y="11" width="16" height="10" rx="3" />
    <path d="M8 11V7a4 4 0 0 1 8 0v4" />
    <circle cx="12" cy="16" r="1.5" fill={color} />
  {:else if name === 'unlock'}
    <!-- Solar Open Padlock -->
    <rect x="4" y="11" width="16" height="10" rx="3" />
    <path d="M8 11V7a4 4 0 0 1 8 0" />
    <circle cx="12" cy="16" r="1.5" fill={color} />
  {:else if name === 'shield'}
    <!-- Solar Cyber Defense Shield -->
    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    <path d="m9 12 2 2 4-4" />
  {:else if name === 'fingerprint'}
    <!-- Solar Biometric Fingerprint -->
    <path d="M12 10a2 2 0 0 0-2 2c0 2 2 4 2 6" />
    <path d="M14 13.5a4 4 0 0 1-1.3 3" />
    <path d="M7.8 7.8A6 6 0 0 1 18 12c0 2.2-1 4.2-2.5 5.5" />
    <path d="M5.5 12a8 8 0 0 1 13-6" />
    <path d="M2 12a10 10 0 0 1 19.3-3.7" />
  {:else if name === 'check'}
    <!-- Bold Checkmark -->
    <path d="m4 12 5 5L20 6" />
  {:else if name === 'trend-up'}
    <!-- Smooth Dynamic Trajectory Up -->
    <path d="m22 7-8.5 8.5-5-5L2 17" />
    <path d="M16 7h6v6" />
  {:else if name === 'trend-down'}
    <!-- Smooth Dynamic Trajectory Down -->
    <path d="m22 17-8.5-8.5-5 5L2 7" />
    <path d="M16 17h6v-6" />
  {:else if name === 'user-plus'}
    <!-- Solar User Add Follow -->
    <path d="M16 21v-1c0-2.2-1.8-4-4-4H6c-2.2 0-4 1.8-4 4v1" />
    <circle cx="9" cy="8" r="4" />
    <path d="M19 8v6M16 11h6" />
  {:else if name === 'user-minus'}
    <!-- Solar User Remove Unfollow -->
    <path d="M16 21v-1c0-2.2-1.8-4-4-4H6c-2.2 0-4 1.8-4 4v1" />
    <circle cx="9" cy="8" r="4" />
    <path d="M16 11h6" />
  {:else if name === 'user-check'}
    <!-- Solar User Mutual -->
    <path d="M16 21v-1c0-2.2-1.8-4-4-4H6c-2.2 0-4 1.8-4 4v1" />
    <circle cx="9" cy="8" r="4" />
    <path d="m16 11 2 2 4-4" />
  {:else if name === 'rename'}
    <!-- Solar Edit Identity Quill -->
    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
    <path d="M18.5 2.5a2.12 2.12 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
  {:else if name === 'trash'}
    <!-- Solar Clean Purge Bin -->
    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    <path d="M10 11v6M14 11v6" />
  {:else if name === 'logout'}
    <!-- Solar Door Exit Logout -->
    <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
    <path d="m16 17 5-5-5-5" />
    <path d="M21 12H9" />
  {:else if name === 'export'}
    <!-- Solar File Share / Export -->
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <path d="m7 10 5 5 5-5" />
    <path d="M12 15V3" />
  {:else if name === 'info'}
    <!-- Solar Info Dot -->
    <circle cx="12" cy="12" r="9" />
    <path d="M12 8v.1M12 12v4" />
  {:else if name === 'warning'}
    <!-- Solar Alert Triangle -->
    <path d="m21.7 18.2-8-14c-.8-1.4-2.7-1.4-3.5 0l-8 14c-.8 1.4.2 3.1 1.8 3.1h16c1.6 0 2.5-1.7 1.7-3.1z" />
    <path d="M12 9v4M12 17h.01" />
  {:else if name === 'zap'}
    <!-- Solar Lightning Bolt -->
    <path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z" />
  {:else if name === 'star'}
    <!-- Solar Star Badge -->
    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
  {:else if name === 'clock'}
    <!-- Solar Precision Clock -->
    <circle cx="12" cy="12" r="9" />
    <path d="M12 6v6l4 2" />
  {:else if name === 'mail'}
    <!-- Solar Curved Email / Mail Envelope -->
    <rect x="3" y="5" width="18" height="14" rx="3" />
    <path d="m3 7 9 6 9-6" />
  {:else if name === 'globe'}
    <!-- Solar Planet / Global Web -->
    <circle cx="12" cy="12" r="9" />
    <path d="M3.6 9h16.8M3.6 15h16.8" />
    <path d="M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18" />
  {/if}
</svg>

<style>
  .stalkr-icon {
    display: inline-block;
    vertical-align: middle;
    flex-shrink: 0;
    transition: transform 0.15s ease, stroke 0.15s ease;
  }
</style>
