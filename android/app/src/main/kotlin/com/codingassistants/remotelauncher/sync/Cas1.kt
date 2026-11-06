package com.codingassistants.remotelauncher.sync

import org.bouncycastle.crypto.InvalidCipherTextException
import org.bouncycastle.crypto.modes.ChaCha20Poly1305
import org.bouncycastle.crypto.params.AEADParameters
import org.bouncycastle.crypto.params.KeyParameter
import java.security.MessageDigest
import java.security.SecureRandom

/** CAS1 object kind byte, matching `hub::sync::types::ObjectKind`. */
enum class ObjectKind(
    val code: Byte,
) {
    Blob(0),
    Manifest(1),
    Snapshot(2),
    ;

    fun toWire(): String = name.lowercase()

    companion object {
        fun fromCode(code: Byte): ObjectKind =
            entries.firstOrNull { it.code == code }
                ?: throw RestoreException("unknown object kind")

        fun fromWire(name: String): ObjectKind =
            when (name) {
                "blob" -> Blob
                "manifest" -> Manifest
                "snapshot" -> Snapshot
                else -> throw RestoreException("unknown object kind")
            }
    }
}

class RestoreException(
    message: String,
) : Exception(message)

/**
 * Portable CAS1 codec (S2). Header is `CAS1` || version || kind || 12-byte nonce;
 * AAD is the first 6 bytes. Cipher is IETF ChaCha20-Poly1305.
 *
 * The companion only decrypts. [encrypt] exists so tests can mint fixtures; restore
 * never uploads.
 */
object Cas1 {
    val MAGIC: ByteArray = byteArrayOf(0x43, 0x41, 0x53, 0x31)
    const val VERSION: Byte = 1
    const val NONCE_LEN: Int = 12
    const val TAG_LEN: Int = 16
    const val HEADER_LEN: Int = 4 + 1 + 1 + NONCE_LEN
    const val AAD_LEN: Int = 6
    const val KEY_LEN: Int = 32

    fun requireKey(key: ByteArray) {
        if (key.size != KEY_LEN) {
            throw RestoreException("cloud-sync.key must be 32 bytes")
        }
    }

    fun decrypt(
        key: ByteArray,
        data: ByteArray,
    ): Pair<ObjectKind, ByteArray> {
        requireKey(key)
        if (data.size < HEADER_LEN + TAG_LEN) {
            throw RestoreException("sync object is truncated")
        }
        if (!data.copyOfRange(0, 4).contentEquals(MAGIC)) {
            throw RestoreException("sync object magic mismatch")
        }
        if (data[4] != VERSION) {
            throw RestoreException("unsupported sync object version")
        }
        val kind = ObjectKind.fromCode(data[5])
        val nonce = data.copyOfRange(6, HEADER_LEN)
        val aad = data.copyOfRange(0, AAD_LEN)
        val payload = data.copyOfRange(HEADER_LEN, data.size)
        return kind to open(key, nonce, aad, payload)
    }

    internal fun encrypt(
        key: ByteArray,
        kind: ObjectKind,
        plaintext: ByteArray,
        nonce: ByteArray = freshNonce(),
    ): ByteArray {
        requireKey(key)
        if (nonce.size != NONCE_LEN) {
            throw RestoreException("object nonce must be 12 bytes")
        }
        val header = ByteArray(HEADER_LEN)
        MAGIC.copyInto(header, 0)
        header[4] = VERSION
        header[5] = kind.code
        nonce.copyInto(header, 6)
        val aad = header.copyOfRange(0, AAD_LEN)
        val sealed = seal(key, nonce, aad, plaintext)
        return header + sealed
    }

    fun sha256Hex(bytes: ByteArray): String {
        val digest = MessageDigest.getInstance("SHA-256").digest(bytes)
        return toHex(digest)
    }

    fun toHex(bytes: ByteArray): String = bytes.joinToString("") { b -> "%02x".format(b.toInt() and 0xff) }

    fun isBlobId(name: String): Boolean = name.length == 64 && name.all { it.isHex() }

    private fun freshNonce(): ByteArray {
        val nonce = ByteArray(NONCE_LEN)
        SecureRandom().nextBytes(nonce)
        return nonce
    }

    private fun open(
        key: ByteArray,
        nonce: ByteArray,
        aad: ByteArray,
        sealed: ByteArray,
    ): ByteArray {
        val cipher = ChaCha20Poly1305()
        cipher.init(false, AEADParameters(KeyParameter(key), TAG_LEN * 8, nonce, aad))
        val out = ByteArray(cipher.getOutputSize(sealed.size))
        return try {
            val n = cipher.processBytes(sealed, 0, sealed.size, out, 0)
            val m = cipher.doFinal(out, n)
            out.copyOf(n + m)
        } catch (_: InvalidCipherTextException) {
            throw RestoreException("sync object decryption failed")
        }
    }

    private fun seal(
        key: ByteArray,
        nonce: ByteArray,
        aad: ByteArray,
        plaintext: ByteArray,
    ): ByteArray {
        val cipher = ChaCha20Poly1305()
        cipher.init(true, AEADParameters(KeyParameter(key), TAG_LEN * 8, nonce, aad))
        val out = ByteArray(cipher.getOutputSize(plaintext.size))
        val n = cipher.processBytes(plaintext, 0, plaintext.size, out, 0)
        val m = cipher.doFinal(out, n)
        return out.copyOf(n + m)
    }
}

private fun Char.isHex(): Boolean = this in '0'..'9' || this in 'a'..'f' || this in 'A'..'F'
