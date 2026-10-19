# Tasks 11–12: hosted builds and full RegTest integration

Paths: .github/workflows/, scripts/build-sdk.sh, scripts/regtest.py,
scripts/app-regtest.py and build/test documentation.

Push/PR jobs run Rust formatting, Clippy, tests and audit on Ubuntu; Linux builds
Android/JVM artifacts and strict generation/parity, compiles release AAR/debug APK,
and runs emulator instrumentation plus real Docker RegTest app smoke tests.
macOS builds all Apple slices, compiles/runs Swift contracts, builds iOS app and
runs XCTest UI tests. Upload APK/AAR/JAR/XCFramework, reports and logs from the same
commit, pin toolchains/dependencies and run with read-only repository permission.

Full cross-platform integration uses a manually dispatched workflow on a macOS
runner labelled bark-bolt-regtest, with Xcode, Docker, Android emulator/SDK and
JDK17. This is explicit because hosted macOS runners cannot be assumed to run
Linux service containers. Validate that job using an ephemeral runner on this
machine; registration is scoped to the new private repo and executes only this
repo's trusted workflow. It exits after one job and is not installed as a service.
Default PR builds do not depend on the ephemeral runner remaining online.
Document the prerequisites for a permanent integration runner separately.

All commands have bounded timeouts; retain diagnostics on failure. Never claim a
hosted build from a local result. Trigger actual Actions runs after pushing the
implementation and fix failed jobs. Verify API parity after strict generation.
