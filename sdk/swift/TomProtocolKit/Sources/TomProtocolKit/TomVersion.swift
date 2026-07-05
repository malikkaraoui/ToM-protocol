/// Single source of truth for the ToM app version across iOS, tvOS and macOS.
///
/// Shown in each app's Settings so every device can confirm — at a glance —
/// that it runs the SAME build. Bump `build` on every GitHub push (and `semantic`
/// on feature changes). All three Apple apps import this from TomProtocolKit, so
/// there is exactly one place to update.
public enum TomVersion {
    /// Semantic (marketing) version.
    public static let semantic = "0.6.0"

    /// Build number — incremented on every push to GitHub.
    public static let build = 15

    /// Human-readable version shown in Settings, e.g. "0.6.0 (build 1)".
    public static var display: String { "\(semantic) (build \(build))" }
}
