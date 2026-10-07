import type { Account } from './api';

export interface NativeLoginResult {
  success: boolean;
  account?: Account;
  error?: string;
}

/**
 * Checks if genuine Android Chromium in-app login bridge is available.
 */
export function isNativeAuthAvailable(): boolean {
  return typeof window !== 'undefined' && !!(window as any).StalkrAuth?.launchInstagramLogin;
}

/**
 * Launches the official Instagram In-App login flow.
 * Intercepts session cookies safely, connects the account, and triggers initial metrics sync.
 */
export function startNativeInstagramLogin(
  onStatusUpdate?: (status: string) => void
): Promise<NativeLoginResult> {
  return new Promise((resolve) => {
    if (!isNativeAuthAvailable()) {
      resolve({
        success: false,
        error: 'Native Instagram authentication is only available on Android devices.',
      });
      return;
    }

    onStatusUpdate?.('Opening official Instagram authentication...');

    (window as any).__stalkr_onInstagramLogin = async (payload: {
      success: boolean;
      sessionId?: string;
      dsUserId?: string;
      csrfToken?: string;
      cookies?: string;
      username?: string;
      displayName?: string;
      avatarUrl?: string;
      followersCount?: number;
      followingCount?: number;
      isPrivate?: boolean;
      isVerified?: boolean;
      error?: string;
    }) => {
      delete (window as any).__stalkr_onInstagramLogin;

      if (!payload.success || !payload.sessionId || !payload.dsUserId) {
        resolve({
          success: false,
          error: payload.error || 'Instagram login was cancelled or dismissed.',
        });
        return;
      }

      try {
        const { api } = await import('./api');
        onStatusUpdate?.('Saving authenticated session...');
        const account = await api.connectInstagram(
          payload.sessionId,
          payload.dsUserId,
          payload.csrfToken || undefined,
          payload.cookies || undefined,
          {
            username: payload.username || undefined,
            displayName: payload.displayName || undefined,
            avatarUrl: payload.avatarUrl || undefined,
            followersCount: payload.followersCount,
            followingCount: payload.followingCount,
            isPrivate: payload.isPrivate,
            isVerified: payload.isVerified,
          }
        );

        onStatusUpdate?.(`Session saved! Fetching real Instagram metrics...`);
        let freshAccount = account;
        try {
          await api.syncNow(account.id);
          // Re-fetch account to get updated profile data (username, followers, avatar)
          // that sync may have refreshed from Instagram
          const allAccounts = await api.getAccounts();
          const updated = allAccounts.find((a) => a.id === account.id);
          if (updated) freshAccount = updated;
          onStatusUpdate?.(`Connected @${freshAccount.username}! Ready.`);
        } catch (syncErr) {
          console.warn('Initial sync deferred:', syncErr);
          onStatusUpdate?.(`Connected! Metrics will refresh shortly.`);
        }

        resolve({ success: true, account: freshAccount });
      } catch (err: any) {
        resolve({
          success: false,
          error: typeof err === 'string' ? err : err?.message || 'Failed to authenticate Instagram session.',
        });
      }
    };

    try {
      (window as any).StalkrAuth.launchInstagramLogin();
    } catch (err: any) {
      delete (window as any).__stalkr_onInstagramLogin;
      resolve({
        success: false,
        error: err?.message || 'Failed to launch in-app Instagram login.',
      });
    }
  });
}

/**
 * Open external URL or mailto intent via native Android bridge or standard fallback.
 */
export function openExternalUrl(url: string): void {
  if (typeof window !== 'undefined' && (window as any).StalkrAuth?.openUrl) {
    try {
      const handled = (window as any).StalkrAuth.openUrl(url);
      if (handled) return;
    } catch (e) {
      console.warn('Native openUrl failed:', e);
    }
  }
  window.open(url, '_blank', 'noopener,noreferrer');
}
