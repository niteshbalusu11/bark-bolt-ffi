// swift-tools-version:5.9
import PackageDescription
let package = Package(name: "BarkContractTests", platforms: [.macOS(.v13)],
    dependencies: [.package(path: "../../dist/apple")],
    targets: [.testTarget(name: "BarkContractTests", dependencies: [.product(name: "BarkBoltFFI", package: "apple")], path: "Tests")])
