// Romit for VS Code: errors as you type, autocomplete, and run/preview commands.
// Highlighting and snippets are declarative (package.json); this file adds the rest.
"use strict";
const vscode = require("vscode");
const cp = require("child_process");
const path = require("path");

const WORDS = require("./completions.json");
const KIND = {
  element: "Class",
  modifier: "Property",
  action: "Function",
  keyword: "Keyword",
  operator: "Operator",
  constant: "Constant",
};
const DETAIL = {
  element: "Romit element",
  modifier: "Romit modifier",
  action: "Romit action",
  keyword: "Romit keyword",
  operator: "Romit word",
  constant: "Romit value",
};

let missingWarned = false;

/** Convert `romit check --json` output into VS Code diagnostics. */
function toDiagnostics(items, doc) {
  const out = [];
  for (const d of items) {
    // only show problems that belong to this file (not to files it `use`s)
    if (d.file && doc.uri.fsPath && path.resolve(d.file) !== path.resolve(doc.uri.fsPath)) continue;
    const line = Math.max(0, Math.min(doc.lineCount - 1, (d.line || 1) - 1));
    const text = doc.lineAt(line).text;
    const col = Math.max(0, Math.min(text.length, (d.column || 1) - 1));
    // underline the word at the error, or the rest of the line
    const word = /^[\w"#.]+/.exec(text.slice(col));
    const end = word ? col + word[0].length : Math.max(col + 1, text.length);
    const range = new vscode.Range(line, col, line, Math.min(end, text.length || col + 1));
    const message = d.help ? `${d.message}\n${d.help}` : d.message;
    const sev = d.severity === "warning" ? vscode.DiagnosticSeverity.Warning : vscode.DiagnosticSeverity.Error;
    const diag = new vscode.Diagnostic(range, message, sev);
    diag.source = "romit";
    diag.code = d.code;
    out.push(diag);
  }
  return out;
}

/** Run `romit check --json --stdin` on the document's current (possibly unsaved) text. */
function check(doc, collection) {
  if (doc.languageId !== "romit") return;
  const cfg = vscode.workspace.getConfiguration("romit");
  const exe = cfg.get("path") || "romit";
  const file = doc.uri.scheme === "file" ? doc.uri.fsPath : "untitled.rmt";
  const child = cp.spawn(exe, ["check", "--json", "--no-coverage", "--stdin", file], {
    cwd: doc.uri.scheme === "file" ? path.dirname(file) : undefined,
  });
  let stdout = "";
  child.stdout.on("data", (b) => (stdout += b));
  child.on("error", (err) => {
    if (err.code === "ENOENT" && !missingWarned) {
      missingWarned = true;
      vscode.window
        .showInformationMessage("Install the Romit compiler to see errors as you type.", "How to install")
        .then((pick) => pick && vscode.env.openExternal(vscode.Uri.parse("https://romitlang.org/install.html")));
    }
  });
  child.on("close", () => {
    try {
      collection.set(doc.uri, toDiagnostics(JSON.parse(stdout || "[]"), doc));
    } catch (e) {
      /* older romit without --json: leave diagnostics as they are */
    }
  });
  child.stdin.end(doc.getText());
}

function runInTerminal(doc) {
  if (!doc || doc.languageId !== "romit") return;
  const exe = vscode.workspace.getConfiguration("romit").get("path") || "romit";
  doc.save().then(() => {
    const term = vscode.window.terminals.find((t) => t.name === "Romit") || vscode.window.createTerminal("Romit");
    term.show();
    term.sendText(`${exe} run "${doc.uri.fsPath}"`);
  });
}

function activate(context) {
  const collection = vscode.languages.createDiagnosticCollection("romit");
  context.subscriptions.push(collection);
  const timers = new Map();
  const schedule = (doc, delay) => {
    if (doc.languageId !== "romit") return;
    if (!vscode.workspace.getConfiguration("romit").get("checkOnType", true) && delay > 0) return;
    clearTimeout(timers.get(doc.uri.toString()));
    timers.set(doc.uri.toString(), setTimeout(() => check(doc, collection), delay));
  };
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((d) => schedule(d, 0)),
    vscode.workspace.onDidChangeTextDocument((e) => schedule(e.document, 350)),
    vscode.workspace.onDidSaveTextDocument((d) => schedule(d, 0)),
    vscode.workspace.onDidCloseTextDocument((d) => collection.delete(d.uri)),
    vscode.languages.registerCompletionItemProvider("romit", {
      provideCompletionItems() {
        const items = [];
        for (const [kind, words] of Object.entries(WORDS)) {
          for (const w of words) {
            const item = new vscode.CompletionItem(w, vscode.CompletionItemKind[KIND[kind]] ?? vscode.CompletionItemKind.Text);
            item.detail = DETAIL[kind];
            items.push(item);
          }
        }
        return items;
      },
    }),
    vscode.commands.registerCommand("romit.run", () => runInTerminal(vscode.window.activeTextEditor && vscode.window.activeTextEditor.document)),
  );
  vscode.workspace.textDocuments.forEach((d) => schedule(d, 0));
}

function deactivate() {}

module.exports = { activate, deactivate, toDiagnostics, check };
