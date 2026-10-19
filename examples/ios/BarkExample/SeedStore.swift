import Foundation
import Security
import BarkBoltFFI

enum SeedStore {
    static func loadOrCreate(account: String, restore: String = "") throws -> String {
        let phrase = restore.trimmingCharacters(in: .whitespacesAndNewlines)
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "io.github.niteshbalusu11.bark.example",
            kSecAttrAccount as String: account]
        var item: CFTypeRef?
        var read = query
        read[kSecReturnData as String] = true
        let status = SecItemCopyMatching(read as CFDictionary, &item)
        if status == errSecSuccess, let data = item as? Data, let seed = String(data: data, encoding: .utf8) {
            guard phrase.isEmpty || phrase == seed else { throw AppError.message("A wallet already exists. Open it to preserve its funds.") }
            return seed
        }
        guard status == errSecItemNotFound else { throw AppError.message("Keychain could not read the wallet (\(status)).") }
        let seed = phrase.isEmpty ? try generateMnemonic() : phrase
        guard try validateMnemonic(mnemonic: seed) else { throw AppError.message("Enter a valid recovery phrase.") }
        var insert = query
        insert[kSecValueData as String] = Data(seed.utf8)
        insert[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let saved = SecItemAdd(insert as CFDictionary, nil)
        guard saved == errSecSuccess else { throw AppError.message("Keychain could not save the wallet (\(saved)).") }
        return seed
    }
}

enum AppError: LocalizedError {
    case message(String)
    var errorDescription: String? { if case let .message(message) = self { return message }; return nil }
}
