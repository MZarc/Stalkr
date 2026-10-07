package com.meet.stalkr

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    instance = this
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.addJavascriptInterface(StalkrBiometricBridge(this, webView), "StalkrBiometric")
    webView.addJavascriptInterface(StalkrAuthBridge(this, webView), "StalkrAuth")
  }

  companion object {
    var instance: MainActivity? = null
      private set

    @JvmStatic
    fun setPeriodicSync(accountId: String, intervalMinutes: Long) {
      instance?.let { ctx ->
        if (intervalMinutes <= 0) {
          WorkScheduler.cancelSync(ctx, accountId)
        } else {
          WorkScheduler.schedulePeriodicSync(ctx, accountId, intervalMinutes)
        }
      }
    }

    @JvmStatic
    fun cancelAllSync() {
      instance?.let { ctx ->
        WorkScheduler.cancelAll(ctx)
      }
    }
  }
}
