# Prepared-send SDK release, v0.4.0

After PR #3's native source CI passes, merge without changing the tested Rust or
build configuration. Reuse that run's authenticated Apple/Android SDK artifacts,
verify their published Actions digests and exact native-source equivalence, then
stage Package.swift and sdk/swift from the real five-slice Apple build. Keep the
CI-built three-ABI AAR and Linux JVM JAR/POMs. Run the newly compiled Swift
prepared-send contracts locally against real RegTest before publication; hosted
Android runs both ordinary and packaged Kotlin contracts with RegTest enabled.
Tag an immutable v0.4.0 metadata commit, publish checksums/build provenance, verify
fresh downloads and a clean public Swift consumer, then notify Noah with the
exact API/ABI and process-local reservation limits. Record actual outcomes before
checking off the remaining items. Darwin JVM binaries remain source-buildable.
