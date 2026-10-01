package com.codingassistants.remotelauncher.sync

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.io.File
import kotlin.io.path.createTempDirectory

class SnapshotRestoreTest {
    private val key = ByteArray(32) { 0x11 }
    private val device = "11111111-1111-1111-1111-111111111111"

    @Test
    fun restoresVerifiedFilesIntoStagingOnly() {
        val home = newHome()
        plant(home, listOf(entry("notes/hello.txt", "hello", "markdown_export", ObjectKind.Blob)))
        File(home, "hub.db").writeText("live-db")
        val result = SnapshotRestore.restore(home)
        assertEquals(1, result.restored)
        assertTrue(result.warnings.contains(SnapshotRestore.LIVE_DB_WARNING))
        val staged = File(result.stagingDir, "notes/hello.txt")
        assertEquals("hello", staged.readText())
        assertFalse(File(home, "notes/hello.txt").exists())
        assertEquals("live-db", File(home, "hub.db").readText())
        assertFalse(File(home, "sync/lock").exists())
        val verified = File(home, "sync/last-verified.json").readText()
        assertTrue(verified.contains(result.baseBlobId))
    }

    @Test
    fun hubDbLandsInStagingNeverReplacingLive() {
        val home = newHome()
        plant(home, listOf(entry("hub.db", "snapshot-bytes", "hub_database", ObjectKind.Snapshot)))
        File(home, "hub.db").writeText("live")
        val result = SnapshotRestore.restore(home)
        assertEquals("snapshot-bytes", File(result.stagingDir, "hub.db").readText())
        assertEquals("live", File(home, "hub.db").readText())
        assertTrue(result.warnings.contains(SnapshotRestore.LIVE_DB_WARNING))
    }

    @Test
    fun wrongKeyLeavesPriorStaging() {
        val home = newHome()
        plant(home, listOf(entry("ok.txt", "ok", "shared_durable", ObjectKind.Blob)))
        SnapshotRestore.restore(home)
        File(home, SnapshotRestore.KEY_RELATIVE).writeBytes(ByteArray(32) { 0x22 })
        expectRestore("decryption") { SnapshotRestore.restore(home) }
        assertEquals("ok", File(home, "${SnapshotRestore.STAGING_RELATIVE}/ok.txt").readText())
    }

    @Test
    fun tamperedCiphertextFailsClosed() {
        val home = newHome()
        val planted = plant(home, listOf(entry("ok.txt", "ok", "shared_durable", ObjectKind.Blob)))
        val blob = planted.inbox.listFiles()!!.first { it.name != planted.manifestId }
        val bytes = blob.readBytes()
        bytes[bytes.lastIndex] = (bytes[bytes.lastIndex].toInt() xor 0xff).toByte()
        blob.writeBytes(bytes)
        expectRestore("decryption") { SnapshotRestore.restore(home) }
    }

    @Test
    fun traversalAndKeyPathsAreRefused() {
        val home = newHome()
        plant(home, listOf(entry("../escape.txt", "x", "shared_durable", ObjectKind.Blob)))
        expectRestore("relative") { SnapshotRestore.restore(home) }

        val home2 = newHome()
        plant(home2, listOf(entry("keys/cloud-sync.key", "stolen", "cloud_sync_key", ObjectKind.Blob)))
        expectRestore("restorable") { SnapshotRestore.restore(home2) }
    }

    @Test
    fun multipleManifestsAndMissingManifestFail() {
        val home = newHome()
        val keyFile = File(home, SnapshotRestore.KEY_RELATIVE)
        keyFile.parentFile!!.mkdirs()
        keyFile.writeBytes(key)
        val inbox = File(home, SnapshotRestore.INBOX_RELATIVE)
        inbox.mkdirs()
        writeObject(inbox, ObjectKind.Manifest, """{"format_version":1,"device_id":"$device","entries":[]}""")
        writeObject(inbox, ObjectKind.Manifest, """{"format_version":1,"device_id":"$device","entries":[]}""")
        expectRestore("multiple manifests") { SnapshotRestore.restore(home) }

        val empty = newHome()
        File(empty, SnapshotRestore.KEY_RELATIVE).apply {
            parentFile!!.mkdirs()
            writeBytes(key)
        }
        File(empty, SnapshotRestore.INBOX_RELATIVE).mkdirs()
        writeObject(File(empty, SnapshotRestore.INBOX_RELATIVE), ObjectKind.Blob, "nope")
        expectRestore("missing") { SnapshotRestore.restore(empty) }
    }

    @Test
    fun skipsNonHexInboxNamesAndDoesNotUpload() {
        val home = newHome()
        plant(home, listOf(entry("ok.txt", "ok", "shared_durable", ObjectKind.Blob)))
        File(home, "${SnapshotRestore.INBOX_RELATIVE}/readme.txt").writeText("ignore me")
        File(home, "${SnapshotRestore.INBOX_RELATIVE}/journals").mkdirs()
        val result = SnapshotRestore.restore(home)
        assertEquals(1, result.restored)
        val src = File("src/main/kotlin/com/codingassistants/remotelauncher/sync/SnapshotRestore.kt")
        assertTrue(src.isFile)
        val text = src.readText()
        assertFalse(text.contains("put_if_unmodified"))
        assertFalse(text.contains("DriveClient"))
        assertFalse(text.contains("HttpURLConnection"))
        assertFalse(text.contains("upload_home"))
        assertFalse(text.contains("reconcile"))
        assertFalse(text.contains("try_rebase"))
    }

    @Test
    fun kindMismatchFailsClosed() {
        val home = newHome()
        val item = entry("ok.txt", "ok", "shared_durable", ObjectKind.Blob)
        val homeFiles = plant(home, listOf(item))
        val manifestFile = File(homeFiles.inbox, homeFiles.manifestId)
        val wrong =
            """
            {"format_version":1,"device_id":"$device","entries":[{
              "blob_id":"${item.blobIdAfter(homeFiles)}",
              "relative_path":"ok.txt","category":"shared_durable",
              "content_hash":"${Cas1.sha256Hex("ok".toByteArray())}",
              "size":2,"kind":"snapshot"
            }]}
            """.trimIndent()
        val sealed = Cas1.encrypt(key, ObjectKind.Manifest, wrong.toByteArray())
        manifestFile.writeBytes(sealed)
        expectRestore("verification") { SnapshotRestore.restore(home) }
    }

    private fun newHome(): File {
        val dir = createTempDirectory("ca-s13-").toFile()
        dir.deleteOnExit()
        return dir
    }

    private data class Planted(
        val inbox: File,
        val manifestId: String,
        val blobIds: List<String>,
    )

    private data class PlannedEntry(
        val relative: String,
        val plaintext: String,
        val category: String,
        val kind: ObjectKind,
        var ciphertext: ByteArray? = null,
    ) {
        fun blobIdAfter(planted: Planted): String = planted.blobIds.first()
    }

    private fun entry(
        relative: String,
        plaintext: String,
        category: String,
        kind: ObjectKind,
    ): PlannedEntry = PlannedEntry(relative, plaintext, category, kind)

    private fun plant(
        home: File,
        entries: List<PlannedEntry>,
    ): Planted {
        val keyFile = File(home, SnapshotRestore.KEY_RELATIVE)
        keyFile.parentFile!!.mkdirs()
        keyFile.writeBytes(key)
        val inbox = File(home, SnapshotRestore.INBOX_RELATIVE)
        inbox.mkdirs()
        val blobIds = mutableListOf<String>()
        val manifestEntries = StringBuilder()
        entries.forEachIndexed { index, item ->
            val sealed = Cas1.encrypt(key, item.kind, item.plaintext.toByteArray())
            item.ciphertext = sealed
            val blobId = Cas1.sha256Hex(sealed)
            blobIds += blobId
            File(inbox, blobId).writeBytes(sealed)
            if (index > 0) manifestEntries.append(',')
            manifestEntries.append(
                """{"blob_id":"$blobId","relative_path":"${item.relative}","category":"${item.category}",""" +
                    """"content_hash":"${Cas1.sha256Hex(item.plaintext.toByteArray())}",""" +
                    """"size":${item.plaintext.toByteArray().size},"kind":"${item.kind.toWire()}"}""",
            )
        }
        val manifestJson =
            """{"format_version":1,"device_id":"$device","schema_version":"1","entries":[$manifestEntries]}"""
        val manifestId = writeObject(inbox, ObjectKind.Manifest, manifestJson)
        return Planted(inbox, manifestId, blobIds)
    }

    private fun writeObject(
        inbox: File,
        kind: ObjectKind,
        plaintext: String,
    ): String {
        val sealed = Cas1.encrypt(key, kind, plaintext.toByteArray())
        val blobId = Cas1.sha256Hex(sealed)
        File(inbox, blobId).writeBytes(sealed)
        return blobId
    }

    private fun expectRestore(
        needle: String,
        block: () -> Unit,
    ) {
        try {
            block()
            fail("expected RestoreException containing $needle")
        } catch (error: RestoreException) {
            assertTrue(
                "expected '$needle' in '${error.message}'",
                error.message!!.contains(needle),
            )
        }
    }
}
