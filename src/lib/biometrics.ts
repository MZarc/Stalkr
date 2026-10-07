/**
 * Biometric authentication service for Stalkr.
 * Seamlessly interfaces with Android BiometricPrompt via StalkrBiometric native bridge,
 * with fallbacks for desktop development mode.
 */

declare global {
  interface Window {
    StalkrBiometric?: {
      canAuthenticate: () => boolean;
      authenticate: (title: string, subtitle: string) => void;
    };
    __stalkr_onBiometricResult?: (success: boolean, error: string | null) => void;
  }
}

export async function isBiometricsAvailable(): Promise<boolean> {
  if (typeof window !== 'undefined' && window.StalkrBiometric) {
    try {
      return Boolean(window.StalkrBiometric.canAuthenticate());
    } catch {
      return false;
    }
  }

  // Fallback for WebAuthn in modern browsers
  if (typeof window !== 'undefined' && window.PublicKeyCredential) {
    try {
      return await window.PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
    } catch {
      return false;
    }
  }

  return true; // allow dev testing
}

export function promptBiometricAuth(
  title = 'Stalkr Privacy Lock',
  subtitle = 'Confirm biometric or device lock to continue'
): Promise<{ success: boolean; error?: string }> {
  return new Promise((resolve) => {
    if (typeof window !== 'undefined' && window.StalkrBiometric) {
      window.__stalkr_onBiometricResult = (success: boolean, error: string | null) => {
        delete window.__stalkr_onBiometricResult;
        resolve({ success, error: error || undefined });
      };

      try {
        window.StalkrBiometric.authenticate(title, subtitle);
      } catch (err: any) {
        delete window.__stalkr_onBiometricResult;
        resolve({ success: false, error: err?.message || String(err) });
      }
      return;
    }

    // Fallback for local testing in desktop browser / tauri dev
    setTimeout(() => {
      resolve({ success: true });
    }, 200);
  });
}
