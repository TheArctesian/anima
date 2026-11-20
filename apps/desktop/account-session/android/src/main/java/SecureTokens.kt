package ing.anima.accountsession

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.io.File
import java.io.FileNotFoundException
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

internal class SecureTokens(context: Context) {
    private val directory = File(context.noBackupFilesDir, "sessions")

    @Synchronized
    fun read(scope: String, account: String): String? {
        val entry = entry(scope, account)
        val bytes = try {
            file(entry).readFully()
        } catch (_: FileNotFoundException) {
            return null
        }
        require(bytes.size >= NONCE_BYTES + TAG_BITS / 8)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, checkNotNull(keyStore().getKey(ALIAS, null)),
            GCMParameterSpec(TAG_BITS, bytes, 0, NONCE_BYTES))
        cipher.updateAAD(entry)
        val token = cipher.doFinal(bytes, NONCE_BYTES, bytes.size - NONCE_BYTES)
        return try {
            token.toString(Charsets.UTF_8)
        } finally {
            token.fill(0)
        }
    }

    @Synchronized
    fun write(scope: String, account: String, token: String) {
        require(token.isNotEmpty())
        val entry = entry(scope, account)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        val key = keyStore().getKey(ALIAS, null) ?: createKey()
        cipher.init(Cipher.ENCRYPT_MODE, key)
        check(cipher.iv.size == NONCE_BYTES)
        cipher.updateAAD(entry)
        val plain = token.toByteArray(Charsets.UTF_8)
        val bytes = try {
            cipher.iv + cipher.doFinal(plain)
        } finally {
            plain.fill(0)
        }
        val file = file(entry)
        val stream = file.startWrite()
        try {
            stream.write(bytes)
            file.finishWrite(stream)
        } catch (error: Exception) {
            file.failWrite(stream)
            throw error
        }
    }

    @Synchronized
    fun clear(scope: String, account: String) {
        val file = file(entry(scope, account))
        file.delete()
        check(!file.baseFile.exists())
    }

    private fun entry(scope: String, account: String) =
        "$scope\u0000$account".toByteArray(Charsets.UTF_8)

    private fun file(entry: ByteArray): AtomicFile {
        val name = MessageDigest.getInstance("SHA-256").digest(entry)
            .joinToString("") { "%02x".format(it) }
        return AtomicFile(File(directory, name))
    }

    private fun keyStore() = KeyStore.getInstance(PROVIDER).apply { load(null) }

    private fun createKey(): SecretKey = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        .apply {
            init(KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build())
        }.generateKey()

    private companion object {
        const val PROVIDER = "AndroidKeyStore"
        const val ALIAS = "anima-mobile-sessions-v1"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val NONCE_BYTES = 12
        const val TAG_BITS = 128
    }
}
