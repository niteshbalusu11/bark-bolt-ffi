// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "BarkBoltffiConsumer",
    platforms: [.macOS(.v13)],
    dependencies: [.package(path: "../../dist/apple")],
    targets: [
        .executableTarget(
            name: "BarkBoltffiConsumer",
            dependencies: [.product(name: "BarkBoltffiProof", package: "apple")]
        )
    ]
)
