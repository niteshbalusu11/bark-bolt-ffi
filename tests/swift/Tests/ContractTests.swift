import Foundation
import XCTest
import BarkBoltFFI

final class ContractTests: XCTestCase {
    func testLoggerRetainsThenReleasesForeignCallback() throws {
        weak var released: Logger?
        do {
            let logger = Logger()
            released = logger
            try setLogger(logger: logger, maxLevel: .info)
        }
        XCTAssertNotNil(released)
        clearLogger()
        XCTAssertNil(released)
    }

    func testMnemonicAndErrors() throws {
        XCTAssertTrue(try validateMnemonic(mnemonic: generateMnemonic()))
        XCTAssertFalse(try validateMnemonic(mnemonic: "invalid"))
        XCTAssertEqual(defaultVtxoKeyGapLimit(), 250)
        XCTAssertEqual(maxVtxoKeyGapLimit(), 100000)
    }

    func testCallbackFullRangeAndRelease() async throws {
        weak var released: Callbacks?
        do {
            let callback = Callbacks()
            released = callback
            let wallet = try OnchainWallet(custom: callback)
            let balance = try await wallet.balance()
            XCTAssertEqual(balance.totalSats, UInt64.max)
            do {
                try await wallet.evictTx(txid: String(repeating: "11", count: 32))
                XCTFail("Expected callback error")
            } catch let error as BarkError { XCTAssertTrue(error.message.contains("callback failure 🦀\0")) }
            callback.enableUnexpectedError()
            do {
                try await wallet.evictTx(txid: String(repeating: "11", count: 32))
                XCTFail("Expected unexpected callback error conversion")
            } catch let error as BarkError { XCTAssertTrue(error.message.contains("unexpected callback failure")) }
            let again = try await wallet.balance()
            XCTAssertEqual(again.totalSats, UInt64.max)
        }
        XCTAssertNil(released)
    }
}

private final class Callbacks: CustomOnchainWalletCallbacks {
    private let lock = NSLock()
    private var unexpected = false
    func enableUnexpectedError() { lock.lock(); unexpected = true; lock.unlock() }
    func getBalance() throws -> UInt64 { UInt64.max }
    func prepareTx(destinations: [Destination], feeRateSatPerVb: UInt64) throws -> String { throw BarkError(message: "unexpected prepare") }
    func prepareDrainTx(address: String, feeRateSatPerVb: UInt64) throws -> String { throw BarkError(message: "unexpected drain") }
    func signPsbt(psbtBase64: String) throws -> String { throw BarkError(message: "unexpected sign") }
    func finishPsbt(psbtBase64: String) throws -> String { throw BarkError(message: "unexpected finish") }
    func isMine(scriptPubkeyHex: String) throws -> Bool { false }
    func registerTx(txHex: String) throws {}
    func evictTx(txid: String) throws {
        lock.lock(); let enabled = unexpected; lock.unlock()
        if enabled { throw NSError(domain: "BarkContract", code: 1, userInfo: [NSLocalizedDescriptionKey: "unexpected callback failure"]) }
        throw BarkError(message: "callback failure 🦀\0")
    }
    func makeSignedP2aCpfp(params: CpfpParams) throws -> String { throw BarkError(message: "unexpected cpfp") }
    func storeSignedP2aCpfp(txHex: String) throws {}
    func sync() throws {}
}

private final class Logger: BarkLogger {
    func log(level: LogLevel, target: String, message: String) {}
}

private var fixtures: URL {
    if let configured = ProcessInfo.processInfo.environment["BARK_CONTRACT_FIXTURES"] { return URL(fileURLWithPath: configured) }
    return URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("fixtures")
}
private struct NitroFixture: Decodable {
    struct Vector: Decodable {
        let index: UInt32
        let offlinePublicKey, offlineSignature, walletPublicKey, walletSignature: String
    }
    let mnemonic, message: String
    let vectors: [Vector]
    static func load() throws -> Self { try JSONDecoder().decode(Self.self, from: Data(contentsOf: fixtures.appendingPathComponent("nitro-v0.0.144-crypto.json"))) }
}

extension ContractTests {
    func testNitroOfflineGoldenVectors() throws {
        let fixture = try NitroFixture.load()
        for network: Network in [.bitcoin, .testnet, .signet, .regtest] {
            for vector in fixture.vectors {
                let key = try derivePublicKeyFromMnemonic(mnemonic: fixture.mnemonic, network: network, index: vector.index)
                let signature = try signMessageWithMnemonic(message: fixture.message, mnemonic: fixture.mnemonic, network: network, index: vector.index)
                XCTAssertEqual(key, vector.offlinePublicKey)
                XCTAssertEqual(signature, vector.offlineSignature)
                XCTAssertNotEqual(key, vector.walletPublicKey)
                XCTAssertTrue(try verifyMessage(message: fixture.message, signatureDerHex: signature, publicKeyHex: key))
                XCTAssertFalse(try verifyMessage(message: fixture.message + "!", signatureDerHex: signature, publicKeyHex: key))
            }
        }
        XCTAssertThrowsError(try derivePublicKeyFromMnemonic(mnemonic: "invalid", network: .regtest, index: 0))
        XCTAssertThrowsError(try verifyMessage(message: "test", signatureDerHex: "00", publicKeyHex: "00"))
    }

    func testSchema42SnapshotWithoutWallet() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let path = directory.appendingPathComponent("db.sqlite")
        try FileManager.default.copyItem(at: fixtures.appendingPathComponent("noah-v2-schema42.sqlite"), to: path)
        let before = try Data(contentsOf: path)
        let info = try await validateWalletSnapshot(path: path.path, expected: nil)
        XCTAssertEqual(info.schemaVersion, 42)
        XCTAssertEqual(info.network, "regtest")
        XCTAssertEqual(info.sizeBytes, UInt64(before.count))
        XCTAssertEqual(info.mailboxPubkey, "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5")
        XCTAssertEqual(try Data(contentsOf: path), before)
        do {
            _ = try await validateWalletSnapshot(path: path.path, expected: WalletSnapshotExpectation(network: "bitcoin"))
            XCTFail("Expected identity mismatch")
        } catch is BarkError {}
    }

    func testWalletNitroVectorsAndSnapshotObserver() async throws {
        guard ProcessInfo.processInfo.environment["BARK_REGTEST"] == "1" else { throw XCTSkip("requires live RegTest") }
        let fixture = try NitroFixture.load()
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let env = ProcessInfo.processInfo.environment
        let config = Config(serverAddress: "http://127.0.0.1:\(env["ARK_PORT"] ?? "3635")", bitcoindAddress: "http://127.0.0.1:\(env["BITCOIN_PORT"] ?? "19443")", bitcoindUser: "second", bitcoindPass: "ark")
        let wallet = try await Wallet.open(network: .regtest, mnemonicOrSeed: fixture.mnemonic, config: config, args: WalletOpenArgs(runDaemon: false, datadir: directory.path, createWithoutServer: true, skipRecovery: true))
        do { _ = try await wallet.peekPublicKey(index: 0); XCTFail("Expected unstored key error") } catch is BarkError {}
        do { _ = try await wallet.signMessage(message: fixture.message, index: 0); XCTFail("Expected unstored key error") } catch is BarkError {}
        let holder = try wallet.stateChanges()
        defer { holder.stop() }
        XCTAssertTrue(holder.isActive())
        let initial = try await holder.nextChange()
        XCTAssertEqual(initial?.reason, "initial")
        let aborted = Task { try await holder.nextChange() }
        try await Task.sleep(nanoseconds: 50_000_000)
        aborted.cancel()
        _ = try? await aborted.value
        let cancelled = Task { try await holder.nextChange() }
        try await Task.sleep(nanoseconds: 50_000_000)
        do { _ = try await holder.nextChange(); XCTFail("Expected concurrent wait error") } catch is BarkError {}
        holder.cancelNextChangeWait()
        let cancelledValue = try await cancelled.value
        XCTAssertNil(cancelledValue)
        for index in UInt32(0)...7 {
            let stored = try await wallet.deriveStoreNextKeypair()
            XCTAssertEqual(stored.index, index)
            if let vector = fixture.vectors.first(where: { $0.index == index }) {
                XCTAssertEqual(stored.publicKey, vector.walletPublicKey)
                let key = try await wallet.peekPublicKey(index: index)
                XCTAssertEqual(key, stored.publicKey)
                let signature = try await wallet.signMessage(message: fixture.message, index: index)
                XCTAssertEqual(signature, vector.walletSignature)
                XCTAssertTrue(try verifyMessage(message: fixture.message, signatureDerHex: signature, publicKeyHex: key))
                XCTAssertFalse(try verifyMessage(message: fixture.message + "!", signatureDerHex: signature, publicKeyHex: key))
            }
        }
        let changed = try await holder.nextChange()
        XCTAssertEqual(changed?.reason, "databaseChanged")
        let snapshot = try await wallet.createSnapshot(destinationPath: directory.appendingPathComponent("snapshot.sqlite").path)
        let validated = try await validateWalletSnapshot(path: snapshot.path, expected: nil)
        XCTAssertEqual(snapshot.sha256, validated.sha256)
        do { _ = try await wallet.createSnapshot(destinationPath: snapshot.path); XCTFail("Expected no overwrite") } catch is BarkError {}
        do { _ = try await validateWalletSnapshot(path: directory.appendingPathComponent("db.sqlite").path, expected: nil); XCTFail("Expected live database refusal") } catch is BarkError {}
        let stopped = Task { try await holder.nextChange() }
        try await Task.sleep(nanoseconds: 50_000_000)
        holder.stop()
        let stoppedValue = try await stopped.value
        XCTAssertNil(stoppedValue)
        XCTAssertFalse(holder.isActive())
    }
}
