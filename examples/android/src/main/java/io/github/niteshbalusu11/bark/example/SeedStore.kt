package io.github.niteshbalusu11.bark.example

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import io.github.niteshbalusu11.bark.generateMnemonic
import io.github.niteshbalusu11.bark.validateMnemonic
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class SeedStore(context: Context) {
    private val preferences = context.getSharedPreferences("wallet-seeds", Context.MODE_PRIVATE)
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val alias = "bark-wallet-seed"
        if (store.containsAlias(alias)) return store.getKey(alias, null) as SecretKey
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }
    fun loadOrCreate(account: String, restore: String = ""): String {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        val stored = preferences.getString(account, null)
        if (stored != null) {
            require(restore.isBlank()) { "A wallet already exists. Open it to preserve its funds." }
            val bytes = Base64.decode(stored, Base64.NO_WRAP)
            cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, bytes.copyOfRange(0, 12)))
            return cipher.doFinal(bytes.copyOfRange(12, bytes.size)).toString(Charsets.UTF_8)
        }
        val seed = if (restore.isBlank()) generateMnemonic() else restore.trim()
        require(validateMnemonic(seed)) { "Enter a valid recovery phrase." }
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val encrypted = cipher.iv + cipher.doFinal(seed.toByteArray(Charsets.UTF_8))
        check(preferences.edit().putString(account, Base64.encodeToString(encrypted, Base64.NO_WRAP)).commit()) { "Could not save the recovery phrase." }
        return seed
    }
}
