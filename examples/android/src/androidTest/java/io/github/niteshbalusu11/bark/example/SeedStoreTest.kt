package io.github.niteshbalusu11.bark.example

import androidx.test.core.app.ApplicationProvider
import android.content.Context
import org.junit.Test
import org.junit.Assert.*
import java.util.UUID

class SeedStoreTest {
    @Test fun samePhraseCanRetryAndDifferentPhraseCannotReplaceIt() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val account = "seed-test-${UUID.randomUUID()}"
        val other = "seed-test-${UUID.randomUUID()}"
        val store = SeedStore(context)
        try {
            val seed = store.loadOrCreate(account)
            assertTrue(store.loadOrCreate(account, seed) == seed)
            val different = store.loadOrCreate(other)
            try { store.loadOrCreate(account, different); fail("Different phrase must be rejected") }
            catch (_: IllegalArgumentException) { }
            assertTrue(store.loadOrCreate(account) == seed)
        } finally {
            context.getSharedPreferences("wallet-seeds", Context.MODE_PRIVATE).edit().remove(account).remove(other).commit()
        }
    }
}
