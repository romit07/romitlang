// Runs extension.js against a small stand-in for the VS Code API and the real
// `romit` compiler (ROMIT_BIN, default: ../../target/debug/romit). Skips if no binary.
"use strict";
const fs = require("fs");
const path = require("path");
const Module = require("module");

const bin = process.env.ROMIT_BIN || path.join(__dirname, "../../../target/debug/romit");
if (!fs.existsSync(bin)) {
  console.log(`extension test skipped (no romit binary at ${bin})`);
  process.exit(0);
}

class Range { constructor(l1, c1, l2, c2) { this.start = { line: l1, character: c1 }; this.end = { line: l2, character: c2 }; } }
class Diagnostic { constructor(range, message, severity) { Object.assign(this, { range, message, severity }); } }
const sets = [];
const vscode = {
  Range, Diagnostic,
  DiagnosticSeverity: { Error: 0, Warning: 1 },
  CompletionItemKind: { Class: 6, Property: 9, Function: 2, Keyword: 13, Operator: 23, Constant: 20, Text: 0 },
  workspace: { getConfiguration: () => ({ get: (k, d) => (k === "path" ? bin : d) }) },
  window: { showInformationMessage: () => Promise.resolve() },
  env: {}, Uri: { parse: (x) => x },
};
const realLoad = Module._load;
Module._load = function (req, ...rest) { return req === "vscode" ? vscode : realLoad.call(this, req, ...rest); };
const ext = require("../extension.js");

function fakeDoc(file, text) {
  const lines = text.split("\n");
  return { languageId: "romit", uri: { scheme: "file", fsPath: file, toString: () => file }, lineCount: lines.length, lineAt: (i) => ({ text: lines[i] }), getText: () => text };
}
function checkText(file, text) {
  return new Promise((resolve) => {
    const doc = fakeDoc(file, text);
    ext.check(doc, { set: (uri, diags) => { sets.push(diags); resolve(diags); } });
  });
}

(async () => {
  let failed = 0;
  const expect = (cond, msg) => { if (!cond) { failed++; console.error("FAIL " + msg); } };
  const tmp = path.join(__dirname, "fixture.rmt");

  let d = await checkText(tmp, "count = 1\nsay cuont\n");
  expect(d.length === 1, `one script error, got ${d.length}`);
  expect(d[0] && d[0].code === "E108" && d[0].range.start.line === 1 && d[0].range.start.character === 4, `E108 at 2:5 → ${JSON.stringify(d[0] && d[0].range)}`);
  expect(d[0] && d[0].range.end.character === 9, "underline covers `cuont`");
  expect(d[0] && /check the spelling/.test(d[0].message), "help text included");

  d = await checkText(tmp, "use ui\nscreen Home\n  buton \"x\"\n");
  expect(d.length === 1 && d[0].code === "U001" && /did you mean `button`/.test(d[0].message), "app error with suggestion");

  d = await checkText(tmp, "use ui\nstate\n  n is 0\nscreen Home\n  button \"+\" do n += 1\n");
  expect(d.length === 0, `valid app has no errors, got ${JSON.stringify(d.map((x) => x.message))}`);

  d = await checkText(tmp, "say \"unsaved {1 + 1}\"\n");
  expect(d.length === 0, "checks unsaved text from stdin");

  const items = require("../completions.json");
  expect(items.element.includes("button") && items.modifier.includes("bold") && items.action.includes("toast"), "completion word lists");

  if (failed) { console.error(`${failed} extension check(s) failed`); process.exit(1); }
  console.log("extension ok");
})();
