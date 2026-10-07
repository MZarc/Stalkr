package com.meet.stalkr

import android.content.Context
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Headless periodic background synchronization worker.
 * Executes on Android WorkManager schedule without spinning up WebView.
 */
class SyncWorker(
    appContext: Context,
    params: WorkerParameters
) : CoroutineWorker(appContext, params) {

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        val accountId = inputData.getString("account_id") ?: return@withContext Result.failure()
        val candidateFiles = listOf(
            File(applicationContext.filesDir, "stalkr.db"),
            File(applicationContext.dataDir, "stalkr.db"),
            File(applicationContext.noBackupFilesDir, "stalkr.db"),
            File(applicationContext.filesDir, "databases/stalkr.db"),
            applicationContext.getDatabasePath("stalkr.db")
        )
        val dbFile = candidateFiles.firstOrNull { it.exists() } ?: File(applicationContext.filesDir, "stalkr.db")

        try {
            val responseJson = StalkrNative.headlessSync(accountId, dbFile.absolutePath)
            if (responseJson.contains("\"success\":true") || responseJson.contains("\"changes_count\"") || responseJson.contains("\"status\":\"complete\"")) {
                Result.success()
            } else {
                Result.retry()
            }
        } catch (e: Exception) {
            Result.failure()
        }
    }
}
