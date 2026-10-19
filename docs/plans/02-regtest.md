# Task 02 engineering plan

Initial release0.7.1 implementation plan. The validated current-master revision
and dated images are specified by [task14](14-master-and-cleanup.md).

Use a separate Compose project `bark-bolt-ffi` with its own named volumes and
loopback host ports (Bitcoin RPC 19443 and Ark gRPC 3635). Start Bitcoin 31,
Postgres 16, CLN with the hold plugin, LND, and the official Second Ark server
and CLI tagged 0.7.1. Resolve image digests after pulling and use those digests.
The Nitro nightly images identify themselves as 0.6.2-dev, so they are not the
initial compatibility baseline.

Adapt the upstream 0.7.1 server default configuration with container endpoints,
CLN certificates and a short RegTest exit delay. Keep original Nitro volumes
untouched. Bootstrap Postgres/server state and fund the server with mature coins;
set up a funded CLN/LND channel. Use bounded health/readiness loops and idempotent
wallet creation instead of fixed sleeps or deleting wallet directories.

Paths: `regtest/compose.yaml`, server configuration and `scripts/ark-dev.sh`.
Task16 restores the original Justfile/Bash entry points in place of the initial
Python stack manager; current setup is `just setup-everything`.
Validation: healthy services, exact versions/digests, reference CLI handshake,
funding/boarding, a real payment, channel liquidity and repeatable setup.

Observed prerequisite: the exact 0.7.1 server rejects Bitcoin Core 30 with
`Old bitcoind version detected. Please upgrade to v31 or later`. Use Bitcoin 31
rather than bypassing that check. The CLN image also requires a separate `cln`
Postgres database, now included in the fresh-volume initialization SQL.
