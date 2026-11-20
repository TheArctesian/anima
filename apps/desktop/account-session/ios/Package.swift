// swift-tools-version:5.3
// The iOS side of the account session plugin (mobile plan M3-5).

import PackageDescription

let package = Package(
  name: "tauri-plugin-account-session",
  platforms: [
    .iOS(.v13)
  ],
  products: [
    .library(
      name: "tauri-plugin-account-session",
      type: .static,
      targets: ["tauri-plugin-account-session"])
  ],
  dependencies: [
    .package(name: "Tauri", path: "../.tauri/tauri-api")
  ],
  targets: [
    .target(
      name: "tauri-plugin-account-session",
      dependencies: [
        .byName(name: "Tauri")
      ],
      path: "Sources")
  ]
)
