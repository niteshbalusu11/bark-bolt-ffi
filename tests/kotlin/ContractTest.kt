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

    @Test fun callbackFullRangeErrorAndLifetime() = runBlocking<Unit> {
        val callbacks = object : CustomOnchainWalletCallbacks {
            override fun getBalance(): ULong = ULong.MAX_VALUE
            override fun prepareTx(destinations: List<Destination>, feeRateSatPerVb: ULong): String = error("unexpected prepare")
            override fun prepareDrainTx(address: String, feeRateSatPerVb: ULong): String = error("unexpected drain")
            override fun finishPsbt(psbtBase64: String): String = error("unexpected finish")
            override fun isMine(scriptPubkeyHex: String): Boolean = false
            override fun registerTx(txHex: String) {}
            override fun evictTx(txid: String) { throw BarkError("callback failure 🦀\u0000") }
            override fun makeSignedP2aCpfp(params: CpfpParams): String = error("unexpected cpfp")
            override fun storeSignedP2aCpfp(txHex: String) {}
            override fun sync() {}
        }
        val wallet = OnchainWallet.custom(callbacks)
        try {
            assertEquals(ULong.MAX_VALUE, wallet.balance().totalSats)
            val failure = assertFailsWith<BarkError> { wallet.evictTx("11".repeat(32)) }
            assertTrue(failure.message.contains("callback failure 🦀\u0000"))
            assertEquals(ULong.MAX_VALUE, wallet.balance().totalSats)
        } finally { wallet.close() }
        assertFailsWith<IllegalStateException> { wallet.balance() }
    }
}
