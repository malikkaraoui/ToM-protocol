// swift-tools-version: 5.9
// TomProtocolKit — SDK Apple du protocole ToM (chantier S2.2).
// Le binaire TomProtocolFFI.xcframework est copié dans Artifacts/ par
// scripts/sync-xcframework-to-package.sh (build local) ; les releases
// publiées remplaceront ce binaryTarget par une URL + checksum.
import PackageDescription

let package = Package(
    name: "TomProtocolKit",
    platforms: [
        .iOS(.v16),
        .tvOS(.v16),
        .macOS(.v13),
    ],
    products: [
        .library(name: "TomProtocolKit", targets: ["TomProtocolKit"])
    ],
    targets: [
        .binaryTarget(
            name: "TomProtocolFFI",
            path: "Artifacts/TomProtocolFFI.xcframework"
        ),
        .target(
            name: "TomProtocolKit",
            dependencies: ["TomProtocolFFI"],
            // Exigences de link de la lib statique Rust (libtom_protocol_ffi.a) :
            // portées par le package pour que les apps consommatrices n'aient
            // aucun OTHER_LDFLAGS à déclarer (S2.4).
            linkerSettings: [
                .linkedLibrary("z"),
                .linkedLibrary("resolv"),
                .linkedFramework("Network"),
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
            ]
        ),
    ]
)
