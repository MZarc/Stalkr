package com.meet.stalkr

import android.app.Activity
import android.content.Intent
import android.webkit.JavascriptInterface
import android.webkit.WebView
import org.json.JSONObject

/**
 * Official Instagram Authentication Bridge.
 *
 * Connects the web UI directly to the genuine Android Chromium In-App Login Activity.
 * Enables zero-risk, password & 2FA authenticated Instagram session capture
 * without exposing credentials or requiring manual cookie copying.
 */
class StalkrAuthBridge(
    private val activity: MainActivity,
    private val webView: WebView
) {

    @JavascriptInterface
    fun isAvailable(): Boolean {
        return true
    }

    @JavascriptInterface
    fun schedulePeriodicSync(accountId: String, intervalMinutes: Long): Boolean {
        return try {
            MainActivity.setPeriodicSync(accountId, intervalMinutes)
            true
        } catch (e: Exception) {
            false
        }
    }

    @JavascriptInterface
    fun cancelSync(accountId: String): Boolean {
        return try {
            MainActivity.setPeriodicSync(accountId, 0)
            true
        } catch (e: Exception) {
            false
        }
    }

    @JavascriptInterface
    fun openUrl(url: String): Boolean {
        return try {
            val intent = if (url.startsWith("mailto:")) {
                Intent(Intent.ACTION_SENDTO, android.net.Uri.parse(url)).apply {
                    flags = Intent.FLAG_ACTIVITY_NEW_TASK
                }
            } else {
                Intent(Intent.ACTION_VIEW, android.net.Uri.parse(url)).apply {
                    flags = Intent.FLAG_ACTIVITY_NEW_TASK
                }
            }
            activity.startActivity(intent)
            true
        } catch (e: Exception) {
            false
        }
    }

    @JavascriptInterface
    fun launchInstagramLogin() {
        activity.runOnUiThread {
            try {
                val intent = Intent(activity, InstagramLoginActivity::class.java)
                activity.launchActivityForResult(intent) { result ->
                    if (result != null && result.resultCode == Activity.RESULT_OK && result.data != null) {
                        val sessionId = result.data?.getStringExtra(InstagramLoginActivity.EXTRA_SESSION_ID) ?: ""
                        val dsUserId = result.data?.getStringExtra(InstagramLoginActivity.EXTRA_DS_USER_ID) ?: ""
                        val csrfToken = result.data?.getStringExtra(InstagramLoginActivity.EXTRA_CSRF_TOKEN) ?: ""
                        val cookies = result.data?.getStringExtra(InstagramLoginActivity.EXTRA_COOKIES) ?: ""
                        val username = result.data?.getStringExtra(InstagramLoginActivity.EXTRA_USERNAME) ?: ""
                        val displayName = result.data?.getStringExtra(InstagramLoginActivity.EXTRA_DISPLAY_NAME) ?: ""
                        val avatarUrl = result.data?.getStringExtra(InstagramLoginActivity.EXTRA_AVATAR_URL) ?: ""
                        val followersCount = result.data?.getLongExtra(InstagramLoginActivity.EXTRA_FOLLOWERS_COUNT, 0L) ?: 0L
                        val followingCount = result.data?.getLongExtra(InstagramLoginActivity.EXTRA_FOLLOWING_COUNT, 0L) ?: 0L
                        val isPrivate = result.data?.getBooleanExtra(InstagramLoginActivity.EXTRA_IS_PRIVATE, false) ?: false
                        val isVerified = result.data?.getBooleanExtra(InstagramLoginActivity.EXTRA_IS_VERIFIED, false) ?: false

                        val payload = JSONObject().apply {
                            put("success", true)
                            put("sessionId", sessionId)
                            put("dsUserId", dsUserId)
                            put("csrfToken", csrfToken)
                            put("cookies", cookies)
                            put("username", username)
                            put("displayName", displayName)
                            put("avatarUrl", avatarUrl)
                            put("followersCount", followersCount)
                            put("followingCount", followingCount)
                            put("isPrivate", isPrivate)
                            put("isVerified", isVerified)
                            put("error", JSONObject.NULL)
                        }.toString()

                        webView.post {
                            webView.evaluateJavascript(
                                "window.__stalkr_onInstagramLogin && window.__stalkr_onInstagramLogin($payload);",
                                null
                            )
                        }
                    } else {
                        val payload = JSONObject().apply {
                            put("success", false)
                            put("sessionId", JSONObject.NULL)
                            put("dsUserId", JSONObject.NULL)
                            put("csrfToken", JSONObject.NULL)
                            put("error", "Login cancelled or dismissed")
                        }.toString()

                        webView.post {
                            webView.evaluateJavascript(
                                "window.__stalkr_onInstagramLogin && window.__stalkr_onInstagramLogin($payload);",
                                null
                            )
                        }
                    }
                }
            } catch (e: Exception) {
                val payload = JSONObject().apply {
                    put("success", false)
                    put("sessionId", JSONObject.NULL)
                    put("dsUserId", JSONObject.NULL)
                    put("csrfToken", JSONObject.NULL)
                    put("error", e.message ?: "Failed to launch login activity")
                }.toString()

                webView.post {
                    webView.evaluateJavascript(
                        "window.__stalkr_onInstagramLogin && window.__stalkr_onInstagramLogin($payload);",
                        null
                    )
                }
            }
        }
    }
}
