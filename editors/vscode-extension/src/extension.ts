import * as vscode from 'vscode';
import { execSync } from 'child_process';

function runlens(args: string): string {
  try {
    return execSync(`runlens ${args}`, { encoding: 'utf8', timeout: 30000 });
  } catch (e: any) {
    return `Error: ${e.stderr?.trim() || e.message}`;
  }
}

export function activate(context: vscode.ExtensionContext) {
  const init = vscode.commands.registerCommand('runlens.init', async () => {
    const ws = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    if (!ws) { vscode.window.showErrorMessage('No workspace open'); return; }
    const label = await vscode.window.showInputBox({ prompt: 'Session label (optional)' });
    const output = runlens(`init --dir "${ws}"${label ? ` --label "${label}"` : ''}`);
    vscode.window.showInformationMessage(output);
  });

  const record = vscode.commands.registerCommand('runlens.record', async () => {
    const cmd = await vscode.window.showInputBox({ prompt: 'Command to record' });
    if (!cmd) return;
    const ws = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '.';
    const output = runlens(`record --dir "${ws}" -- "${cmd}"`);
    vscode.window.showInformationMessage(`RunLens: ${output}`);
  });

  const list = vscode.commands.registerCommand('runlens.list', () => {
    const output = runlens('list');
    const panel = vscode.window.createOutputChannel('RunLens');
    panel.clear();
    panel.appendLine(output);
    panel.show();
  });

  const show = vscode.commands.registerCommand('runlens.show', async () => {
    const id = await vscode.window.showInputBox({ prompt: 'Session ID' });
    if (!id) return;
    const output = runlens(`show ${id}`);
    const panel = vscode.window.createOutputChannel('RunLens');
    panel.clear();
    panel.appendLine(output);
    panel.show();
  });

  const verify = vscode.commands.registerCommand('runlens.verify', async () => {
    const id = await vscode.window.showInputBox({ prompt: 'Session ID' });
    if (!id) return;
    const output = runlens(`verify ${id}`);
    vscode.window.showInformationMessage(`RunLens: ${output}`);
  });

  const redactions = vscode.commands.registerCommand('runlens.redactions', async () => {
    const id = await vscode.window.showInputBox({ prompt: 'Session ID' });
    if (!id) return;
    const output = runlens(`redactions ${id}`);
    const panel = vscode.window.createOutputChannel('RunLens');
    panel.clear();
    panel.appendLine(output);
    panel.show();
  });

  context.subscriptions.push(init, record, list, show, verify, redactions);
}