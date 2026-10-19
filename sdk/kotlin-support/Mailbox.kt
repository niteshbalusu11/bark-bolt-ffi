package io.github.niteshbalusu11.bark

/** Absolute Unix expiry in seconds, matching Nitro v0.0.144. */
fun Wallet.mailboxAuthorization(authorizationExpiry: Long): MailboxAuthorization =
    mailboxAuthorizationAt(authorizationExpiry)
