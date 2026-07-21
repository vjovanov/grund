// grund terminal citations — a VS Code TerminalLinkProvider that turns §<ID>
// citations printed in the integrated terminal into links. Clicking one runs
// `grund` to resolve the citation to its cited line and opens it
// (§FS-integrations). Nothing is published to a marketplace; this extension is
// materialized on disk by `grund integrations vscode --write`.
const vscode = require('vscode');
const { execFile } = require('child_process');

// The leading [^\w\s]{1,3} matches the citation marker without naming it:
// `[reference] marker` is per-repo while this extension is user-global and
// installed once, so hardcoding § would leave every repo with a custom marker
// silently unclickable. The leading punctuation is stripped before resolving.
const CITATION = /[^\w\s]{1,3}(?:[a-z][a-z0-9-]*\/)?[A-Z]+-[a-z0-9][a-z0-9-]*(?:\.[0-9]+)*/g;

function activate(context) {
  const provider = vscode.window.registerTerminalLinkProvider({
    provideTerminalLinks(ctx) {
      const links = [];
      let m;
      CITATION.lastIndex = 0;
      while ((m = CITATION.exec(ctx.line)) !== null) {
        links.push({
          startIndex: m.index,
          length: m[0].length,
          tooltip: 'Open grund declaration',
          data: m[0],
        });
      }
      return links;
    },
    handleTerminalLink(link) {
      // Strip the marker without hardcoding one, and keep the `.<section>`
      // suffix: `grund <ID>.<section>` resolves to that section's own line,
      // where dropping it would always land on the declaration heading
      // (§FS-integrations.3.1).
      const id = link.data.replace(/^[^A-Za-z0-9]+/, '');
      const cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
      execFile('grund', [id, '--format', 'json'], { cwd }, (err, stdout) => {
        if (err) {
          vscode.window.showWarningMessage(`grund: unknown id ${id}`);
          return;
        }
        let decl;
        try { decl = JSON.parse(stdout); } catch {
          vscode.window.showErrorMessage(`grund: could not parse resolution for ${id}`);
          return;
        }
        if (!decl || !decl.path) {
          vscode.window.showWarningMessage(`grund: unknown id ${id}`);
          return;
        }
        // `path` is relative to the config root, which is the workspace folder
        // `grund` was run in (§FS-config.3.6).
        const abs = cwd ? vscode.Uri.joinPath(vscode.Uri.file(cwd), decl.path) : vscode.Uri.file(decl.path);
        const pos = new vscode.Position(Math.max(0, (decl.line || 1) - 1), 0);
        vscode.window.showTextDocument(abs, { selection: new vscode.Range(pos, pos) });
      });
    },
  });
  context.subscriptions.push(provider);
}

function deactivate() {}

module.exports = { activate, deactivate };
