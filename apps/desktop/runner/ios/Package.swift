// swift-tools-version:5.3

import PackageDescription

let package = Package(
  name: "tauri-plugin-runner",
  platforms: [.iOS(.v13)],
  products: [
    .library(name: "tauri-plugin-runner", type: .static, targets: ["tauri-plugin-runner"])
  ],
  dependencies: [.package(name: "Tauri", path: "../.tauri/tauri-api")],
  targets: [
    .target(name: "tauri-plugin-runner", dependencies: [.byName(name: "Tauri")], path: "Sources")
  ]
)
