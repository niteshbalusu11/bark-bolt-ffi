# Tasks 07–10: native examples and executable acceptance

Paths: examples/ios (SwiftUI, XcodeGen project, XCTest UI target), examples/android
(Compose, ViewModel, instrumentation target), shared test consumers in tests/
and scripts for building, devices and deterministic RegTest transactions.

The user has authorized implementation without another design approval round.
Use native system navigation and controls: wallet overview, receive/send,
Lightning, activity and advanced maintenance/exit actions. The first view shows
actual balances and connection state; addresses use selectable monospaced text.
Show a clearly visible RegTest label. Loading disables conflicting actions;
errors preserve inputs and offer retry. Empty activity explains funding/boarding.
Use semantic colors/dynamic text, system icons, accessible action identifiers.
Phone-only initial app targets avoid pretending to have tablet layouts.

Store a newly generated or restored mnemonic in Keychain (iOS) or AES-GCM with an
Android Keystore key and private app preferences. Database stays in private
application storage. Never log/export the seed in diagnostics. Restore is an
explicit onboarding operation and never replaces an existing wallet silently.
Settings hold local RegTest endpoints, not production keys; cleartext loopback
network access is debug-only. SDK handles remain owned by model; subscriptions
cancel before shutdown and explicit native handles close in lifecycle cleanup.

SDK APIs provide all wallet behavior. A debug test helper may call Bitcoin RPC
for funding/mining, but it is not part of SDK and never simulates SDK results.
Both app test suites exercise create/open, actual board and transfer, balances,
restart, errors and cancellation through the same model used by the UI. Longer
Lightning, offboard/refresh and unilateral exit/restore scenarios also run as
native SDK integration tests, recording real identifiers/state assertions.

Device setup: install API35 arm64 emulator locally, use API35 x86_64 on Actions;
open iOS/Android via T3 device tools before driving with their exact agent-device
launcher. iOS localhost reaches host; Android uses adb reverse for both ports.
Use bounded readiness/confirmation waits and isolated app test directories.
Screenshots and sanitized results go under ignored artifacts/; keep a short
committed validation record with real dates/commands/IDs.

Simulator Keychain requires an ad-hoc signed app with application-identifier and
keychain-access-groups entitlements; an unsigned simulator build fails -34018.
Use local ad-hoc signing (no Apple developer credentials) for simulator tests.

Independent native finish review: retain visible labels for prefilled iOS Ark
server/Bitcoin RPC/send amount/board amount fields. Add platform-native pinned
operation feedback outside the scrollable task content on both platforms, so
busy/error/result state remains visible from Send/maintenance. A details action
must expose the full error without truncation; announce state changes using
native accessibility mechanisms. Preserve the existing task layout and actual
SDK calls. Make generic successful actions report an updated status when they
did not supply a specific message. Replace empty-history copy with factual
“No saved activity yet” because restoring funds need not restore history.
Build once, run native UI checks, then recapture required phone views for the
reviewer's verdict. Do not change the Rust interface or generated bindings.
