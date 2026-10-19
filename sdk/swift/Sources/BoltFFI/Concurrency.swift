// Rust owns synchronized Send + Sync objects; BoltFFI atomically retains handles
// across async calls. NotificationHolder still allows only one active consumer.
extension Wallet: @unchecked Sendable {}
extension OnchainWallet: @unchecked Sendable {}
extension NotificationHolder: @unchecked Sendable {}
