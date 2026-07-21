// Stub TypeScript entry for the VS Code extension. Real implementation
// is shipped in a follow-up. This file keeps `npm run compile` from
// completing prematurely when the project adopts this scaffold.
import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
    context.subscriptions.push(
        vscode.commands.registerCommand('runlens.init', () => {
            vscode.window.showInformationMessage('RunLens: init (not implemented in stub)');
        }),
        vscode.commands.registerCommand('runlens.record', () => {
            vscode.window.showInformationMessage('RunLens: record (not implemented in stub)');
        }),
        vscode.commands.registerCommand('runlens.list', () => {
            vscode.window.showInformationMessage('RunLens: list (not implemented in stub)');
        }),
        vscode.commands.registerCommand('runlens.showActive', () => {
            vscode.window.showInformationMessage('RunLens: showActive (not implemented in stub)');
        }),
        vscode.commands.registerCommand('runlens.verify', () => {
            vscode.window.showInformationMessage('RunLens: verify (not implemented in stub)');
        }),
        vscode.commands.registerCommand('runlens.compare', () => {
            vscode.window.showInformationMessage('RunLens: compare (not implemented in stub)');
        })
    );
}

export function deactivate() {}
