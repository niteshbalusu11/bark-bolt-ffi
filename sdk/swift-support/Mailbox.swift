// Handwritten overload; generated bindings stay untouched.
extension Wallet {
    /// Absolute Unix expiry in seconds, matching Nitro v0.0.144.
    public func mailboxAuthorization(authorizationExpiry: Int64) throws -> MailboxAuthorization {
        try mailboxAuthorizationAt(authorizationExpiry: authorizationExpiry)
    }
}
