# Security reporting

This repository maintains the BoltFFI bindings and native examples. Report
binding, packaging or example-app vulnerabilities privately to the repository
owner through GitHub. Include the commit, platform and a minimal reproduction;
exclude recovery phrases, wallet databases and credentials.

For vulnerabilities in the underlying Bark wallet/protocol, follow
[upstream's security policy](https://gitlab.com/ark-bitcoin/bark/-/blob/master/SECURITY.md).
The upstream team's reporting channel applies to upstream core code; it does
not imply that they maintain this fork.

CI runs cargo-audit against Cargo.lock. Advisory exceptions require a documented
reason in .cargo/audit.toml; the current ignore list is empty.
