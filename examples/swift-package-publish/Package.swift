// swift-tools-version:5.9

import PackageDescription

let package = Package(
    name: "MySwiftPackage",
    platforms: [.macOS(.v13), .iOS(.v16)],
    products: [
        .library(name: "MySwiftPackage", targets: ["MySwiftPackage"]),
    ],
    targets: [
        .target(name: "MySwiftPackage"),
        .testTarget(name: "MySwiftPackageTests", dependencies: ["MySwiftPackage"]),
    ]
)
