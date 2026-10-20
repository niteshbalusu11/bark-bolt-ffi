// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "BarkBoltFFI",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
    ],
    products: [
        .library(
            name: "BarkBoltFFI",
            targets: ["BarkBoltFFI"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "BarkBoltFFIFFIBinary",
            url: "https://github.com/smolcars/bark-bolt-ffi/releases/download/v0.3.1/BarkBoltFFI.xcframework.zip", checksum: "06e837aea74517e58e21c1a8eb0080e93651dfd282dedafc9aebc382b737477b"
        ),
        .target(
            name: "BarkBoltFFIFFI",
            dependencies: ["BarkBoltFFIFFIBinary"],
            path: "sdk/swift/FFI",
            publicHeadersPath: "include"
        ),
        .target(
            name: "BarkBoltFFI",
            dependencies: ["BarkBoltFFIFFI"],
            path: "sdk/swift/Sources"
        ),
    ]
)
