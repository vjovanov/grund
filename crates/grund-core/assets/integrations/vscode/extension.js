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

// Resolve a citation to `{path, line, body}` via `grund`. VS Code is the one
// client that can show declaration content on *hover* rather than on click
// (§FS-integrations.3.3), and a tooltip has to carry its text at provide time —
// so this is called while building links, not when one is clicked. Results are
// cached by citation because provideTerminalLinks runs per rendered line and the
// same citation scrolls past repeatedly; the cache is what keeps that cheap.
const resolved = new Map();

function resolve(id, cwd, brief) {
  const key = `${brief ? 'b:' : 'f:'}${id}`;
  if (resolved.has(key)) return Promise.resolve(resolved.get(key));
  const args = brief ? [id, '--brief', '--format', 'json'] : [id, '--format', 'json'];
  return new Promise((done) => {
    execFile('grund', args, { cwd }, (err, stdout) => {
      let value = null;
      if (!err) {
        try { value = JSON.parse(stdout); } catch { value = null; }
      }
      resolved.set(key, value);
      done(value);
    });
  });
}

function workspaceCwd() {
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

function activate(context) {
  const provider = vscode.window.registerTerminalLinkProvider({
    async provideTerminalLinks(ctx) {
      const cwd = workspaceCwd();
      const found = [];
      let m;
      CITATION.lastIndex = 0;
      while ((m = CITATION.exec(ctx.line)) !== null) {
        found.push({ index: m.index, text: m[0] });
      }
      // `--brief` is heading + first paragraph: enough to answer "what does this
      // say?" without pasting a whole section into a tooltip.
      const decls = await Promise.all(
        found.map((f) => resolve(f.text.replace(/^[^A-Za-z0-9]+/, ''), cwd, true))
      );
      return found.map((f, i) => {
        const decl = decls[i];
        return {
          startIndex: f.index,
          length: f.text.length,
          tooltip: decl?.body
            ? `${decl.body.trim()}\n\n${decl.path}:${decl.line}`
            : 'Open grund declaration',
          data: f.text,
        };
      });
    },
    async handleTerminalLink(link) {
      // Strip the marker without hardcoding one, and keep the `.<section>`
      // suffix: `grund <ID>.<section>` resolves to that section's own line,
      // where dropping it would always land on the declaration heading
      // (§FS-integrations.3.1). Shares the cache with the hover above, so a
      // click on something already hovered costs nothing.
      const id = link.data.replace(/^[^A-Za-z0-9]+/, '');
      const cwd = workspaceCwd();
      const decl = await resolve(id, cwd, false);
      if (!decl || !decl.path) {
        vscode.window.showWarningMessage(`grund: unknown id ${id}`);
        return;
      }
      // `path` is relative to the config root, which is the workspace folder
      // `grund` was run in (§FS-config.3.6).
      const abs = cwd
        ? vscode.Uri.joinPath(vscode.Uri.file(cwd), decl.path)
        : vscode.Uri.file(decl.path);
      const pos = new vscode.Position(Math.max(0, (decl.line || 1) - 1), 0);
      vscode.window.showTextDocument(abs, { selection: new vscode.Range(pos, pos) });
    },
  });
  context.subscriptions.push(provider);
}

function deactivate() {}

module.exports = { activate, deactivate };
