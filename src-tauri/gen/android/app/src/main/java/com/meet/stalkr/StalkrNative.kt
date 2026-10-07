package com.meet.stalkr

object StalkrNative {
    init {
        try {
            System.loadLibrary("app_lib")
        } catch (e: UnsatisfiedLinkError) {
            // Native library loaded during runtime
        }
    }

    /**
     * Executes headless sync directly from Rust without requiring WebView.
     * @param accountId Stable account identifier
     * @param dbPath Absolute path to stalkr.db
     * @return JSON string containing sync status and report
     */
    external fun headlessSync(accountId: String, dbPath: String): String
}
