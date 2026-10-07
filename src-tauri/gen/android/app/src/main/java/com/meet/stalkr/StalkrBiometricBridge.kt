package com.meet.stalkr

import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity

/**
 * Native Android Biometric Bridge.
 *
 * Integrates AndroidX BiometricPrompt directly with Stalkr's web environment.
 * Supports hardware fingerprint scanners, face recognition, and fallback device credentials (PIN/pattern).
 */
class StalkrBiometricBridge(
    private val activity: FragmentActivity,
    private val webView: WebView
) {

    @JavascriptInterface
    fun canAuthenticate(): Boolean {
        return try {
            val biometricManager = BiometricManager.from(activity)
            val authenticators = BiometricManager.Authenticators.BIOMETRIC_STRONG or
                    BiometricManager.Authenticators.DEVICE_CREDENTIAL
            val canAuth = biometricManager.canAuthenticate(authenticators)
            canAuth == BiometricManager.BIOMETRIC_SUCCESS
        } catch (e: Exception) {
            false
        }
    }

    @JavascriptInterface
    fun authenticate(title: String, subtitle: String) {
        activity.runOnUiThread {
            try {
                val promptTitle = if (title.isNotBlank()) title else "Stalkr Privacy Lock"
                val promptSubtitle = if (subtitle.isNotBlank()) subtitle else "Confirm biometric or device lock"

                val promptInfo = BiometricPrompt.PromptInfo.Builder()
                    .setTitle(promptTitle)
                    .setSubtitle(promptSubtitle)
                    .setAllowedAuthenticators(
                        BiometricManager.Authenticators.BIOMETRIC_STRONG or
                                BiometricManager.Authenticators.DEVICE_CREDENTIAL
                    )
                    .build()

                val biometricPrompt = BiometricPrompt(
                    activity,
                    ContextCompat.getMainExecutor(activity),
                    object : BiometricPrompt.AuthenticationCallback() {
                        override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                            super.onAuthenticationSucceeded(result)
                            webView.post {
                                webView.evaluateJavascript(
                                    "window.__stalkr_onBiometricResult && window.__stalkr_onBiometricResult(true, null);",
                                    null
                                )
                            }
                        }

                        override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                            super.onAuthenticationError(errorCode, errString)
                            webView.post {
                                val escaped = errString.toString().replace("'", "\\'")
                                webView.evaluateJavascript(
                                    "window.__stalkr_onBiometricResult && window.__stalkr_onBiometricResult(false, '$escaped');",
                                    null
                                )
                            }
                        }

                        override fun onAuthenticationFailed() {
                            super.onAuthenticationFailed()
                            // Sensor detected a mismatch; BiometricPrompt handles inline feedback automatically.
                        }
                    }
                )

                biometricPrompt.authenticate(promptInfo)
            } catch (e: Exception) {
                val escaped = (e.message ?: "Authentication error").replace("'", "\\'")
                webView.post {
                    webView.evaluateJavascript(
                        "window.__stalkr_onBiometricResult && window.__stalkr_onBiometricResult(false, '$escaped');",
                        null
                    )
                }
            }
        }
    }
}
