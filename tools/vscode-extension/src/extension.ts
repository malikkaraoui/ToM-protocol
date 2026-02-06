/**
 * ToM Protocol VS Code Extension
 *
 * Provides a chat interface and LLM integration for the ToM distributed network.
 *
 * Features:
 * - Chat webview panel for real-time messaging
 * - Network status monitoring
 * - Participant tree view
 * - Command palette integration
 *
 * @see CLAUDE.md for LLM integration guide
 */

import * as vscode from 'vscode';

let chatPanel: vscode.WebviewPanel | undefined;
let statusBarItem: vscode.StatusBarItem;

export function activate(context: vscode.ExtensionContext) {
  console.log('ToM Protocol extension activated');

  // Status bar item
  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBarItem.text = '$(comment-discussion) ToM: Disconnected';
  statusBarItem.tooltip = 'ToM Protocol - Click for status';
  statusBarItem.command = 'tom.showStatus';
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('tom.openChat', () => openChatPanel(context)),
    vscode.commands.registerCommand('tom.connect', () => connectToNetwork()),
    vscode.commands.registerCommand('tom.disconnect', () => disconnectFromNetwork()),
    vscode.commands.registerCommand('tom.showStatus', () => showNetworkStatus()),
  );

  // Register views
  context.subscriptions.push(vscode.window.registerWebviewViewProvider('tom.chatView', new ChatViewProvider(context)));

  context.subscriptions.push(
    vscode.window.registerTreeDataProvider('tom.participantsView', new ParticipantsProvider()),
  );

  context.subscriptions.push(vscode.window.registerTreeDataProvider('tom.networkView', new NetworkStatusProvider()));

  // Auto-connect if configured
  const config = vscode.workspace.getConfiguration('tom');
  if (config.get('autoConnect')) {
    connectToNetwork();
  }
}

export function deactivate() {
  disconnectFromNetwork();
}

function openChatPanel(context: vscode.ExtensionContext) {
  if (chatPanel) {
    chatPanel.reveal();
    return;
  }

  chatPanel = vscode.window.createWebviewPanel('tomChat', 'ToM Chat', vscode.ViewColumn.One, {
    enableScripts: true,
    retainContextWhenHidden: true,
  });

  chatPanel.webview.html = getChatWebviewContent(chatPanel.webview);

  chatPanel.onDidDispose(() => {
    chatPanel = undefined;
  });

  // Handle messages from webview
  chatPanel.webview.onDidReceiveMessage(
    (message) => {
      switch (message.command) {
        case 'sendMessage':
          handleSendMessage(message.to, message.text);
          break;
        case 'connect':
          connectToNetwork();
          break;
        case 'disconnect':
          disconnectFromNetwork();
          break;
      }
    },
    undefined,
    context.subscriptions,
  );
}

function connectToNetwork() {
  const config = vscode.workspace.getConfiguration('tom');
  const signalingUrl = config.get<string>('signalingUrl') ?? 'ws://localhost:3001';
  const username = config.get<string>('username');

  if (!username) {
    vscode.window
      .showInputBox({
        prompt: 'Enter your username for the ToM network',
        placeHolder: 'username',
      })
      .then((input) => {
        if (input) {
          config.update('username', input, vscode.ConfigurationTarget.Global);
          doConnect(signalingUrl, input);
        }
      });
    return;
  }

  doConnect(signalingUrl, username);
}

function doConnect(signalingUrl: string, username: string) {
  statusBarItem.text = '$(sync~spin) ToM: Connecting...';
  vscode.window.showInformationMessage(`ToM: Connecting to ${signalingUrl} as ${username}`);

  // Note: Actual connection would use TomClient from tom-sdk
  // For now, show mock connected state
  setTimeout(() => {
    statusBarItem.text = '$(check) ToM: Connected';
    vscode.window.showInformationMessage('ToM: Connected to network');
  }, 1000);
}

function disconnectFromNetwork() {
  statusBarItem.text = '$(comment-discussion) ToM: Disconnected';
  vscode.window.showInformationMessage('ToM: Disconnected from network');
}

function showNetworkStatus() {
  vscode.window.showInformationMessage(
    'ToM Network Status\n\nRole: client\nPeers: 0\nRelays: 0\n\n(Full status coming soon)',
  );
}

function handleSendMessage(to: string, text: string) {
  vscode.window.showInformationMessage(`ToM: Sending message to ${to}`);
  // Actual sending would use TomClient
}

function getChatWebviewContent(webview: vscode.Webview): string {
  return `
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ToM Chat</title>
  <style>
    body {
      font-family: var(--vscode-font-family);
      background-color: var(--vscode-editor-background);
      color: var(--vscode-foreground);
      padding: 0;
      margin: 0;
      height: 100vh;
      display: flex;
      flex-direction: column;
    }
    .header {
      padding: 12px 16px;
      border-bottom: 1px solid var(--vscode-panel-border);
      font-weight: bold;
    }
    .messages {
      flex: 1;
      overflow-y: auto;
      padding: 16px;
    }
    .message {
      margin-bottom: 8px;
      padding: 8px 12px;
      border-radius: 4px;
      max-width: 80%;
    }
    .message.sent {
      background-color: var(--vscode-button-background);
      color: var(--vscode-button-foreground);
      margin-left: auto;
    }
    .message.received {
      background-color: var(--vscode-input-background);
    }
    .input-area {
      padding: 12px 16px;
      border-top: 1px solid var(--vscode-panel-border);
      display: flex;
      gap: 8px;
    }
    input {
      flex: 1;
      background: var(--vscode-input-background);
      color: var(--vscode-input-foreground);
      border: 1px solid var(--vscode-input-border);
      padding: 8px 12px;
      border-radius: 4px;
      font-family: inherit;
    }
    button {
      background: var(--vscode-button-background);
      color: var(--vscode-button-foreground);
      border: none;
      padding: 8px 16px;
      border-radius: 4px;
      cursor: pointer;
    }
    button:hover {
      background: var(--vscode-button-hoverBackground);
    }
    .empty-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      color: var(--vscode-descriptionForeground);
    }
    .empty-state p {
      margin: 8px 0;
    }
  </style>
</head>
<body>
  <div class="header">ToM Protocol Chat</div>
  <div class="messages" id="messages">
    <div class="empty-state">
      <p>Welcome to ToM Protocol</p>
      <p>Connect to start chatting</p>
      <button onclick="connect()">Connect</button>
    </div>
  </div>
  <div class="input-area">
    <input type="text" id="messageInput" placeholder="Type a message..." disabled />
    <button id="sendBtn" disabled onclick="send()">Send</button>
  </div>
  <script>
    const vscode = acquireVsCodeApi();

    function connect() {
      vscode.postMessage({ command: 'connect' });
    }

    function send() {
      const input = document.getElementById('messageInput');
      const text = input.value.trim();
      if (!text) return;

      vscode.postMessage({
        command: 'sendMessage',
        to: 'selected-peer-id',
        text: text
      });

      input.value = '';
    }

    document.getElementById('messageInput').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') send();
    });
  </script>
</body>
</html>
  `;
}

// Chat View Provider
class ChatViewProvider implements vscode.WebviewViewProvider {
  constructor(private context: vscode.ExtensionContext) {}

  resolveWebviewView(webviewView: vscode.WebviewView) {
    webviewView.webview.options = {
      enableScripts: true,
    };
    webviewView.webview.html = getChatWebviewContent(webviewView.webview);
  }
}

// Participants Tree Provider
class ParticipantsProvider implements vscode.TreeDataProvider<ParticipantItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<ParticipantItem | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  getTreeItem(element: ParticipantItem): vscode.TreeItem {
    return element;
  }

  getChildren(): ParticipantItem[] {
    // Return empty for now - would be populated by TomClient
    return [new ParticipantItem('Not connected', '', 'offline')];
  }

  refresh(): void {
    this._onDidChangeTreeData.fire(undefined);
  }
}

class ParticipantItem extends vscode.TreeItem {
  constructor(
    public readonly username: string,
    public readonly nodeId: string,
    public readonly status: 'online' | 'stale' | 'offline',
  ) {
    super(username, vscode.TreeItemCollapsibleState.None);
    this.description = status;
    this.iconPath = new vscode.ThemeIcon(status === 'online' ? 'circle-filled' : 'circle-outline');
    this.contextValue = `participant-${status}`;
  }
}

// Network Status Tree Provider
class NetworkStatusProvider implements vscode.TreeDataProvider<StatusItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<StatusItem | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  getTreeItem(element: StatusItem): vscode.TreeItem {
    return element;
  }

  getChildren(): StatusItem[] {
    return [
      new StatusItem('Status', 'Disconnected'),
      new StatusItem('Role', 'N/A'),
      new StatusItem('Peers', '0'),
      new StatusItem('Relays', '0'),
    ];
  }

  refresh(): void {
    this._onDidChangeTreeData.fire(undefined);
  }
}

class StatusItem extends vscode.TreeItem {
  constructor(label: string, value: string) {
    super(label, vscode.TreeItemCollapsibleState.None);
    this.description = value;
  }
}
