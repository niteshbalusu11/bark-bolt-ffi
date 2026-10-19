# Working on bark-bolt-ffi

The product is a Rust Bark implementation exposed through BoltFFI to Swift and
Kotlin, with native iOS and Android example apps and GitHub Actions CI. Breaking
SDK API changes are allowed. Targets are iOS 16+, macOS 13+, Android API 24+ and
JVM 17. Preserve upstream wallet logic, persistence compatibility and transaction
validation unless a separately documented change is needed.

Use docs/production-migration-todo.md as the execution checklist. Before coding
each task, write or refine its engineering plan: affected paths, interface and
ownership decisions, invariants, dependencies, and validation. Then implement it,
run the checks, and record actual outcomes and remaining limits. Planning does
not require waiting for user approval between routine tasks. Never mark a task
complete from inspection or a mock when acceptance requires a build, running app,
live RegTest transaction, or successful CI run.

The original bark-ffi and Nitro Ark checkouts are references. Work in this new
repository, preserve the upstream license/attribution, and adapt Nitro Ark's
RegTest setup into an isolated environment. Never reset its existing wallet or
database volumes. Apps must use newly built SDK artifacts from this repository.
