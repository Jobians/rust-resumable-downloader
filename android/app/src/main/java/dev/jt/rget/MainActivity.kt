package dev.jt.rget

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.ProgressBar
import android.widget.TextView
import java.io.File
import java.util.concurrent.Executors

private const val STORAGE_PERMISSION_REQUEST = 1001

class MainActivity : Activity() {
    private lateinit var rget: Rget
    private lateinit var urlInput: EditText
    private lateinit var progressBar: ProgressBar
    private lateinit var statusText: TextView
    private lateinit var percentText: TextView
    private lateinit var speedText: TextView
    private lateinit var etaText: TextView
    private lateinit var sizeText: TextView
    private lateinit var detailText: TextView
    private lateinit var downloadButton: Button
    private lateinit var cancelButton: Button

    private val publishExecutor = Executors.newSingleThreadExecutor()
    private var pendingDownload = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        rget = Rget()
        urlInput = findViewById(R.id.urlInput)
        progressBar = findViewById(R.id.progressBar)
        statusText = findViewById(R.id.statusText)
        percentText = findViewById(R.id.percentText)
        speedText = findViewById(R.id.speedText)
        etaText = findViewById(R.id.etaText)
        sizeText = findViewById(R.id.sizeText)
        detailText = findViewById(R.id.detailText)
        downloadButton = findViewById(R.id.downloadButton)
        cancelButton = findViewById(R.id.cancelButton)

        downloadButton.setOnClickListener { startDownload() }
        cancelButton.setOnClickListener { rget.cancel() }
        resetStats()
    }

    private fun startDownload() {
        val url = urlInput.text.toString().trim()
        if (url.isEmpty()) {
            detailText.setText(R.string.error_empty_url)
            return
        }

        if (!hasStoragePermission()) {
            pendingDownload = true
            requestPermissions(
                arrayOf(Manifest.permission.WRITE_EXTERNAL_STORAGE),
                STORAGE_PERMISSION_REQUEST,
            )
            return
        }

        val output = File(filesDir, "rget-download.part")
        setBusy(true)
        statusText.setText(R.string.status_starting)
        percentText.setText(R.string.percent_zero)
        progressBar.progress = 0
        detailText.setText(R.string.detail_starting)
        resetStats()

        val started = rget.download(url, output.absolutePath, object : Rget.Listener {
            override fun onProgress(progress: Rget.Progress) {
                progressBar.progress = progress.percent
                percentText.text = getString(R.string.percent_format, progress.percent)
                statusText.setText(R.string.status_downloading)
                speedText.text = progress.formattedSpeed
                etaText.text = progress.formattedEta
                sizeText.text = progress.formattedSize
            }

            override fun onComplete(
                outputPath: String,
                filename: String,
                contentType: String?,
            ) {
                statusText.setText(R.string.status_saving)
                progressBar.progress = 100
                percentText.text = getString(R.string.percent_format, 100)
                clearLiveStats()
                downloadButton.isEnabled = false
                cancelButton.isEnabled = false
                urlInput.isEnabled = false
                val displayName = filename

                publishExecutor.execute {
                    val result = runCatching {
                        val destination = DownloadsStore.publish(
                            this@MainActivity,
                            File(outputPath),
                            displayName,
                            contentType,
                        )
                        File(outputPath).delete()
                        destination
                    }
                    runOnUiThread {
                        if (isDestroyed) {
                            return@runOnUiThread
                        }
                        setBusy(false)
                        result.onSuccess { destination ->
                            statusText.setText(R.string.status_complete)
                            detailText.text = getString(R.string.detail_saved, destination)
                        }.onFailure { error ->
                            statusText.setText(R.string.status_failed)
                            detailText.text = getString(
                                R.string.error_save_failed,
                                error.message ?: getString(R.string.error_unknown),
                            )
                        }
                    }
                }
            }

            override fun onCancelled() {
                setBusy(false)
                statusText.setText(R.string.status_cancelled)
                clearLiveStats()
                detailText.setText(R.string.detail_cancelled)
            }

            override fun onError(message: String) {
                setBusy(false)
                statusText.setText(R.string.status_failed)
                clearLiveStats()
                detailText.text = getString(R.string.detail_error, message)
            }
        })

        if (!started) {
            setBusy(false)
            statusText.setText(R.string.status_failed)
            detailText.setText(R.string.error_start_failed)
        }
    }

    private fun hasStoragePermission(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M ||
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q
        ) {
            return true
        }
        return checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) ==
            PackageManager.PERMISSION_GRANTED
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode != STORAGE_PERMISSION_REQUEST) {
            return
        }

        val granted = grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED
        if (granted) {
            if (pendingDownload) {
                pendingDownload = false
                startDownload()
            }
        } else {
            pendingDownload = false
            statusText.setText(R.string.status_failed)
            detailText.setText(R.string.error_storage_permission)
        }
    }

    private fun setBusy(busy: Boolean) {
        downloadButton.isEnabled = !busy
        cancelButton.isEnabled = busy
        urlInput.isEnabled = !busy
    }

    private fun resetStats() {
        speedText.setText(R.string.stat_empty)
        etaText.setText(R.string.stat_empty)
        sizeText.setText(R.string.stat_empty)
    }

    private fun clearLiveStats() {
        speedText.setText(R.string.stat_empty)
        etaText.setText(R.string.stat_empty)
    }

    override fun onDestroy() {
        rget.shutdown()
        publishExecutor.shutdownNow()
        super.onDestroy()
    }
}
