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
