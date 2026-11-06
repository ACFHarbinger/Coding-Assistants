package com.codingassistants.remotelauncher.sync

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test

class Cas1Test {
    private val key = ByteArray(32) { 0x11 }
    private val nonce = ByteArray(12) { 0x22 }

    @Test
    fun knownVectorDecryptsHello() {
        val hex = "434153310100222222222222222222222222ed10b97252d7cd4bdb1f600abd332880c9af9bc5f0"
        val data = hexToBytes(hex)
        val (kind, plain) = Cas1.decrypt(key, data)
        assertEquals(ObjectKind.Blob, kind)
        assertArrayEquals("hello".toByteArray(), plain)
    }

    @Test
    fun encryptMatchesKnownVectorWithFixedNonce() {
        val sealed = Cas1.encrypt(key, ObjectKind.Blob, "hello".toByteArray(), nonce)
        val hex = Cas1.toHex(sealed)
        assertEquals(
            "434153310100222222222222222222222222ed10b97252d7cd4bdb1f600abd332880c9af9bc5f0",
            hex,
        )
    }

    @Test
    fun roundtripUsesFreshNonce() {
        val a = Cas1.encrypt(key, ObjectKind.Blob, "hello".toByteArray())
        val b = Cas1.encrypt(key, ObjectKind.Blob, "hello".toByteArray())
        assertNotEquals(a.toList(), b.toList())
        assertTrue(a.copyOfRange(0, 4).contentEquals(Cas1.MAGIC))
        val (kind, plain) = Cas1.decrypt(key, a)
        assertEquals(ObjectKind.Blob, kind)
        assertArrayEquals("hello".toByteArray(), plain)
    }

    @Test
    fun truncatedOrWrongMagicFailsClosed() {
        expectRestore("truncated") { Cas1.decrypt(key, ByteArray(10)) }
        val good = Cas1.encrypt(key, ObjectKind.Manifest, "{}".toByteArray())
        good[0] = 0x00
        expectRestore("magic") { Cas1.decrypt(key, good) }
    }

    @Test
    fun wrongKeyOrTamperFailsClosed() {
        val sealed = Cas1.encrypt(key, ObjectKind.Manifest, "{}".toByteArray())
        val other = ByteArray(32) { 0x22 }
        expectRestore("decryption") { Cas1.decrypt(other, sealed) }
        sealed[sealed.lastIndex] = (sealed[sealed.lastIndex].toInt() xor 0xff).toByte()
        expectRestore("decryption") { Cas1.decrypt(key, sealed) }
    }

    @Test
    fun unknownKindFailsClosed() {
        val sealed = Cas1.encrypt(key, ObjectKind.Blob, "x".toByteArray())
        sealed[5] = 9
        expectRestore("kind") { Cas1.decrypt(key, sealed) }
    }

    @Test
    fun blobIdHelper() {
        assertTrue(Cas1.isBlobId("a".repeat(64)))
        assertFalse(Cas1.isBlobId("zz".repeat(32)))
        assertFalse(Cas1.isBlobId("abc"))
    }

    private fun expectRestore(
        needle: String,
        block: () -> Unit,
    ) {
        try {
            block()
            fail("expected RestoreException containing $needle")
        } catch (error: RestoreException) {
            assertTrue(error.message!!.contains(needle))
        }
    }

    private fun hexToBytes(hex: String): ByteArray {
        return ByteArray(hex.length / 2) { i ->
            hex.substring(i * 2, i * 2 + 2).toInt(16).toByte()
        }
    }
}
