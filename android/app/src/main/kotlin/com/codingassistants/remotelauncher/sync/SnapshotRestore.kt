package com.codingassistants.remotelauncher.sync

import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File

/**
 * Download-only companion restore (S13 / #103).
 *
 * Reads owner-copied CAS1 objects from `sync/inbox/` (64-hex names) and the
 * local `keys/cloud-sync.key`, verifies one Manifest plus every entry, and
 * writes plaintext into `sync/staging/restore/`. Never merges journals, never
 * takes a Hub lock, never uploads, and never replaces a live `hub.db`.
 */
object SnapshotRestore {
    const val LIVE_DB_WARNING: String = "live hub.db was not replaced"
    const val KEY_RELATIVE: String = "keys/cloud-sync.key"
    const val INBOX_RELATIVE: String = "sync/inbox"
    const val STAGING_RELATIVE: String = "sync/staging/restore"

    private val json = Json { ignoreUnknownKeys = true }

    fun restore(home: File): RestoreResult {
        val key = loadKey(home)
        val objects = loadInbox(File(home, INBOX_RELATIVE))
        val (baseId, manifest) = loadManifest(key, objects)
        if (manifest.format_version != 1) {
            throw RestoreException("unsupported manifest format_version")
        }
        val stagingRoot = File(home, "sync/staging")
        val dest = File(stagingRoot, "restore")
        val tmp = File(stagingRoot, "restore.tmp")
        if (tmp.exists() && !tmp.deleteRecursively()) {
            throw RestoreException("failed to clear restore staging tmp")
        }
        if (!tmp.mkdirs()) {
            throw RestoreException("failed to create restore staging")
        }
        try {
            var restored = 0
            val destinations = mutableSetOf<String>()
            for (entry in manifest.entries) {
                if (skipEntry(entry.relative_path)) {
                    continue
                }
                if (isForbidden(entry.relative_path)) {
                    throw RestoreException("manifest path is not restorable")
                }
                val blobFile =
                    objects[entry.blob_id.lowercase()]
                        ?: throw RestoreException("snapshot entry is missing")
                val bytes = blobFile.readBytes()
                val (kind, plain) = Cas1.decrypt(key, bytes)
                if (kind != ObjectKind.fromWire(entry.kind) || Cas1.sha256Hex(plain) != entry.content_hash) {
                    throw RestoreException("snapshot entry failed verification")
                }
                if (entry.size >= 0 && entry.size != plain.size.toLong()) {
                    throw RestoreException("snapshot entry failed verification")
                }
                val destFile = stagingJoin(tmp, entry.relative_path)
                if (!destinations.add(destFile.canonicalPath)) {
                    throw RestoreException("manifest has duplicate restore path")
                }
                destFile.parentFile?.mkdirs()
                destFile.writeBytes(plain)
                restored += 1
            }
            if (dest.exists() && !dest.deleteRecursively()) {
                throw RestoreException("failed to replace restore staging")
            }
            if (!tmp.renameTo(dest)) {
                throw RestoreException("failed to publish restore staging")
            }
            writeLastVerified(home, manifest.schema_version, baseId)
            val warnings = mutableListOf(LIVE_DB_WARNING)
            if (File(home, "hub.db").exists()) {
                warnings.add(LIVE_DB_WARNING)
            }
            return RestoreResult(
                restored = restored,
                warnings = warnings.distinct(),
                stagingDir = dest,
                deviceId = manifest.device_id,
                schemaVersion = manifest.schema_version,
                baseBlobId = baseId,
            )
        } catch (error: Exception) {
            tmp.deleteRecursively()
            throw error
        }
    }

    private fun loadKey(home: File): ByteArray {
        val path = File(home, KEY_RELATIVE)
        val data =
            try {
                path.readBytes()
            } catch (error: Exception) {
                throw RestoreException("failed to read cloud-sync.key: ${error.message}")
            }
        Cas1.requireKey(data)
        return data
    }

    private fun loadInbox(inbox: File): Map<String, File> {
        if (!inbox.isDirectory) {
            throw RestoreException("replica inbox is missing")
        }
        val files = inbox.listFiles() ?: emptyArray()
        val objects = mutableMapOf<String, File>()
        for (file in files.filter { it.isFile && Cas1.isBlobId(it.name) }) {
            val blobId = file.name.lowercase()
            if (Cas1.sha256Hex(file.readBytes()) != blobId) {
                throw RestoreException("inbox blob id mismatch")
            }
            objects[blobId] = file
        }
        return objects
    }

    private fun loadManifest(
        key: ByteArray,
        objects: Map<String, File>,
    ): Pair<String, Manifest> {
        var found: Pair<String, Manifest>? = null
        var decryptFailures = 0
        for ((blobId, file) in objects) {
            val bytes = file.readBytes()
            val (kind, plain) =
                try {
                    Cas1.decrypt(key, bytes)
                } catch (_: RestoreException) {
                    decryptFailures += 1
                    continue
                }
            if (kind != ObjectKind.Manifest) {
                continue
            }
            if (found != null) {
                throw RestoreException("replica has multiple manifests")
            }
            val manifest =
                try {
                    json.decodeFromString<Manifest>(plain.toString(Charsets.UTF_8))
                } catch (_: Exception) {
                    throw RestoreException("replica manifest is not json")
                }
            found = blobId to manifest
        }
        if (found == null && objects.isNotEmpty() && decryptFailures == objects.size) {
            throw RestoreException("sync object decryption failed")
        }
        return found ?: throw RestoreException("replica manifest is missing")
    }

    private fun skipEntry(relative: String): Boolean {
        val normalized = relative.replace('\\', '/')
        return normalized == "hub.db-wal" || normalized == "hub.db-shm"
    }

    private fun isForbidden(relative: String): Boolean {
        val normalized = relative.replace('\\', '/').lowercase()
        if (normalized == KEY_RELATIVE || normalized.startsWith("keys/")) {
            return true
        }
        if (normalized == "sync/lock" || normalized.startsWith("sync/lock/")) {
            return true
        }
        return false
    }

    private fun stagingJoin(
        staging: File,
        relative: String,
    ): File {
        val normalized = relative.replace('\\', '/')
        if (normalized.startsWith("/") ||
            normalized.split('/').any { it == ".." || it.isEmpty() }
        ) {
            throw RestoreException("manifest path is not relative")
        }
        val dest = File(staging, normalized)
        val stagingCanon = staging.canonicalFile
        val destCanon = dest.canonicalFile
        val prefix = stagingCanon.path + File.separator
        if (destCanon.path != stagingCanon.path && !destCanon.path.startsWith(prefix)) {
            throw RestoreException("manifest path is not relative")
        }
        return dest
    }

    private fun writeLastVerified(
        home: File,
        schema: String?,
        base: String,
    ) {
        val syncDir = File(home, "sync")
        if (!syncDir.exists() && !syncDir.mkdirs()) {
            throw RestoreException("failed to write last-verified")
        }
        val body = json.encodeToString(LastVerified(schema ?: "0", base))
        File(syncDir, "last-verified.json").writeText(body)
    }
}
