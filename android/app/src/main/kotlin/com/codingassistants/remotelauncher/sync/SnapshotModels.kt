package com.codingassistants.remotelauncher.sync

import kotlinx.serialization.Serializable

/** Wire shape of `hub::sync::types::Manifest` (serde snake_case). */
@Serializable
data class Manifest(
    val format_version: Int,
    val device_id: String,
    val schema_version: String? = null,
    val entries: List<ManifestEntry> = emptyList(),
)

@Serializable
data class ManifestEntry(
    val blob_id: String,
    val relative_path: String,
    val category: String,
    val content_hash: String,
    val size: Long,
    val kind: String,
)

data class RestoreResult(
    val restored: Int,
    val warnings: List<String>,
    val stagingDir: java.io.File,
    val deviceId: String,
    val schemaVersion: String?,
    val baseBlobId: String,
)
