// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Effector",
    platforms: [.macOS("26.0")],
    targets: [
        .executableTarget(
            name: "Effector",
            path: "Sources/Effector",
            swiftSettings: [.swiftLanguageMode(.v5)]
        )
    ]
)
