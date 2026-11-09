// swift-tools-version:5.9
// The Swift host app. `make swift` builds the host library from
// `apps/host-app-native`, generates its UniFFI bindings into
// `Sources/HostAppNative` and `Sources/HostAppNativeFFI`, and runs the app.

import Foundation
import PackageDescription

// The host library, built by `cargo build -p host-app-native`.
let hostAppNativeLibrary = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent()
    .appendingPathComponent("../../target/debug/libhost_app_native.a")
    .standardized
    .path

let package = Package(
    name: "host-app-swift",
    platforms: [.macOS(.v13)],
    targets: [
        // The C side of the bindings: one header and module per crate the
        // library links, gathered in one module map.
        .systemLibrary(name: "HostAppNativeFFI", path: "Sources/HostAppNativeFFI"),
        // The Swift side of the bindings, every crate's file in one module.
        .target(
            name: "HostAppNative",
            dependencies: ["HostAppNativeFFI"],
            path: "Sources/HostAppNative"
        ),
        .executableTarget(
            name: "HostAppSwift",
            dependencies: ["HostAppNative"],
            path: "Sources/HostAppSwift",
            linkerSettings: [.unsafeFlags([hostAppNativeLibrary])]
        ),
    ]
)
