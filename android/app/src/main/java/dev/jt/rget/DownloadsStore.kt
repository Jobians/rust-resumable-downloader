package dev.jt.rget

import android.content.ContentValues
import android.content.Context
import android.media.MediaScannerConnection
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import java.io.File
import java.io.IOException
import java.util.Locale

private const val DEFAULT_DOWNLOAD_FOLDER = "rget"

object DownloadsStore {
    fun publish(
        context: Context,
        source: File,
        displayName: String,
        contentType: String?,
    ): String {
        val folder = downloadFolder(context)
        val mimeType = mimeTypeFor(displayName, contentType)
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            publishScoped(context, source, folder, displayName, mimeType)
        } else {
            publishLegacy(context, source, folder, displayName)
        }
    }

    private fun publishScoped(
        context: Context,
        source: File,
        folder: String,
        displayName: String,
        mimeType: String,
    ): String {
        val resolver = context.contentResolver
        val values = ContentValues().apply {
            put(MediaStore.MediaColumns.DISPLAY_NAME, displayName)
            put(MediaStore.MediaColumns.MIME_TYPE, mimeType)
            put(
                MediaStore.MediaColumns.RELATIVE_PATH,
                "${Environment.DIRECTORY_DOWNLOADS}/$folder",
            )
            put(MediaStore.MediaColumns.IS_PENDING, 1)
        }

        val uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
            ?: throw IOException("Could not create a Downloads entry")

        try {
            resolver.openOutputStream(uri)?.use { output ->
                source.inputStream().use { input -> input.copyTo(output) }
            } ?: throw IOException("Could not open the Downloads entry")

            val ready = ContentValues().apply {
                put(MediaStore.MediaColumns.IS_PENDING, 0)
            }
            resolver.update(uri, ready, null, null)

            val actualName = resolver.query(
                uri,
                arrayOf(MediaStore.MediaColumns.DISPLAY_NAME),
                null,
                null,
                null,
            )?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0) else displayName
            } ?: displayName

            return "${Environment.DIRECTORY_DOWNLOADS}/$folder/$actualName"
        } catch (error: Throwable) {
            resolver.delete(uri, null, null)
            throw error
        }
    }

    private fun publishLegacy(
        context: Context,
        source: File,
        folder: String,
        displayName: String,
    ): String {
        val downloads = Environment.getExternalStoragePublicDirectory(
            Environment.DIRECTORY_DOWNLOADS,
        )
        val directory = File(downloads, folder)
        if (!directory.exists() && !directory.mkdirs()) {
            throw IOException("Could not create the Downloads folder")
        }

        val destination = uniqueFile(directory, displayName)
        source.inputStream().use { input ->
            destination.outputStream().use { output -> input.copyTo(output) }
        }
        MediaScannerConnection.scanFile(
            context,
            arrayOf(destination.absolutePath),
            null,
            null,
        )

        return "${Environment.DIRECTORY_DOWNLOADS}/$folder/${destination.name}"
    }

    private fun downloadFolder(context: Context): String {
        val packageName = context.packageName
            .replace(Regex("[^A-Za-z0-9._-]"), "_")
            .trim('.', '_')
            .takeIf { it.isNotEmpty() && it != "." && it != ".." }
        return packageName ?: DEFAULT_DOWNLOAD_FOLDER
    }

    private fun mimeTypeFor(displayName: String, contentType: String?): String {
        if (!contentType.isNullOrBlank()) {
            return contentType
        }

        return when (displayName.substringAfterLast('.', "").lowercase(Locale.US)) {
            "apk" -> "application/vnd.android.package-archive"
            "zip" -> "application/zip"
            "pdf" -> "application/pdf"
            "jpg", "jpeg" -> "image/jpeg"
            "png" -> "image/png"
            "txt" -> "text/plain"
            else -> "application/octet-stream"
        }
    }

    private fun uniqueFile(directory: File, name: String): File {
        var candidate = File(directory, name)
        if (!candidate.exists()) {
            return candidate
        }

        val base = name.substringBeforeLast('.', name)
        val extension = name.substringAfterLast('.', "")
        val suffix = if (extension.isEmpty()) "" else ".$extension"
        var index = 1
        while (candidate.exists()) {
            candidate = File(directory, "$base ($index)$suffix")
            index++
        }
        return candidate
    }
}
