import { api, type Account, type SyncReport } from './api';

export type SyncStateEnum = 'IDLE' | 'CONNECTING' | 'FETCHING' | 'VALIDATING' | 'DIFFING' | 'COMPLETE';

class SyncCoordinatorStore {
  isSyncing = $state(false);
  activeAccountId = $state<string | null>(null);
  syncState = $state<SyncStateEnum>('IDLE');
  lastReport = $state<SyncReport | null>(null);
  lastSyncAt = $state<number | null>(null);
  syncVersion = $state(0);

  async executeSync(account: Account): Promise<SyncReport> {
    if (this.isSyncing) {
      if (this.lastReport) return this.lastReport;
      throw new Error('A sync cycle is already in progress.');
    }

    this.isSyncing = true;
    this.activeAccountId = account.id;
    this.syncState = 'CONNECTING';

    const t1 = setTimeout(() => {
      if (this.isSyncing) this.syncState = 'FETCHING';
    }, 400);

    const t2 = setTimeout(() => {
      if (this.isSyncing) this.syncState = 'VALIDATING';
    }, 1000);

    const t3 = setTimeout(() => {
      if (this.isSyncing) this.syncState = 'DIFFING';
    }, 1500);

    try {
      const res = await api.syncNow(account.id);
      clearTimeout(t1);
      clearTimeout(t2);
      clearTimeout(t3);

      this.lastReport = res;
      this.syncState = 'COMPLETE';

      const now = Math.floor(Date.now() / 1000);
      this.lastSyncAt = now;
      // Only advance success markers on real success. Throttled / incomplete /
      // quarantined syncs preserve the last VERIFIED state instead of
      // presenting stale-or-partial numbers as fresh.
      if (res.success && res.status === 'complete') {
        account.last_successful_sync_at = now;
        account.last_attempted_sync_at = now;
        account.followers_count = res.followers_count;
        account.following_count = res.following_count;
      } else {
        account.last_attempted_sync_at = now;
      }

      try {
        const fresh = await api.getAccount(account.id);
        if (fresh) {
          account.last_successful_sync_at = fresh.last_successful_sync_at || now;
          account.followers_count = fresh.followers_count;
          account.following_count = fresh.following_count;
        }
      } catch {}

      this.syncVersion += 1;
      return res;
    } catch (e) {
      clearTimeout(t1);
      clearTimeout(t2);
      clearTimeout(t3);
      this.syncState = 'IDLE';
      throw e;
    } finally {
      this.isSyncing = false;
      setTimeout(() => {
        if (this.syncState === 'COMPLETE') {
          this.syncState = 'IDLE';
        }
      }, 3500);
    }
  }
}

export const globalSync = new SyncCoordinatorStore();
