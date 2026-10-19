package io.github.niteshbalusu11.bark.example

import android.app.Application
import androidx.compose.runtime.*
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import io.github.niteshbalusu11.bark.*
import kotlinx.coroutines.*
import org.json.JSONObject
import java.io.File

class WalletModel(application: Application) : AndroidViewModel(application) {
    var status by mutableStateOf("Open your RegTest wallet"); private set
    var error by mutableStateOf<String?>(null); private set
    var busy by mutableStateOf(false); private set
    var opened by mutableStateOf(false); private set
    var spendable by mutableStateOf(0uL); private set
    var onchainBalance by mutableStateOf(0uL); private set
    var arkAddress by mutableStateOf(""); private set
    var onchainAddress by mutableStateOf(""); private set
    var invoice by mutableStateOf(""); private set
    var activity by mutableStateOf<List<Movement>>(emptyList()); private set
    var endpoint by mutableStateOf("http://127.0.0.1:3635")
    var bitcoinEndpoint by mutableStateOf("http://127.0.0.1:19443")
    private var wallet: Wallet? = null
    private var onchain: OnchainWallet? = null
    private var notification: NotificationHolder? = null
    private var listen: Job? = null
    private var seed = ""
    var account = "wallet"

    suspend fun open(restore: String = "", restoredDirectory: Boolean = false) {
        if (seed.isEmpty()) seed = SeedStore(getApplication()).loadOrCreate(account, restore)
        listen?.cancel(); notification?.cancelNextNotificationWait(); notification?.close(); notification = null
        wallet?.stopDaemonWait(); wallet?.close(); wallet = null; onchain?.close(); onchain = null
        val directory = File(getApplication<Application>().filesDir, account + if (restoredDirectory) "-restored" else "").path
        val config = Config(serverAddress = endpoint, bitcoindAddress = bitcoinEndpoint,
            bitcoindUser = "second", bitcoindPass = "ark", vtxoExitMargin = 2u,
            fallbackFeeRate = 250u, daemonSyncIntervalSecs = 2u, offboardRequiredConfirmations = 1u)
        val chain = OnchainWallet.default(Network.REGTEST, seed, config, directory)
        val ark = try { Wallet.openWithOnchain(Network.REGTEST, seed, config, WalletOpenArgs(datadir = directory), chain) }
                  catch (failure: Throwable) { chain.close(); throw failure }
        onchain = chain; wallet = ark
        if (restoredDirectory) chain.initialScan(0u)
        arkAddress = ark.newAddress(); onchainAddress = chain.newAddress()
        opened = true; sync(); status = "Connected · RegTest"
        val holder = ark.notifications(); notification = holder
        listen = viewModelScope.launch {
            try { while (isActive && holder.nextNotification() != null) readBalances() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message }
        }
    }

    private suspend fun readBalances() {
        spendable = wallet?.balance()?.spendableSats ?: 0u
        onchainBalance = onchain?.balance()?.totalSats ?: 0u
        activity = wallet?.history() ?: emptyList()
    }
    suspend fun sync() { checkNotNull(onchain).sync(); checkNotNull(wallet).sync(); readBalances() }
    fun amount(text: String): ULong = requireNotNull(text.toULongOrNull()?.takeIf { it > 0uL }) { "Enter a positive whole number of sats." }
    fun perform(action: suspend () -> Unit) {
        if (busy) return
        busy = true; error = null
        viewModelScope.launch {
            try { action(); if (opened) readBalances() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Operation failed. Try again." }
            finally { busy = false }
        }
    }
    suspend fun send(destination: String, amount: String) {
        val ark = checkNotNull(wallet)
        if (destination.startsWith("ln", ignoreCase = true)) ark.payLightningInvoice(destination, amount(amount), true)
        else ark.sendArkoorPayment(destination, amount(amount))
        status = "Payment sent"
    }
    suspend fun board(text: String) { checkNotNull(wallet).boardAmount(amount(text)); status = "Board submitted. Mine a block, then sync." }
    suspend fun createInvoice(text: String) { invoice = checkNotNull(wallet).bolt11Invoice(amount(text), "Bark example").invoice }
    suspend fun claim() { checkNotNull(wallet).tryClaimAllLightningReceives(true) }
    suspend fun refresh() { val ark = checkNotNull(wallet); ark.refreshVtxos(ark.spendableVtxos().map { it.id }) }
    suspend fun offboard() { checkNotNull(wallet).offboardAll(onchainAddress) }
    suspend fun startExit() { checkNotNull(wallet).startExitForEntireWallet() }
    suspend fun progressExit() { checkNotNull(wallet).progressExits(null) }
    suspend fun drainExit() {
        val ark = checkNotNull(wallet)
        val claim = ark.drainExits(emptyList(), true, onchainAddress, null)
        val signed = ark.signExitClaimInputs(claim.psbtBase64)
        ark.broadcastTx(extractTxFromPsbt(signed))
    }

    suspend fun automate(request: JSONObject): JSONObject {
        check(BuildConfig.DEBUG) { "Automation requires a debug build" }
        return try {
            endpoint = request.optString("server", endpoint)
            bitcoinEndpoint = request.optString("bitcoin", bitcoinEndpoint)
            if (wallet == null) open()
            val ark = checkNotNull(wallet)
            val sats = request.optLong("amount", 10000).toULong()
            val destination = request.optString("destination")
            val result = JSONObject()
            when (request.optString("action", "open")) {
                "open" -> Unit
                "sync" -> sync()
                "board" -> result.put("txid", ark.boardAmount(sats).txid)
                "send" -> send(destination, sats.toString())
                "invoice" -> { val invoice = ark.bolt11Invoice(sats, "SDK RegTest"); result.put("invoice", invoice.invoice).put("paymentHash", invoice.paymentHash) }
                "claim" -> claim()
                "refresh" -> result.put("txid", ark.refreshVtxos(ark.spendableVtxos().map { it.id }) ?: "")
                "offboard" -> result.put("txid", ark.offboardVtxos(ark.spendableVtxos().take(1).map { it.id }, destination).txid)
                "startExit" -> ark.startExitForVtxos(ark.spendableVtxos().take(1).map { it.id })
                "progressExit" -> { ark.progressExits(1u); result.put("claimable", ark.listClaimableExits().size).put("claimHeight", ark.allExitsClaimableAtHeight()?.toLong() ?: -1) }
                "drainExit" -> {
                    val claim = ark.drainExits(emptyList(), true, destination, 1u)
                    result.put("txid", ark.broadcastTx(extractTxFromPsbt(ark.signExitClaimInputs(claim.psbtBase64))))
                }
                "restore" -> open(restoredDirectory = true)
                "cancel" -> {
                    val holder = ark.notifications()
                    try {
                        coroutineScope {
                            val first = async { holder.nextNotification() }; delay(100); first.cancelAndJoin()
                            val second = async { holder.nextNotification() }; delay(100); holder.cancelNextNotificationWait(); second.await()
                        }
                        result.put("reused", true)
                    } finally { holder.close() }
                }
                else -> error("Unknown automation action")
            }
            readBalances()
            result.put("ok", true).put("arkAddress", arkAddress).put("onchainAddress", onchainAddress)
                .put("spendable", spendable.toLong()).put("onchain", onchainBalance.toLong())
                .put("historyCount", activity.size).put("fingerprint", checkNotNull(wallet).fingerprint())
        } catch (cancelled: CancellationException) { throw cancelled }
          catch (failure: Exception) { JSONObject().put("ok", false).put("error", failure.message) }
    }

    override fun onCleared() {
        listen?.cancel(); notification?.cancelNextNotificationWait(); notification?.close(); wallet?.close(); onchain?.close()
    }
}
