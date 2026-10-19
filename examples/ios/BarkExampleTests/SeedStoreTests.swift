import Security
import XCTest
@testable import BarkExample

final class SeedStoreTests: XCTestCase {
    private func remove(_ account: String) {
        SecItemDelete([kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "io.github.niteshbalusu11.bark.example",
            kSecAttrAccount as String: account] as CFDictionary)
    }

    @MainActor
    func testSamePhraseCanRetryWithoutReplacingSeed() throws {
        let account = "seed-test-" + UUID().uuidString
        defer { remove(account) }
        let seed = try SeedStore.loadOrCreate(account: account)
        XCTAssertTrue(try SeedStore.loadOrCreate(account: account, restore: seed) == seed)
        XCTAssertTrue(try SeedStore.loadOrCreate(account: account) == seed)
    }

    @MainActor
    func testDifferentPhraseCannotReplaceStoredSeed() throws {
        let account = "seed-test-" + UUID().uuidString
        let other = "seed-test-" + UUID().uuidString
        defer { remove(account); remove(other) }
        let seed = try SeedStore.loadOrCreate(account: account)
        let different = try SeedStore.loadOrCreate(account: other)
        XCTAssertThrowsError(try SeedStore.loadOrCreate(account: account, restore: different))
        XCTAssertTrue(try SeedStore.loadOrCreate(account: account) == seed)
    }
    @MainActor
    func testRestoresUseDistinctDirectoriesExcludedFromBackup() throws {
        let account = "directory-test-" + UUID().uuidString
        let first = try WalletModel.walletDirectory(account: account, restored: true)
        let second = try WalletModel.walletDirectory(account: account, restored: true)
        defer {
            try? FileManager.default.removeItem(at: first)
            try? FileManager.default.removeItem(at: second)
        }
        XCTAssertNotEqual(first, second)
        XCTAssertTrue(try first.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup == true)
        XCTAssertTrue(try second.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup == true)
    }

}
