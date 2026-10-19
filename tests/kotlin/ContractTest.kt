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

private val fixtureDirectory: java.io.File get() = java.io.File(System.getProperty("bark.fixtures", "tests/fixtures"))
private data class NitroVector(val index: UInt, val fields: Map<String,String>)
private fun cryptoFixture(): Triple<String,String,List<NitroVector>> {
    val text = java.io.File(fixtureDirectory,"nitro-v0.0.144-crypto.json").readText()
    fun string(name: String) = Regex("\"$name\": \"([^\"]+)\"").find(text)!!.groupValues[1]
    val vectors = Regex("\\{[^{}]*\"index\"[^{}]*}").findAll(text).map { match ->
        val index = Regex("\"index\": ([0-9]+)").find(match.value)!!.groupValues[1].toUInt()
        val fields = Regex("\"([A-Za-z]+)\": \"([^\"]+)\"").findAll(match.value).associate { it.groupValues[1] to it.groupValues[2] }
        NitroVector(index,fields)
    }.toList()
    return Triple(string("mnemonic"),string("message"),vectors)
}

class NoahContractTest {
    @Test fun offlineNitroGoldenVectors() {
        val (mnemonic,message,vectors) = cryptoFixture()
        assertEquals(6,vectors.size)
        for (network in Network.entries) for (v in vectors) {
            val key = derivePublicKeyFromMnemonic(mnemonic,network,v.index)
            val signature = signMessageWithMnemonic(message,mnemonic,network,v.index)
            assertEquals(v.fields["offlinePublicKey"],key)
            assertEquals(v.fields["offlineSignature"],signature)
            assertNotEquals(v.fields["walletPublicKey"],key)
            assertTrue(verifyMessage(message,signature,key))
            assertFalse(verifyMessage("$message!",signature,key))
        }
        assertFailsWith<BarkError> { derivePublicKeyFromMnemonic("invalid",Network.REGTEST,0u) }
        assertFailsWith<BarkError> { verifyMessage(message,"00","00") }
    }

    @Test fun schema42SnapshotWithoutWallet() = runBlocking<Unit> {
        val directory = java.nio.file.Files.createTempDirectory("bark-snapshot-contract").toFile()
        try {
            val file = java.io.File(directory,"db.sqlite")
            java.io.File(fixtureDirectory,"noah-v2-schema42.sqlite").copyTo(file)
            val before = file.readBytes()
            val info = validateWalletSnapshot(file.path,null)
            assertEquals(42u,info.schemaVersion)
            assertEquals("regtest",info.network)
            assertEquals(file.length().toULong(),info.sizeBytes)
            assertEquals("02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",info.mailboxPubkey)
            val digest = java.security.MessageDigest.getInstance("SHA-256").digest(before).joinToString("") { "%02x".format(it) }
            assertEquals(digest,info.sha256)
            assertContentEquals(before,file.readBytes())
            assertFailsWith<BarkError> { validateWalletSnapshot(file.path,WalletSnapshotExpectation(network="bitcoin")) }
        } finally { directory.deleteRecursively() }
    }

    @org.junit.Test(timeout=30000) fun walletNitroVectorsAndBackupObserver() = runBlocking<Unit> {
        org.junit.Assume.assumeTrue(System.getenv("BARK_REGTEST") == "1")
        withTimeout(25000) {
            val (mnemonic,message,vectors) = cryptoFixture()
            val directory = java.nio.file.Files.createTempDirectory("bark-noah-contract").toFile()
            val config = Config(serverAddress="http://127.0.0.1:${System.getenv("ARK_PORT") ?: "3635"}",bitcoindAddress="http://127.0.0.1:${System.getenv("BITCOIN_PORT") ?: "19443"}",bitcoindUser="second",bitcoindPass="ark")
            val args = WalletOpenArgs(runDaemon=false,datadir=directory.path,createWithoutServer=true,skipRecovery=true)
            try {
                Wallet.open(Network.REGTEST,mnemonic,config,args).use { wallet ->
                    assertFailsWith<BarkError> { wallet.peekPublicKey(0u) }
                    assertFailsWith<BarkError> { wallet.signMessage(message,0u) }
                    wallet.stateChanges().use { holder ->
                        assertTrue(holder.isActive())
                        assertEquals("initial",holder.nextChange()!!.reason)
                        val cancelled = async(Dispatchers.Unconfined) { holder.nextChange() }
                        delay(50)
                        assertFailsWith<BarkError> { holder.nextChange() }
                        holder.cancelNextChangeWait()
                        assertNull(cancelled.await())
                        val aborted = launch { holder.nextChange() }
                        delay(50); aborted.cancelAndJoin()
                        for (index in 0u..7u) {
                            val stored = wallet.deriveStoreNextKeypair()
                            assertEquals(index,stored.index)
                            vectors.find { it.index == index }?.let { v ->
                                assertEquals(v.fields["walletPublicKey"],stored.publicKey)
                                assertEquals(stored.publicKey,wallet.peekPublicKey(index))
                                val signature = wallet.signMessage(message,index)
                                assertEquals(v.fields["walletSignature"],signature)
                                assertTrue(verifyMessage(message,signature,stored.publicKey))
                                assertFalse(verifyMessage("$message!",signature,stored.publicKey))
                            }
                        }
                        assertEquals("databaseChanged",holder.nextChange()!!.reason)
                        val snapshot = wallet.createSnapshot(java.io.File(directory,"snapshot.sqlite").path)
                        assertEquals(snapshot.sha256,validateWalletSnapshot(snapshot.path,null).sha256)
                        assertFailsWith<BarkError> { wallet.createSnapshot(snapshot.path) }
                        assertFailsWith<BarkError> { validateWalletSnapshot(java.io.File(directory,"db.sqlite").path,null) }
                        val stopped = async(Dispatchers.Unconfined) { holder.nextChange() }
                        delay(50); holder.stop()
                        assertNull(stopped.await()); assertFalse(holder.isActive())
                    }
                }
                Wallet.open(Network.REGTEST,mnemonic,config,WalletOpenArgs(runDaemon=false,datadir=directory.path,createIfNotExists=false)).use { wallet ->
                    assertEquals(vectors.first().fields["walletPublicKey"],wallet.peekPublicKey(0u))
                }
            } finally { directory.deleteRecursively() }
        }
    }
}

private fun regtestCommand(vararg args: String): String {
    val script = java.io.File(fixtureDirectory.parentFile.parentFile,"scripts/ark-dev.sh")
    val process = ProcessBuilder(listOf("bash",script.path) + args).redirectError(java.lang.ProcessBuilder.Redirect.INHERIT).start()
    val output = process.inputStream.bufferedReader().readText().trim()
    check(process.waitFor() == 0) { "RegTest command failed" }
    return output
}

class NoahP1ContractTest {
    @Test fun pureVtxoDecoding() {
        val hex = java.io.File(fixtureDirectory,"board-vtxo.hex").readText().trim()
        val vtxo = decodeVtxo(hex)
        assertEquals(10000uL,vtxo.amountSats)
        assertTrue(vtxo.state is VtxoState.Unknown)
        assertFalse(vtxo.registered)
        assertEquals(66,vtxo.serverPubkey.length)
        assertEquals(2016u,vtxo.exitDelta)
        assertEquals(64,vtxo.anchorPoint.txid.length)
        assertEquals(64,vtxo.point.txid.length)
        val bytes = hex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
        assertEquals(vtxo.id,decodeVtxo(java.util.Base64.getEncoder().encodeToString(bytes)).id)
        assertFailsWith<BarkError> { decodeVtxo("invalid") }
        assertFailsWith<BarkError> { parseArkAddress("invalid") }
    }

    @org.junit.Test(timeout=90000) fun liveFeatureParity() = runBlocking<Unit> {
        org.junit.Assume.assumeTrue(System.getenv("BARK_REGTEST") == "1")
        val directory = java.nio.file.Files.createTempDirectory("bark-noah-p1").toFile()
        val config = Config(serverAddress="http://127.0.0.1:${System.getenv("ARK_PORT") ?: "3635"}",bitcoindAddress="http://127.0.0.1:${System.getenv("BITCOIN_PORT") ?: "19443"}",bitcoindUser="second",bitcoindPass="ark",vtxoExitMargin=2u)
        val mnemonic = generateMnemonic()
        try {
            OnchainWallet.default(Network.REGTEST,mnemonic,config,directory.path).use { onchain ->
                Wallet.openWithOnchain(Network.REGTEST,mnemonic,config,WalletOpenArgs(runDaemon=false,datadir=directory.path,createWithoutServer=true,skipRecovery=true),onchain).use { wallet ->
                    val address = onchain.newAddress()
                    assertTrue(onchain.isMine(address))
                    assertFailsWith<BarkError> { onchain.isMine("invalid") }
                    assertFailsWith<BarkError> { onchain.drain(address,ULong.MAX_VALUE) }
                    regtestCommand("send-to",address,"0.005")
                    onchain.sync()
                    val board = wallet.boardAmount(100000uL)
                    assertTrue(board.vtxoIds.isNotEmpty())
                    regtestCommand("generate","6"); wallet.sync()
                    val movement = wallet.history().first()
                    wallet.updateHistoryMetadata(movement.id,"{\"note\":\"Noah Kotlin\"}")
                    assertTrue(wallet.history().first { it.id==movement.id }.metadataJson.contains("Noah Kotlin"))
                    assertFailsWith<BarkError> { wallet.updateHistoryMetadata(movement.id,"[]") }
                    val recipientDirectory = java.io.File(directory,"recipient")
                    Wallet.open(Network.REGTEST,generateMnemonic(),config,WalletOpenArgs(runDaemon=false,datadir=recipientDirectory.path,createWithoutServer=true,skipRecovery=true)).use { recipient ->
                        val receiveAddress = recipient.newAddress()
                        wallet.sendArkoorPayment(receiveAddress,10000uL)
                        recipient.sync()
                        val received = recipient.history().flatMap { it.receivedOnAddresses }.filter { it.paymentMethod.value == receiveAddress }
                        assertEquals(10000uL,received.sumOf { it.amountSats })
                        assertTrue(received.all { it.paymentMethod.methodType.isNotEmpty() })
                    }
                    val arkAddress = wallet.newAddressWithIndex()
                    val parsed = parseArkAddress(arkAddress.address)
                    assertEquals(wallet.peekPublicKey(arkAddress.index),parsed.userPubkey)
                    assertTrue(parsed.arkId.isNotEmpty())
                    assertTrue(wallet.arkInfo()!!.mailboxPubkey.isNotEmpty())
                    val auth = wallet.mailboxAuthorization(authorizationExpiry=2000000000L)
                    assertEquals(2000000000L,auth.expiry)
                    assertEquals(auth.mailboxId,wallet.mailboxAuthorizationAt(2000000000L).mailboxId)
                    assertFailsWith<BarkError> { wallet.mailboxAuthorizationAt(Long.MAX_VALUE) }
                    assertTrue(wallet.syncPendingRounds().isEmpty())
                    val receive = wallet.bolt11Invoice(2000uL,"Noah parity",null)
                    val pending = wallet.lightningReceiveState(receive.paymentHash)
                    assertEquals(receive.invoice,pending.invoice)
                    assertEquals(2000uL,pending.amountSats)
                    assertTrue(pending.htlcVtxoIds.isEmpty())
                    assertNull(pending.movementId)
                    wallet.cancelLightningReceive(receive.paymentHash)
                    val id = wallet.spendableVtxos().first().id
                    wallet.startExitForVtxos(listOf(id))
                    val status = wallet.getExitStatus(id,true,true)!!
                    assertTrue(status.transactions.isNotEmpty())
                    assertEquals(status.transactionCount.toInt(),status.transactions.size)
                    assertTrue(status.transactions.all { it.parentTxHex.isNotEmpty() })
                    assertTrue(wallet.getExitStatus(id,false,false)!!.transactions.isEmpty())
                    wallet.cancelExit(id)
                    wallet.dangerousDropVtxo(id)
                    assertFailsWith<BarkError> { wallet.getVtxoById(id) }
                    assertFailsWith<BarkError> { wallet.dangerousDropVtxo("invalid") }
                    onchain.sync()
                    val destination = regtestCommand("bcli","getnewaddress")
                    assertFalse(onchain.isMine(destination))
                    val txid = onchain.drain(destination,null)
                    assertTrue(onchain.transactions().any { it.txid==txid })
                    regtestCommand("generate","1"); onchain.sync()
                    assertEquals(0uL,onchain.balance().totalSats)
                    regtestCommand("send-to",address,"0.002"); onchain.sync()
                    assertTrue(wallet.boardAll().vtxoIds.isNotEmpty())
                    regtestCommand("generate","6"); wallet.sync()
                }
            }
        } finally { directory.deleteRecursively() }
    }
}
