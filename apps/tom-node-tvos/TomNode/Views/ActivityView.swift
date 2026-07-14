import SwiftUI
import TomProtocolKit

struct ActivityView: View {
    @EnvironmentObject var nodeService: TomNodeService
    @State private var selectedCategory: ActivityCategory = .all

    var body: some View {
        #if os(tvOS)
        tvOSLayout
        #else
        iosLayout
        #endif
    }

    // MARK: - Category enum and filtering

    enum ActivityCategory: String, CaseIterable {
        case all = "Tous"
        case peers = "Pairs"
        case roles = "Rôles"
        case security = "Sécurité"
        case other = "Autres"

        var color: Color {
            switch self {
            case .all: return .blue
            case .peers: return .blue
            case .roles: return .purple
            case .security: return .red
            case .other: return .gray
            }
        }
    }

    private func categoryFor(_ entry: ActivityEntry) -> ActivityCategory {
        let type = entry.type

        // Pairs
        if type.contains("Peer") || type.contains("GossipNeighbor") {
            return .peers
        }

        // Rôles
        if type.contains("Role") || type.contains("LocalRole") {
            return .roles
        }

        // Backup (orange) → pour l'instant mappé à "Autres"
        if type.contains("Backup") {
            return .other
        }

        // Sécurité/Antispam
        if type.contains("Throttled") || type.contains("Rejected") || type.contains("GroupSecurity") {
            return .security
        }

        // Chemins (vert) → pour l'instant mappé à "Autres"
        if type.contains("Path") {
            return .other
        }

        // Groupes (gris)
        if type.contains("Group") {
            return .other
        }

        return .other
    }

    private var filteredEntries: [ActivityEntry] {
        let all = nodeService.activityLog
        if selectedCategory == .all {
            return all
        }
        return all.filter { categoryFor($0) == selectedCategory }
    }

    // MARK: - iOS/iPadOS layout

    private var iosLayout: some View {
        VStack(spacing: 0) {
            // Header with title and category filter
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text("Historique")
                        .font(.title2)
                        .fontWeight(.bold)
                    Spacer()
                    Text("\(filteredEntries.count)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .padding(.horizontal, 8).padding(.vertical, 4)
                        .background(Color.secondary.opacity(0.2))
                        .cornerRadius(6)
                }

                // Segmented control for category filter
                Picker("Catégorie", selection: $selectedCategory) {
                    ForEach(ActivityCategory.allCases, id: \.self) { category in
                        Text(category.rawValue).tag(category)
                    }
                }
                .pickerStyle(.segmented)
                .font(.caption)
            }
            .padding(16)
            .background(Color.secondary.opacity(0.08))

            // Activity list
            if filteredEntries.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "bell.slash")
                        .font(.system(size: 40))
                        .foregroundColor(.secondary)
                    Text("Aucune activité")
                        .font(.body)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.secondary.opacity(0.1))
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 8) {
                        ForEach(filteredEntries.reversed()) { entry in
                            activityRowContent(entry)
                        }
                    }
                    .padding(16)
                }
            }
        }
    }

    // MARK: - tvOS layout

    private var tvOSLayout: some View {
        VStack(alignment: .leading, spacing: 20) {
            // Header
            HStack(alignment: .top, spacing: 16) {
                VStack(alignment: .leading, spacing: 12) {
                    Text("Historique des événements")
                        .font(.headline)
                    Text("\(filteredEntries.count) événement\(filteredEntries.count == 1 ? "" : "s")")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Spacer()
            }

            // Category filter as buttons (tvOS friendly)
            VStack(alignment: .leading, spacing: 12) {
                Text("Filtrer par")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                    .kerning(1)

                VStack(spacing: 8) {
                    ForEach(ActivityCategory.allCases, id: \.self) { category in
                        Button(action: {
                            selectedCategory = category
                        }) {
                            HStack {
                                Text(category.rawValue)
                                    .font(.body)
                                Spacer()
                                if selectedCategory == category {
                                    Image(systemName: "checkmark")
                                        .foregroundColor(category.color)
                                }
                            }
                            .padding(.horizontal, 12)
                            .padding(.vertical, 8)
                            .background(selectedCategory == category
                                         ? category.color.opacity(0.2)
                                         : Color.secondary.opacity(0.1))
                            .cornerRadius(8)
                            .foregroundColor(selectedCategory == category
                                             ? category.color
                                             : .white)
                        }
                    }
                }
            }
            .padding(14)
            .background(Color.secondary.opacity(0.06))
            .cornerRadius(12)

            // Activity list (scrollable, cappé visuellement)
            if filteredEntries.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "bell.slash")
                        .font(.system(size: 40))
                        .foregroundColor(.secondary)
                    Text("Aucun événement")
                        .font(.body)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 8) {
                        ForEach(filteredEntries.reversed()) { entry in
                            tvActivityRow(entry)
                        }
                    }
                }
                .frame(maxHeight: .infinity)
            }

            Spacer()
        }
        .padding(40)
    }

    // MARK: - Row components

    private func activityRowContent(_ entry: ActivityEntry) -> some View {
        HStack(alignment: .top, spacing: 12) {
            // Color badge for category
            Circle()
                .fill(colorForCategory(categoryFor(entry)))
                .frame(width: 10, height: 10)
                .padding(.top, 4)

            VStack(alignment: .leading, spacing: 4) {
                Text(entry.displayText)
                    .font(.subheadline)
                    .lineLimit(2)
                Text(entry.relativeTime)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            Spacer()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(colorForCategory(categoryFor(entry)).opacity(0.08))
        .cornerRadius(8)
    }

    private func tvActivityRow(_ entry: ActivityEntry) -> some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 4) {
                Text(entry.displayText)
                    .font(.body)
                    .lineLimit(1)
                Text(entry.relativeTime)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            Spacer()
            Circle()
                .fill(colorForCategory(categoryFor(entry)))
                .frame(width: 10, height: 10)
                .padding(.top, 4)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(colorForCategory(categoryFor(entry)).opacity(0.08))
        .cornerRadius(8)
    }

    private func colorForCategory(_ category: ActivityCategory) -> Color {
        switch category {
        case .all: return .blue
        case .peers: return .blue
        case .roles: return .purple
        case .security: return .red
        case .other: return .gray
        }
    }
}

#Preview {
    ActivityView()
        .environmentObject(TomNodeService.shared)
}
