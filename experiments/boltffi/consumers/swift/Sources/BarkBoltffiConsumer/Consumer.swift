import BarkBoltffiProof
import Foundation

struct CheckFailed: Error {
    let message: String
}

func require(_ condition: Bool, _ message: String) throws {
    if !condition { throw CheckFailed(message: message) }
}

func waitUntil(_ condition: () -> Bool) async throws {
    for _ in 0..<1_000 {
        if condition() { return }
        try await Task.sleep(nanoseconds: 1_000_000)
    }
    throw CheckFailed(message: "timed out waiting for native state")
}

func expectMessage(_ message: String, _ operation: () async throws -> Void) async throws {
    do {
        try await operation()
        throw CheckFailed(message: "expected WalletError: \(message)")
    } catch let error as WalletError {
        try require(error.message.contains(message), "wrong error: \(error.message)")
    }
}

final class Logger: BarkLogger {
    private let lock = NSLock()
    private var messages: [String] = []

    func log(message: String) {
        lock.lock()
        messages.append(message)
        lock.unlock()
    }

    var count: Int {
        lock.lock()
        defer { lock.unlock() }
        return messages.count
    }
}

final class Callbacks: CustomOnchainWalletCallbacks {
    var failSync = false

    func getBalance() throws -> UInt64 { UInt64.max }

    func prepareTx(destinations: [Destination]) throws -> String {
        switch destinations.first?.address {
        case "typed": throw WalletError(message: "declined by callback")
        case "unexpected": throw NSError(domain: "proof", code: 1, userInfo: [
            NSLocalizedDescriptionKey: "unexpected foreign failure"
        ])
        default:
            try require(destinations.count == 1, "callback lost vector arguments")
            try require(destinations[0].amountSats == UInt64.max, "callback truncated u64")
            return destinations[0].address
        }
    }

    func sync() throws {
        if failSync { throw WalletError(message: "sync callback failed") }
    }
}

@main
struct Consumer {
    static func main() async throws {
        let logger = Logger()
        try require(echoAmount(amount: UInt64.max) == UInt64.max, "u64 round trip")

        try await expectMessage("opening wallet: invalid datadir") {
            _ = try await Wallet.open(args: WalletOpenArgs(datadir: "fail"), logger: logger)
        }

        var plain: Wallet? = try await Wallet.open(
            args: WalletOpenArgs(datadir: "memory"), logger: logger
        )
        try require(plain!.daemonRequested(), "record boolean default")
        try require(plain!.label() == nil, "optional record default")
        try require(try await plain!.balance().confirmedSats == 0, "plain factory")
        plain = nil
        try require(liveWallets() == 0, "plain wallet not released")

        var callbacks: Callbacks? = Callbacks()
        weak var weakCallbacks = callbacks
        var onchain: OnchainWallet? = OnchainWallet(custom: callbacks!)
        var wallet: Wallet? = try await Wallet.openWithOnchain(
            args: WalletOpenArgs(datadir: "memory", runDaemon: false, label: "雪\u{0}wallet"),
            logger: logger,
            onchain: onchain!
        )
        try require(try onchain!.balance() == UInt64.max, "borrow consumed onchain object")
        onchain = nil
        callbacks = nil
        try require(weakCallbacks != nil, "native wallet did not retain callback")
        try require(!wallet!.daemonRequested(), "explicit boolean option")
        try require(wallet!.label() == "雪\u{0}wallet", "optional UTF-8/NUL string")
        try require(try await wallet!.balance().confirmedSats == UInt64.max, "async callback balance")
        try await wallet!.sync()
        weakCallbacks!.failSync = true
        try await expectMessage("sync callback failed") { try await wallet!.sync() }
        weakCallbacks!.failSync = false

        let destination = Destination(address: "雪\u{0}destination", amountSats: UInt64.max)
        try require(try await wallet!.prepareTx(destinations: [destination]) == destination.address,
                    "callback record/vector/string round trip")
        try await expectMessage("declined by callback") {
            _ = try await wallet!.prepareTx(destinations: [Destination(address: "typed", amountSats: 1)])
        }
        try await expectMessage("unexpected foreign failure") {
            _ = try await wallet!.prepareTx(destinations: [Destination(address: "unexpected", amountSats: 1)])
        }

        let holder = wallet!.notifications()
        wallet!.emit(notification: .movementCreated(destinations: [destination]))
        let event = try await holder.nextNotification()
        try require(event == .movementCreated(destinations: [destination]), "notification payload")

        let explicitWait = Task { try await holder.nextNotification() }
        try await waitUntil { holder.isWaiting() }
        try await expectMessage("concurrent notification wait") {
            _ = try await holder.nextNotification()
        }
        holder.cancelNextNotificationWait()
        try require(try await explicitWait.value == nil, "explicit cancel result")

        let cancelledWait = Task { try await holder.nextNotification() }
        try await waitUntil { holder.isWaiting() }
        cancelledWait.cancel()
        do { _ = try await cancelledWait.value } catch is CancellationError {}
        try await waitUntil { !holder.isWaiting() }
        wallet!.emit(notification: .channelLagging)
        try require(try await holder.nextNotification() == .channelLagging, "reuse after task cancellation")

        wallet = nil
        try require(liveWallets() == 0, "wallet not released")
        try require(weakCallbacks == nil, "native callback leaked")
        try require(try await holder.nextNotification() == nil, "stream did not close on wallet drop")
        try require(logger.count >= 3, "logger callback not delivered")
        print("PASS Swift: async factories, Tokio timer, defaults, u64, callbacks/errors, notifications, cancellation, release")
    }
}
