import SwiftUI
import BarkBoltFFI

@main
struct BarkExampleApp: App {
    @StateObject private var model = WalletModel(account: ProcessInfo.processInfo.environment["BARK_RUN_ID"] ?? "wallet")
    var body: some Scene {
        WindowGroup {
            WalletView(model: model)
                .task {
                    #if DEBUG
                    if let command = ProcessInfo.processInfo.environment["BARK_COMMAND"],
                       let data = command.data(using: .utf8),
                       let request = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                        let result = await model.automate(request)
                        if let output = try? JSONSerialization.data(withJSONObject: result, options: [.sortedKeys]) {
                            let file = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("automation-result.json")
                            try? output.write(to: file, options: .atomic)
                            model.status = (result["ok"] as? Bool == true) ? "Test completed · RegTest" : "Test failed · RegTest"
                            model.error = result["error"] as? String
                        }
                    }
                    #endif
                }
        }
    }
}

struct WalletView: View {
    @ObservedObject var model: WalletModel
    @State private var restore = ""
    @State private var destination = ""
    @State private var amount = "10000"
    @State private var boardAmount = "1000000"
    @State private var confirmExit = false

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Label("RegTest · local funds only", systemImage: "network")
                    Text(model.status).foregroundStyle(.secondary).accessibilityIdentifier("connection-status")
                    if model.busy { ProgressView("Working…") }
                    if let error = model.error { Text(error).foregroundStyle(.red).textSelection(.enabled).accessibilityIdentifier("wallet-error") }
                }
                if !model.opened {
                    Section("Connect") {
                        TextField("Ark server", text: $model.endpoint).keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled()
                        TextField("Bitcoin RPC", text: $model.bitcoinEndpoint).keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled()
                        SecureField("Recovery phrase (optional)", text: $restore).textInputAutocapitalization(.never).autocorrectionDisabled()
                        Button("Create or open wallet") { model.perform { try await model.open(restore: restore); restore = "" } }
                            .accessibilityIdentifier("open-wallet")
                    }
                } else {
                    Section("Wallet") {
                        LabeledContent("Available", value: "\(model.spendable) sats").accessibilityIdentifier("spendable-balance")
                        LabeledContent("Onchain", value: "\(model.onchainBalance) sats")
                        Button { model.perform { try await model.sync() } } label: { Label("Sync wallet", systemImage: "arrow.clockwise") }.accessibilityIdentifier("sync-wallet")
                    }
                    Section("Receive") {
                        address("Ark address", model.arkAddress)
                        address("Onchain address", model.onchainAddress)
                        Button("New Ark address") { model.perform { model.arkAddress = try await model.wallet!.newAddress() } }
                    }
                    Section("Send") {
                        TextField("Ark address or Lightning invoice", text: $destination, axis: .vertical)
                            .textInputAutocapitalization(.never).autocorrectionDisabled().accessibilityIdentifier("destination")
                        TextField("Amount in sats", text: $amount).keyboardType(.numberPad).accessibilityIdentifier("send-amount")
                        Button("Send payment") { model.perform { try await model.send(destination: destination, amount: amount) } }.accessibilityIdentifier("send-payment")
                    }
                    Section("Lightning receive") {
                        Button("Create invoice for \(amount) sats") {
                            model.perform { model.invoice = try await model.wallet!.bolt11Invoice(amountSats: model.amount(amount), description: "Bark example", token: nil).invoice }
                        }
                        if !model.invoice.isEmpty { address("Invoice", model.invoice) }
                        Button("Claim received payments") { model.perform { _ = try await model.wallet!.tryClaimAllLightningReceives(wait: true) } }
                    }
                    Section("Activity") {
                        if model.activity.isEmpty { Text("No activity yet. Fund the onchain address, then board to Ark.").foregroundStyle(.secondary) }
                        ForEach(Array(model.activity.enumerated()), id: \.offset) { _, movement in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(movement.subsystemKind.capitalized).font(.headline)
                                Text("\(movement.status) · \(movement.id)").font(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                    Section("Board & maintain") {
                        TextField("Board amount in sats", text: $boardAmount).keyboardType(.numberPad)
                        Button("Board to Ark") { model.perform { _ = try await model.wallet!.boardAmount(amountSats: model.amount(boardAmount)); model.status = "Board submitted. Mine a block, then sync." } }.accessibilityIdentifier("board-wallet")
                        Button("Refresh VTXOs") { model.perform { let ids = try await model.wallet!.spendableVtxos().map(\.id); _ = try await model.wallet!.refreshVtxos(vtxoIds: ids) } }
                        Button("Offboard all to onchain address") { model.perform { _ = try await model.wallet!.offboardAll(bitcoinAddress: model.onchainAddress) } }
                        Button("Start unilateral exit", role: .destructive) { confirmExit = true }
                        Button("Progress exits") { model.perform { _ = try await model.wallet!.progressExits(feeRateSatPerVb: nil) } }
                        Button("Claim matured exits") {
                            model.perform {
                                let claim = try await model.wallet!.drainExits(vtxoIds: [], drainAll: true, address: model.onchainAddress, feeRateSatPerVb: nil)
                                let signed = try await model.wallet!.signExitClaimInputs(psbtBase64: claim.psbtBase64)
                                _ = try await model.wallet!.broadcastTx(txHex: extractTxFromPsbt(psbtBase64: signed))
                            }
                        }
                    }
                }
            }
            .disabled(model.busy)
            .navigationTitle("Bark Wallet")
            .tint(.green)
            .confirmationDialog("Exit every VTXO? This broadcasts onchain transactions and requires mining the exit delay.", isPresented: $confirmExit, titleVisibility: .visible) {
                Button("Start exit", role: .destructive) { model.perform { try await model.wallet!.startExitForEntireWallet() } }
            }
        }
    }

    private func address(_ title: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.subheadline).foregroundStyle(.secondary)
            Text(value).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
            ShareLink(item: value) { Label("Share", systemImage: "square.and.arrow.up") }
        }.padding(.vertical, 4)
    }
}
