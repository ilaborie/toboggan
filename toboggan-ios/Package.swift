// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "TobogganCore",
    platforms: [.iOS(.v16)],
    products: [
        .library(name: "TobogganCore", targets: ["TobogganCore"])
    ],
    targets: [
        .binaryTarget(
            name: "TobogganCore",
            path: "./target/TobogganCore.xcframework"
        )
    ]
)