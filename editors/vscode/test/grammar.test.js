// Tokenizes Romit with the same engine VS Code uses (vscode-textmate + Oniguruma)
// and checks that key constructs get the expected scopes.
"use strict";
const fs = require("fs");
const path = require("path");
const vsctm = require("vscode-textmate");
const oniguruma = require("vscode-oniguruma");

const root = path.join(__dirname, "..");
const wasm = fs.readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm")).buffer;

async function main() {
  await oniguruma.loadWASM(wasm);
  const registry = new vsctm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (s) => new oniguruma.OnigScanner(s),
      createOnigString: (s) => new oniguruma.OnigString(s),
    }),
    loadGrammar: async () =>
      vsctm.parseRawGrammar(fs.readFileSync(path.join(root, "syntaxes/romit.tmLanguage.json"), "utf8"), "romit.tmLanguage.json"),
  });
  const grammar = await registry.loadGrammar("source.romit");

  // Return [text, scopes] for every token in a (multi-line) source.
  const tokenize = (src) => {
    let stack = vsctm.INITIAL;
    const out = [];
    for (const line of src.split("\n")) {
      const r = grammar.tokenizeLine(line, stack);
      for (const t of r.tokens) out.push([line.slice(t.startIndex, t.endIndex), t.scopes]);
      stack = r.ruleStack;
    }
    return out;
  };
  const scopeOf = (src, text) => {
    const hit = tokenize(src).find(([t]) => t.trim() === text);
    if (!hit) throw new Error(`token ${JSON.stringify(text)} not found in ${JSON.stringify(src)}`);
    return hit[1].join(" ");
  };

  let failed = 0;
  const expect = (src, text, scope) => {
    const got = scopeOf(src, text);
    if (!got.includes(scope)) {
      failed++;
      console.error(`FAIL ${JSON.stringify(text)} in ${JSON.stringify(src)}\n  want ${scope}\n  got  ${got}`);
    }
  };

  // declarations
  expect("use ui", "use", "keyword.control.import");
  expect("app Todos", "app", "storage.type");
  expect("app Todos", "Todos", "entity.name.type");
  expect("  screen Detail with todo", "Detail", "entity.name.type");
  expect("  view TodoRow with todo", "TodoRow", "entity.name.type");
  expect("action addTodo", "addTodo", "entity.name.function");
  expect("fn double n", "double", "entity.name.function");
  expect("sum: a, b", "sum", "entity.name.function");
  expect("  state", "state", "storage.type");
  expect("  toolbar bottom", "toolbar", "storage.type");
  expect("    on tap", "tap", "keyword.control.event");
  expect("  on background", "background", "keyword.control.event");
  expect("  every 1 second", "every", "keyword.control.event");
  expect("spec Counter version 1", "Counter", "entity.name.type");
  // elements, modifiers, components, actions
  expect("    button \"Add\" do addTodo", "button", "entity.name.tag");
  expect("    subheading m.from", "subheading", "entity.name.tag");
  expect("      bold", "bold", "support.function.modifier");
  expect("    text \"Hi\" size 24 color red if done", "size", "support.function.modifier");
  expect("    text \"Hi\" size 24 color red if done", "if", "keyword.control");
  expect("          TodoRow todo", "TodoRow", "entity.name.type");
  expect("    count += 1", "+=", "keyword.operator");
  expect("    add draft to todos", "add", "keyword.other.action");
  expect("    go back", "go", "keyword.other.action");
  expect("  button \"x\" do toast \"hi\"", "toast", "keyword.other.action");
  expect("  button \"x\" do toast \"hi\"", "do", "keyword.control");
  // expressions
  expect("left is count of todos where not done", "where", "keyword.operator.word");
  expect("x is above 3 and y", "above", "keyword.operator.word");
  expect("    done is true", "true", "constant.language");
  expect("  price is 9.99", "9.99", "constant.numeric");
  expect("  accent \"#7c3aed\"", "#7c3aed", "string.quoted");
  expect("      background #ff9500", "#ff9500", "constant.other.color");
  expect("    link todo.title to Detail with todo", "title", "variable.other.property");
  // strings and interpolation
  expect('say "Hi {name}!"', "name", "meta.embedded.expression");
  expect('say "Hi {name}!"', "{", "punctuation.section.interpolation.begin");
  expect('say "{count of todos} left"', "of", "keyword.operator.word");
  expect('say "{{literal}}"', "{{", "constant.character.escape");
  // comments
  expect("# a comment", "# a comment", "comment.line.number-sign");
  expect("note A to-do app", "note A to-do app", "comment.line.note");
  expect("say 1 // trailing", "// trailing", "comment.line.double-slash");
  expect("say 1 # trailing", "# trailing", "comment.line.number-sign");
  // braces outside text are an error in Romit
  expect("say (1 + 2)", "(", "invalid.illegal.bracket");

  // Every example in the repository tokenizes without "invalid" scopes.
  const exampleDirs = ["examples/beginner", "examples/apps", "examples/ui", "examples/advanced"];
  for (const dir of exampleDirs) {
    const abs = path.join(root, "../..", dir);
    for (const f of fs.readdirSync(abs).filter((n) => n.endsWith(".rmt"))) {
      const bad = tokenize(fs.readFileSync(path.join(abs, f), "utf8")).filter(([, s]) => s.some((x) => x.startsWith("invalid")));
      if (bad.length) {
        failed++;
        console.error(`FAIL ${dir}/${f}: invalid tokens ${JSON.stringify(bad.slice(0, 3).map(([t]) => t))}`);
      }
    }
  }

  if (failed) {
    console.error(`${failed} grammar check(s) failed`);
    process.exit(1);
  }
  console.log("grammar ok");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
