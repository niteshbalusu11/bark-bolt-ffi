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
            url: "https://github.com/smolcars/bark-bolt-ffi/releases/download/v0.1.0/BarkBoltFFI.xcframework.zip", checksum: "14afb3037fe89e7fd9068563f0135fb0fcc5a5f3362343312ef205aad9d1dc67"
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
