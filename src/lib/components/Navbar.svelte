<script lang="ts">
  import Icon from './Icon.svelte';

  let { activeTab, onChangeTab }: { activeTab: string; onChangeTab: (tab: string) => void } = $props();

  // 5 Tabs with Home/Dashboard anchored prominently in the dead center
  const leftTabs: { id: string; label: string; icon: 'users' | 'changes' }[] = [
    {
      id: 'people',
      label: 'Circle',
      icon: 'users',
    },
    {
      id: 'changes',
      label: 'Changes',
      icon: 'changes',
    },
  ];

  const rightTabs: { id: string; label: string; icon: 'sync' | 'settings' }[] = [
    {
      id: 'monitor',
      label: 'Sync',
      icon: 'sync',
    },
    {
      id: 'settings',
      label: 'Settings',
      icon: 'settings',
    },
  ];
</script>

<nav class="bottom-nav">
  <div class="nav-inner">
    <!-- Left Navigation Group (Circle, Changes) -->
    <div class="nav-group">
      {#each leftTabs as tab}
        <button
          class="nav-item {activeTab === tab.id ? 'active' : ''}"
          onclick={() => onChangeTab(tab.id)}
          aria-label={tab.label}
        >
          <span class="nav-icon">
            <Icon name={tab.icon} size={22} strokeWidth={activeTab === tab.id ? 2.5 : 2.0} />
          </span>
          <span class="nav-label">{tab.label}</span>
          {#if activeTab === tab.id}
            <span class="active-dot"></span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- Centerpiece Hero: Home / Dashboard Button -->
    <div class="center-hero-wrapper">
      <button
        class="center-hero-btn {activeTab === 'pulse' ? 'active' : ''}"
        onclick={() => onChangeTab('pulse')}
        aria-label="Home Dashboard"
        title="Home Dashboard"
      >
        <div class="hero-inner-circle">
          <span class="hero-icon">
            <Icon name="home" size={24} color="#ffffff" strokeWidth={2.4} />
          </span>
        </div>
      </button>
      <span class="center-label {activeTab === 'pulse' ? 'active' : ''}">Dashboard</span>
    </div>

    <!-- Right Navigation Group (Targets, Settings) -->
    <div class="nav-group">
      {#each rightTabs as tab}
        <button
          class="nav-item {activeTab === tab.id ? 'active' : ''}"
          onclick={() => onChangeTab(tab.id)}
          aria-label={tab.label}
        >
          <span class="nav-icon">
            <Icon name={tab.icon} size={22} strokeWidth={activeTab === tab.id ? 2.5 : 2.0} />
          </span>
          <span class="nav-label">{tab.label}</span>
          {#if activeTab === tab.id}
            <span class="active-dot"></span>
          {/if}
        </button>
      {/each}
    </div>
  </div>
</nav>

<style>
  .bottom-nav {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: calc(64px + max(12px, env(safe-area-inset-bottom, 0px)));
    background: rgba(14, 10, 26, 0.94);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-top: 1px solid rgba(168, 85, 247, 0.18);
    display: flex;
    justify-content: center;
    align-items: center;
    padding-bottom: max(12px, env(safe-area-inset-bottom, 0px));
    padding-left: max(6px, env(safe-area-inset-left, 0px));
    padding-right: max(6px, env(safe-area-inset-right, 0px));
    z-index: 95;
    box-shadow: 0 -8px 32px rgba(6, 3, 14, 0.6);
  }

  .nav-inner {
    width: 100%;
    max-width: 520px;
    height: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 4px;
    position: relative;
  }

  .nav-group {
    display: flex;
    align-items: center;
    justify-content: space-around;
    flex: 1;
  }

  .nav-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 6px 8px;
    border-radius: 12px;
    position: relative;
    transition: all 200ms cubic-bezier(0.16, 1, 0.3, 1);
    min-width: 44px;
    min-height: 44px;
    touch-action: manipulation;
  }

  .nav-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 200ms cubic-bezier(0.16, 1, 0.3, 1), color 200ms ease;
  }

  .nav-label {
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    transition: color 200ms ease;
  }

  .nav-item:hover {
    color: var(--text-secondary);
  }

  .nav-item.active {
    color: #ffffff;
  }

  .nav-item.active .nav-icon {
    transform: translateY(-2px);
    color: #ffffff;
  }

  .nav-item.active .nav-label {
    font-weight: 600;
    color: #ffffff;
  }

  .active-dot {
    position: absolute;
    bottom: 0px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: #e1306c;
  }

  /* Centerpiece Hero Home / Dashboard Button */
  .center-hero-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    margin: 0 8px;
    top: -10px;
  }

  .center-hero-btn {
    width: 50px;
    height: 50px;
    border-radius: 50%;
    background: var(--ig-gradient);
    border: 3px solid var(--bg-surface);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    transition: transform 180ms cubic-bezier(0.16, 1, 0.3, 1), box-shadow 180ms ease;
    padding: 0;
  }

  .center-hero-btn:hover {
    transform: translateY(-2px) scale(1.05);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.55);
  }

  .center-hero-btn:active {
    transform: translateY(0px) scale(0.95);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
  }

  .center-hero-btn.active {
    background: var(--ig-gradient);
    border-color: #ffffff;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.55);
  }

  .hero-inner-circle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
  }

  .hero-icon {
    color: #ffffff;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.5));
    transition: transform 200ms ease;
  }

  .center-hero-btn:hover .hero-icon {
    transform: scale(1.08);
  }

  .center-hero-btn.active .hero-icon {
    transform: scale(1.05);
    stroke-width: 2.4;
  }

  .center-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-tertiary);
    margin-top: 3px;
    letter-spacing: 0.02em;
    transition: color 200ms ease;
  }

  .center-label.active {
    color: #ffffff;
    font-weight: 700;
  }

  @media (max-width: 360px) {
    .nav-label {
      font-size: 9.5px;
    }
    .center-hero-btn {
      width: 44px;
      height: 44px;
    }
    .center-hero-wrapper {
      margin: 0 4px;
      top: -7px;
    }
    .center-label {
      font-size: 9px;
    }
  }
</style>
