import SwiftUI

struct GroupsView: View {
    @EnvironmentObject var nodeService: TomNodeService
    @State private var showCreateSheet = false
    @State private var newGroupName = ""
    @State private var newGroupMembers = ""

    // Derive groups from messages (group_id present)
    private var groups: [GroupId: [TomMessage]] {
        Dictionary(grouping: nodeService.messages.filter { $0.groupId != nil }) { $0.groupId! }
    }

    var body: some View {
        NavigationStack {
            Group {
                if groups.isEmpty {
                    VStack(spacing: 16) {
                        Image(systemName: "person.3")
                            .font(.system(size: 48))
                            .foregroundColor(.secondary)
                        Text("No groups yet")
                            .font(.title3)
                            .foregroundColor(.secondary)
                        if nodeService.state == .running {
                            Text("Create a group or wait for an invitation")
                                .font(.callout)
                                .foregroundColor(.secondary)
                        }
                    }
                } else {
                    List {
                        ForEach(Array(groups.keys.sorted()), id: \.self) { groupId in
                            NavigationLink {
                                GroupDetailView(
                                    groupId: groupId,
                                    messages: groups[groupId] ?? []
                                )
                            } label: {
                                GroupRow(groupId: groupId, messageCount: groups[groupId]?.count ?? 0)
                            }
                        }
                    }
                }
            }
            .navigationTitle("Groups")
            .toolbar {
                if nodeService.state == .running {
                    Button(action: { showCreateSheet = true }) {
                        Image(systemName: "plus")
                    }
                }
            }
            .sheet(isPresented: $showCreateSheet) {
                CreateGroupSheet(
                    groupName: $newGroupName,
                    members: $newGroupMembers,
                    onCreate: {
                        let memberList = newGroupMembers
                            .split(separator: ",")
                            .map { $0.trimmingCharacters(in: .whitespaces) }
                        nodeService.createGroup(name: newGroupName, members: memberList)
                        newGroupName = ""
                        newGroupMembers = ""
                        showCreateSheet = false
                    }
                )
            }
        }
    }
}

struct GroupRow: View {
    let groupId: GroupId
    let messageCount: Int

    var body: some View {
        HStack {
            Image(systemName: "person.3.fill")
                .foregroundColor(.accentColor)
            VStack(alignment: .leading) {
                Text(String(groupId.prefix(12)) + "...")
                    .font(.system(.body, design: .monospaced))
                Text("\(messageCount) messages")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
}

struct GroupDetailView: View {
    @EnvironmentObject var nodeService: TomNodeService
    let groupId: GroupId
    let messages: [TomMessage]
    @State private var messageText = ""

    var body: some View {
        VStack {
            List(messages) { msg in
                MessageRow(message: msg)
            }

            HStack {
                TextField("Message...", text: $messageText)
                Button("Send") {
                    nodeService.sendGroupMessage(groupId: groupId, text: messageText)
                    messageText = ""
                }
                .disabled(messageText.isEmpty)
            }
            .padding()
        }
        .navigationTitle("Group " + String(groupId.prefix(8)))
    }
}

struct CreateGroupSheet: View {
    @Binding var groupName: String
    @Binding var members: String
    let onCreate: () -> Void

    var body: some View {
        NavigationStack {
            Form {
                Section("Group Name") {
                    TextField("My Group", text: $groupName)
                }
                Section("Members (comma-separated Node IDs)") {
                    TextField("abc123..., def456...", text: $members)
                        .font(.system(.body, design: .monospaced))
                }
            }
            .navigationTitle("Create Group")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Create", action: onCreate)
                        .disabled(groupName.isEmpty)
                }
            }
        }
    }
}

#Preview {
    GroupsView()
        .environmentObject(TomNodeService.shared)
}
