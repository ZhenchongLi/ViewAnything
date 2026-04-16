// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "AnythingView",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "AnythingView",
            path: "Sources/AnythingView",
            exclude: ["Info.plist"],
            resources: [.process("Resources")]
        ),
    ]
)
