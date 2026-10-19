import kotlinx.coroutines.*
import tech.second.bark.proof.*
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

private suspend fun waitUntil(condition: () -> Boolean) {
    withTimeout(5_000) {
        while (!condition()) delay(1)
    }
}

private suspend fun expectMessage(message: String, operation: suspend () -> Unit) {
    try {
        operation()
        error("expected WalletError: $message")
    } catch (error: WalletError) {
        check(error.message.contains(message)) { "wrong error: ${error.message}" }
    }
}

private class Callbacks : CustomOnchainWalletCallbacks {
    val failSync = AtomicBoolean(false)

    override fun getBalance(): ULong = ULong.MAX_VALUE

    override fun prepareTx(destinations: List<Destination>): String {
        when (destinations.first().address) {
            "typed" -> throw WalletError("declined by callback")
            "unexpected" -> throw IllegalStateException("unexpected foreign failure")
        }
        check(destinations.size == 1)
        check(destinations[0].amountSats == ULong.MAX_VALUE)
        return destinations[0].address
    }

    override fun sync() {
        if (failSync.get()) throw WalletError("sync callback failed")
    }
}

fun main() = runBlocking {
    withTimeout(30_000) {
        val logCount = AtomicInteger(0)
        val logger = BarkLogger { logCount.incrementAndGet(); Unit }
        check(echoAmount(ULong.MAX_VALUE) == ULong.MAX_VALUE)
        expectMessage("opening wallet: invalid datadir") {
            Wallet.open(WalletOpenArgs("fail"), logger)
        }

        Wallet.open(WalletOpenArgs("memory"), logger).use { plain ->
            check(plain.daemonRequested())
            check(plain.label() == null)
            check(plain.balance().confirmedSats == 0uL)
        }
        check(liveWallets() == 0u)

        val callbacks = Callbacks()
        val onchain = OnchainWallet.custom(callbacks)
        val wallet = Wallet.openWithOnchain(WalletOpenArgs("memory", false, "雪\u0000wallet"), logger, onchain)
        check(onchain.balance() == ULong.MAX_VALUE)
        onchain.close()
        wallet.use {
            check(!wallet.daemonRequested())
            check(wallet.label() == "雪\u0000wallet")
            check(wallet.balance().confirmedSats == ULong.MAX_VALUE)
            wallet.sync()
            callbacks.failSync.set(true)
            expectMessage("sync callback failed") { wallet.sync() }
            callbacks.failSync.set(false)
            val destination = Destination("雪\u0000destination", ULong.MAX_VALUE)
            check(wallet.prepareTx(listOf(destination)) == destination.address)
            expectMessage("declined by callback") { wallet.prepareTx(listOf(Destination("typed", 1uL))) }
            expectMessage("unexpected foreign failure") { wallet.prepareTx(listOf(Destination("unexpected", 1uL))) }

            wallet.notifications().use { holder ->
                wallet.emit(WalletNotification.MovementCreated(listOf(destination)))
                check(holder.nextNotification() == WalletNotification.MovementCreated(listOf(destination)))
                val explicitWait = async { holder.nextNotification() }
                waitUntil { holder.isWaiting() }
                expectMessage("concurrent notification wait") { holder.nextNotification() }
                holder.cancelNextNotificationWait()
                check(explicitWait.await() == null)

                val cancelledWait = async { holder.nextNotification() }
                waitUntil { holder.isWaiting() }
                cancelledWait.cancelAndJoin()
                waitUntil { !holder.isWaiting() }
                wallet.emit(WalletNotification.ChannelLagging)
                check(holder.nextNotification() == WalletNotification.ChannelLagging)
                wallet.close()
                check(holder.nextNotification() == null)
            }
        }
        check(liveWallets() == 0u)
        check(logCount.get() >= 3)
        println("PASS Kotlin: async factories, Tokio timer, defaults, u64, callbacks/errors, notifications, cancellation, close")
    }
}
