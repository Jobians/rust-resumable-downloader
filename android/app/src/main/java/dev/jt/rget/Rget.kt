package dev.jt.rget

import android.os.Handler
import android.os.Looper
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicLong

class Rget {
    data class Progress(
        val downloadedBytes: Long,
        val totalBytes: Long,
        val bytesPerSecond: Double,
        val etaSeconds: Long,
        val formattedSize: String,
        val formattedSpeed: String,
        val formattedEta: String,
    ) {
        val percent: Int
            get() = if (totalBytes > 0) {
                ((downloadedBytes.coerceAtMost(totalBytes) * 100) / totalBytes).toInt()
            } else {
                0
            }
    }

    interface Listener {
        fun onProgress(progress: Progress) {}
        fun onComplete(outputPath: String) {}
        fun onComplete(
            outputPath: String,
            filename: String,
            contentType: String?,
        ) {
            onComplete(outputPath)
        }
        fun onCancelled() {}
        fun onError(message: String) {}
    }

    private val executor = Executors.newSingleThreadExecutor { runnable ->
        Thread(runnable, "rget-download")
    }
    private val mainHandler = Handler(Looper.getMainLooper())
    private val activeHandle = AtomicLong(0L)

    fun download(url: String, outputPath: String, listener: Listener): Boolean {
        val handle = RgetNative.nativeCreateHandle()
        if (!activeHandle.compareAndSet(0L, handle)) {
            RgetNative.nativeFreeHandle(handle)
            mainHandler.post {
                listener.onError("A download is already running on this Rget instance")
            }
            return false
        }

        val callback = object : RgetNativeCallback {
            override fun onProgress(
                downloaded: Long,
                total: Long,
                speed: Double,
                eta: Long,
                size: String,
                speedText: String,
                etaText: String,
            ) {
                mainHandler.post {
                    listener.onProgress(
                        Progress(
                            downloaded,
                            total,
                            speed,
                            eta,
                            size,
                            speedText,
                            etaText,
                        ),
                    )
                }
            }

            override fun onComplete(
                outputPath: String,
                filename: String,
                contentType: String,
            ) {
                mainHandler.post {
                    listener.onComplete(
                        outputPath,
                        filename,
                        contentType.ifEmpty { null },
                    )
                }
            }

            override fun onCancelled() {
                mainHandler.post { listener.onCancelled() }
            }

            override fun onError(message: String) {
                mainHandler.post { listener.onError(message) }
            }
        }

        executor.execute {
            try {
                RgetNative.nativeDownload(handle, url, outputPath, callback)
            } catch (throwable: Throwable) {
                mainHandler.post {
                    listener.onError(throwable.message ?: throwable.toString())
                }
            } finally {
                activeHandle.compareAndSet(handle, 0L)
                RgetNative.nativeFreeHandle(handle)
            }
        }

        return true
    }

    fun cancel() {
        val handle = activeHandle.get()
        if (handle != 0L) {
            RgetNative.nativeCancel(handle)
        }
    }

    fun shutdown() {
        cancel()
        executor.shutdown()
    }
}

internal interface RgetNativeCallback {
    fun onProgress(
        downloaded: Long,
        total: Long,
        speed: Double,
        eta: Long,
        size: String,
        speedText: String,
        etaText: String,
    )
    fun onComplete(outputPath: String, filename: String, contentType: String)
    fun onCancelled()
    fun onError(message: String)
}

internal object RgetNative {
    init {
        System.loadLibrary("rget")
    }

    external fun nativeCreateHandle(): Long
    external fun nativeDownload(
        handle: Long,
        url: String,
        outputPath: String,
        callback: RgetNativeCallback,
    ): Int
    external fun nativeCancel(handle: Long)
    external fun nativeFreeHandle(handle: Long)
}
