import Foundation
import BarkBoltFFI

@MainActor
final class WalletModel: ObservableObject {
    @Published var status = "Open your RegTest wallet"
    @Published var error: String?
    @Published var busy = false
    @Published var spendable: UInt64 = 0
    @Published var onchainBalance: UInt64 = 0
    @Published var arkAddress = ""
    @Published var onchainAddress = ""
    @Published var invoice = ""
    @Published var activity: [Movement] = []
    @Published var opened = false
    @Published var notificationsReceived = 0
    @Published var endpoint = "http://127.0.0.1:3635"
    @Published var bitcoinEndpoint = "http://127.0.0.1:19443"
    var wallet: Wallet?
    var onchain: OnchainWallet?
    var notification: NotificationHolder?
    var notificationTask: Task<Void, Never>?
    private var seed = ""
    private var directory = ""
    private let account: String

    init(account: String = "wallet") { self.account = account }

    func open(restore: String = "", restoredDirectory: Bool = false) async throws {
        if seed.isEmpty { seed = try SeedStore.loadOrCreate(account: account, restore: restore) }
        notification?.cancelNextNotificationWait()
        notificationTask?.cancel()
        if let wallet { try await wallet.stopDaemonWait() }
        notification = nil; wallet = nil; onchain = nil
        let root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        directory = root.appendingPathComponent(account + (restoredDirectory ? "-restored" : "")).path
        let config = Config(serverAddress: endpoint, bitcoindAddress: bitcoinEndpoint,
                            bitcoindUser: "second", bitcoindPass: "ark", vtxoExitMargin: 2,
                            fallbackFeeRate: 250, daemonSyncIntervalSecs: 2, offboardRequiredConfirmations: 1)
        let chain = try await OnchainWallet.default(network: .regtest, mnemonic: seed, config: config, datadir: directory)
        let openedWallet = try await Wallet.openWithOnchain(network: .regtest, mnemonicOrSeed: seed,
            config: config, args: WalletOpenArgs(datadir: directory), onchain: chain)
        self.onchain = chain; self.wallet = openedWallet
        if restoredDirectory { _ = try await chain.initialScan(birthdayHeight: 0) }
        arkAddress = try await openedWallet.newAddress()
        onchainAddress = try await chain.newAddress()
        opened = true
        try await sync()
        status = "Connected · RegTest"
        let holder = openedWallet.notifications()
        notification = holder
        notificationTask = Task { [weak self] in
            do {
                while !Task.isCancelled {
                    guard try await holder.nextNotification() != nil else { break }
                    guard let self else { break }
                    self.notificationsReceived += 1
                    try await self.readBalances()
                }
            } catch { if !Task.isCancelled { self?.error = error.localizedDescription } }
        }
    }

    func readBalances() async throws {
        guard let wallet, let onchain else { return }
        spendable = try await wallet.balance().spendableSats
        onchainBalance = try await onchain.balance().totalSats
        activity = try await wallet.history()
    }

    func sync() async throws {
        guard let wallet, let onchain else { throw AppError.message("Open a wallet first.") }
        _ = try await onchain.sync()
        try await wallet.sync()
        try await readBalances()
    }

    func perform(_ action: @escaping @MainActor () async throws -> Void) {
        guard !busy else { return }
        let previousStatus = status
        busy = true; error = nil
        Task {
            defer { busy = false }
            do {
                try await action(); if opened { try await readBalances() }
                if status == previousStatus { status = "Wallet updated · RegTest" }
            }
            catch let failure as BarkError { error = failure.message }
            catch { self.error = error.localizedDescription }
        }
    }

    func amount(_ text: String) throws -> UInt64 {
        guard let value = UInt64(text), value > 0 else { throw AppError.message("Enter a positive whole number of sats.") }
        return value
    }

    func send(destination: String, amount: String) async throws {
        guard let wallet else { throw AppError.message("Open a wallet first.") }
        let sats = try self.amount(amount)
        if destination.lowercased().hasPrefix("ln") {
            _ = try await wallet.payLightningInvoice(invoice: destination, amountSats: sats, wait: true)
        } else {
            try await wallet.sendArkoorPayment(arkAddress: destination, amountSats: sats)
        }
        status = "Payment sent"
    }

    #if DEBUG
    // Test orchestration uses the same SDK handles as the visible wallet.
    func automate(_ request: [String: Any]) async -> [String: Any] {
        do {
            endpoint = request["server"] as? String ?? endpoint
            bitcoinEndpoint = request["bitcoin"] as? String ?? bitcoinEndpoint
            if wallet == nil { try await open() }
            guard let wallet, let onchain else { throw AppError.message("Wallet unavailable") }
            let action = request["action"] as? String ?? "open"
            var extra: [String: Any] = [:]
            let sats = (request["amount"] as? NSNumber)?.uint64Value ?? 10000
            let destination = request["destination"] as? String ?? ""
            switch action {
            case "open": break
            case "sync": try await sync()
            case "board": extra["txid"] = try await wallet.boardAmount(amountSats: sats).txid
            case "send": try await send(destination: destination, amount: String(sats))
            case "invoice":
                let result = try await wallet.bolt11Invoice(amountSats: sats, description: "SDK RegTest", token: nil)
                extra["invoice"] = result.invoice; extra["paymentHash"] = result.paymentHash
            case "claim": _ = try await wallet.tryClaimAllLightningReceives(wait: true)
            case "refresh": extra["txid"] = try await wallet.refreshVtxos(vtxoIds: wallet.spendableVtxos().map(\.id)) ?? ""
            case "offboard": extra["txid"] = try await wallet.offboardVtxos(vtxoIds: wallet.spendableVtxos().prefix(1).map(\.id), bitcoinAddress: destination).txid
            case "estimate":
                let estimate = try await wallet.estimateEmergencyExitFee(vtxoIds: [], feeRateSatPerVb: 1, destination: onchainAddress)
                extra["totalFee"] = estimate.totalFeeSats
                extra["broadcastFee"] = estimate.exitBroadcastFeeSats
                extra["claimFee"] = estimate.claimFeeSats
            case "startExit": try await wallet.startExitForVtxos(vtxoIds: wallet.spendableVtxos().prefix(1).map(\.id))
            case "progressExit":
                _ = try await wallet.progressExits(feeRateSatPerVb: 1)
                extra["claimable"] = try await wallet.listClaimableExits().count
                extra["claimHeight"] = try await wallet.allExitsClaimableAtHeight().map { Int($0) } ?? -1
            case "drainExit":
                let claim = try await wallet.drainExits(vtxoIds: [], drainAll: true, address: destination, feeRateSatPerVb: 1)
                let signed = try await wallet.signExitClaimInputs(psbtBase64: claim.psbtBase64)
                extra["txid"] = try await wallet.broadcastTx(txHex: extractTxFromPsbt(psbtBase64: signed))
            case "restore": try await open(restoredDirectory: true)
            case "cancel":
                let holder = wallet.notifications()
                let wait = Task { try await holder.nextNotification() }
                try await Task.sleep(nanoseconds: 100_000_000)
                wait.cancel(); _ = try? await wait.value
                let again = Task { try await holder.nextNotification() }
                try await Task.sleep(nanoseconds: 100_000_000)
                holder.cancelNextNotificationWait(); _ = try await again.value
                extra["reused"] = true
            default: throw AppError.message("Unknown automation action")
            }
            if action == "board" {
                for _ in 0..<100 where notificationsReceived == 0 { try await Task.sleep(nanoseconds: 20_000_000) }
            }
            try await readBalances()
            return ["notifications": notificationsReceived, "ok": true, "arkAddress": arkAddress, "onchainAddress": onchainAddress,
                    "spendable": spendable, "onchain": onchainBalance, "historyCount": activity.count,
                    "fingerprint": self.wallet!.fingerprint()].merging(extra) { _, value in value }
        } catch let error as BarkError { return ["ok": false, "error": error.message] }
        catch { return ["ok": false, "error": error.localizedDescription] }
    }
    #endif
}
