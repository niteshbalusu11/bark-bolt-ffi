#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

if [[ "$(uname -s)/$(uname -m)" != "Darwin/arm64" ]]; then
    echo "This proof selects Apple silicon slices; run it on an Apple silicon Mac." >&2
    exit 1
fi

BOLTFFI="${BOLTFFI:-boltffi}"
KOTLINC="${KOTLINC:-kotlinc}"
if [[ "$("$BOLTFFI" --version)" != "boltffi 0.31.1" ]]; then
    echo "Use the BoltFFI 0.31.1 CLI to match Cargo.lock." >&2
    exit 1
fi
"$KOTLINC" -version
export JAVA_HOME="${JAVA_HOME:-$(/usr/libexec/java_home -v 17)}"
export RUSTUP_TOOLCHAIN=1.90.0

coroutines_jar="target/tools/kotlinx-coroutines-core-jvm-1.8.1.jar"
mkdir -p target/tools
if [[ ! -f "$coroutines_jar" ]]; then
    curl --fail --location --silent --show-error \
        https://repo.maven.apache.org/maven2/org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm/1.8.1/kotlinx-coroutines-core-jvm-1.8.1.jar \
        --output "$coroutines_jar"
fi
echo "f3d4f5de1c391bbcc20f3b3435ccbac013521e76b6902d7d59635ec15c1f797e  $coroutines_jar" | shasum -a 256 --check

cargo fmt --check
cargo build --locked
"$BOLTFFI" pack apple --deny-skipped --layout ffi-only
swift run --package-path consumers/swift
# Swift 6.4's default build engine rejects iOS 14; the native engine compiles it.
swift build --build-system native --package-path dist/apple --scratch-path target/swift-ios-simulator \
    --triple arm64-apple-ios14.0-simulator \
    --sdk "$(xcrun --sdk iphonesimulator --show-sdk-path)"

"$BOLTFFI" pack android --deny-skipped
"$KOTLINC" dist/kotlin/tech/second/bark/proof/BarkBoltffiProof.kt \
    consumers/kotlin/Consumer.kt \
    -classpath "$coroutines_jar" -jvm-target 17 -include-runtime \
    -d target/kotlin-consumer.jar
java -cp "target/kotlin-consumer.jar:$coroutines_jar:dist/android/desktopJniLibs" ConsumerKt
