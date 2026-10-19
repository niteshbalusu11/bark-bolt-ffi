import io.github.niteshbalusu11.bark.*
import kotlinx.coroutines.*
import kotlin.test.*

class ContractTest {
    @Test fun mnemonicAndValidation() {
        val mnemonic = generateMnemonic()
        assertTrue(validateMnemonic(mnemonic))
        assertFalse(validateMnemonic("invalid"))
        assertEquals(250u, defaultVtxoKeyGapLimit())
        assertEquals(100000u, maxVtxoKeyGapLimit())
    }

    @org.junit.Test(timeout = 15000) fun realWalletNotificationCancellationAndReopen() = runBlocking<Unit> {
        org.junit.Assume.assumeTrue(System.getenv("BARK_REGTEST") == "1")
        withTimeout(15000) {
            val directory = java.nio.file.Files.createTempDirectory("bark-sdk-contract").toString()
            val mnemonic = generateMnemonic()
            val config = Config(serverAddress = "http://127.0.0.1:${System.getenv("ARK_PORT") ?: "3635"}", bitcoindAddress = "http://127.0.0.1:${System.getenv("BITCOIN_PORT") ?: "19443"}", bitcoindUser = "second", bitcoindPass = "ark", vtxoExitMargin = 2u)
            val args = WalletOpenArgs(runDaemon = false, datadir = directory, createWithoutServer = true, skipRecovery = true)
            val wallet = Wallet.open(Network.REGTEST, mnemonic, config, args)
            val fingerprint = wallet.fingerprint()
            assertEquals(config.serverAddress, wallet.config().serverAddress)
            val holder = wallet.notifications()
            try {
                val first = launch { holder.nextNotification() }
                delay(100)
                first.cancelAndJoin()
                val second = async(Dispatchers.Unconfined) { holder.nextNotification() }
                delay(100)
                holder.cancelNextNotificationWait()
                assertNull(second.await())
            } finally { holder.close(); wallet.close() }
            Wallet.open(Network.REGTEST, mnemonic, config, args).use { reopened ->
                assertEquals(fingerprint, reopened.fingerprint())
            }
        }
    }

    @Test fun callbackFullRangeErrorAndLifetime() = runBlocking<Unit> {
        val unexpected = java.util.concurrent.atomic.AtomicBoolean(false)
        var entered: java.util.concurrent.CountDownLatch? = null
        var released: java.util.concurrent.CountDownLatch? = null
        val callbacks = object : CustomOnchainWalletCallbacks {
            override fun getBalance(): ULong {
                entered?.countDown()
                released?.await(5, java.util.concurrent.TimeUnit.SECONDS)
                return ULong.MAX_VALUE
            }
            override fun prepareTx(destinations: List<Destination>, feeRateSatPerVb: ULong): String = error("unexpected prepare")
            override fun prepareDrainTx(address: String, feeRateSatPerVb: ULong): String = error("unexpected drain")
            override fun signPsbt(psbtBase64: String): String = error("unexpected sign")
            override fun finishPsbt(psbtBase64: String): String = error("unexpected finish")
            override fun isMine(scriptPubkeyHex: String): Boolean = false
            override fun registerTx(txHex: String) {}
            override fun evictTx(txid: String) {
                if (unexpected.get()) throw IllegalStateException("unexpected callback failure")
                throw BarkError("callback failure 🦀\u0000")
            }
            override fun makeSignedP2aCpfp(params: CpfpParams): String = error("unexpected cpfp")
            override fun storeSignedP2aCpfp(txHex: String) {}
            override fun sync() {}
        }
        val wallet = OnchainWallet.custom(callbacks)
        try {
            assertEquals(ULong.MAX_VALUE, wallet.balance().totalSats)
            val failure = assertFailsWith<BarkError> { wallet.evictTx("11".repeat(32)) }
            assertTrue(failure.message.contains("callback failure 🦀\u0000"))
            unexpected.set(true)
            assertTrue(assertFailsWith<BarkError> { wallet.evictTx("11".repeat(32)) }.message.contains("unexpected callback failure"))
            assertEquals(ULong.MAX_VALUE, wallet.balance().totalSats)
            entered = java.util.concurrent.CountDownLatch(1)
            released = java.util.concurrent.CountDownLatch(1)
            val pending = async(Dispatchers.Default) { wallet.balance() }
            withContext(Dispatchers.IO) { assertTrue(entered!!.await(5, java.util.concurrent.TimeUnit.SECONDS)) }
            wallet.close()
            released!!.countDown()
            assertEquals(ULong.MAX_VALUE, pending.await().totalSats)
        } finally { wallet.close() }
        assertFailsWith<IllegalStateException> { wallet.balance() }
    }
}
