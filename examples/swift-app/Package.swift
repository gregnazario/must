// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "swift-app",
    targets: [
        .executableTarget(
            name: "swift-app",
            path: "Sources/swift-app"
        ),
        .testTarget(
            name: "swift-appTests",
            dependencies: ["swift-app"],
            path: "Tests/swift-appTests"
        ),
    ]
)
