const DEFAULT = 'say "Hello from Romit"\n';

const BUILT_IN = {
  hello: { name: "Hello (script)", file: "hello.rmt" },
  variables: { name: "Variables", file: "variables.rmt" },
  assign_equals: { name: "Assign with =", file: "assign_equals.rmt" },
  print_alias: { name: "print alias", file: "print_alias.rmt" },
  sum_fn: { name: "Function sum:", file: "sum_fn.rmt" },
  fn_aliases: { name: "fn / function aliases", file: "fn_aliases.rmt" },
  operators: { name: "Operators", file: "operators.rmt" },
  control_flow: { name: "Control flow", file: "control_flow.rmt" },
  factorial: { name: "Factorial", file: "factorial.rmt" },
  greet_default: { name: "Default parameter", file: "greet_default.rmt" },
  hello_world: { name: "Hello (spec)", file: "hello_world.rmt" },
  calculator: { name: "Calculator", file: "calculator.rmt" },
  fibonacci: { name: "Fibonacci", file: "fibonacci.rmt" },
  todo_list: { name: "Todo list", file: "todo_list.rmt" },
  traffic_light: { name: "Traffic light", file: "traffic_light.rmt" },
  lists: { name: "Lists and text", file: "lists.rmt" },
  app_counter: { name: "App: Counter (UI)", file: "counter.rmt" },
  app_todos: { name: "App: Todos (UI)", file: "todos.rmt" },
  app_notes: { name: "App: Notes (UI)", file: "notes.rmt" },
  app_showcase: { name: "App: Showcase (UI)", file: "showcase.rmt" },
  app_mail: { name: "App: Mail — split view, sheets, files (UI)", file: "mail.rmt" },
};

const editor = document.getElementById("editor");
const lineNums = document.getElementById("line-nums");
const output = document.getElementById("output");
const preview = document.getElementById("preview");
const statusEl = document.getElementById("status");
const examplesSel = document.getElementById("examples");

let wasmReady = null;

function updateLines() {
  const n = editor.value.split("\n").length;
  lineNums.textContent = Array.from({ length: n }, (_, i) => i + 1).join("\n");
}

function loadFromHash() {
  const h = location.hash.slice(1);
  if (!h) return false;
  try {
    editor.value = decodeURIComponent(escape(atob(h)));
    return true;
  } catch {
    return false;
  }
}

function saveHash() {
  const enc = btoa(unescape(encodeURIComponent(editor.value)));
  history.replaceState(null, "", "#" + enc);
}

async function initWasm() {
  const { default: init, run_program, check_program, test_program, romit_version, is_ui_program, build_ui_program } = await import(
    "./pkg/romit_wasm.js"
  );
  await init();
  statusEl.textContent = `Compiler WASM v${romit_version()} loaded`;
  return { run_program, check_program, test_program, is_ui_program, build_ui_program };
}

function showPreview(json) {
  const data = JSON.parse(json);
  if (data.ok) {
    output.classList.remove("error");
    output.textContent = "Live preview below — click around, it's a real app.";
    preview.hidden = false;
    preview.srcdoc = data.html;
  } else {
    preview.hidden = true;
    output.classList.add("error");
    output.textContent = data.diagnostics || "Error";
  }
}

function showResult(json) {
  preview.hidden = true;
  const data = JSON.parse(json);
  if (data.ok) {
    output.classList.remove("error");
    output.textContent = data.stdout || "(ok, no output)";
  } else {
    output.classList.add("error");
    output.textContent = data.diagnostics || "Error";
  }
  if (data.steps) {
    statusEl.textContent = `${data.steps} step(s)`;
  }
}

async function runAction(mode) {
  if (!wasmReady) return;
  output.textContent = "Running…";
  saveHash();
  const src = editor.value;
  const timeout = new Promise((_, rej) => setTimeout(() => rej(new Error("timeout")), 8000));
  if (wasmReady.is_ui_program && wasmReady.is_ui_program(src)) {
    showPreview(wasmReady.build_ui_program(src));
    return;
  }
  try {
    const json = await Promise.race([
      Promise.resolve().then(() => {
        if (mode === "check") return wasmReady.check_program(src);
        const head = src.trimStart();
        if (head.startsWith("spec")) return wasmReady.test_program(src);
        return wasmReady.run_program(src);
      }),
      timeout,
    ]);
    showResult(json);
  } catch (e) {
    output.classList.add("error");
    output.textContent = e.message === "timeout" ? "Timed out (8s limit)" : String(e);
  }
}

async function loadExample(id) {
  const meta = BUILT_IN[id];
  if (!meta) return;
  try {
    const res = await fetch(`examples/${meta.file}`);
    editor.value = await res.text();
  } catch {
    editor.value = `/* Could not load ${meta.file} */\n`;
  }
  updateLines();
  saveHash();
}

Object.entries(BUILT_IN).forEach(([id, ex]) => {
  const opt = document.createElement("option");
  opt.value = id;
  opt.textContent = ex.name;
  examplesSel.appendChild(opt);
});

examplesSel.addEventListener("change", () => loadExample(examplesSel.value));

document.getElementById("run-btn").addEventListener("click", () => runAction("run"));
document.getElementById("check-btn").addEventListener("click", () => runAction("check"));
document.getElementById("reset-btn").addEventListener("click", () => {
  editor.value = DEFAULT;
  updateLines();
  output.textContent = "";
  location.hash = "";
});

editor.addEventListener("input", updateLines);
editor.addEventListener("keydown", (e) => {
  if (e.key === "Tab") {
    e.preventDefault();
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    editor.value = editor.value.slice(0, start) + "  " + editor.value.slice(end);
    editor.selectionStart = editor.selectionEnd = start + 2;
    updateLines();
  }
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
    e.preventDefault();
    runAction("run");
  }
});

if (!loadFromHash()) {
  editor.value = DEFAULT;
}
updateLines();

initWasm()
  .then((api) => {
    wasmReady = api;
  })
  .catch((err) => {
    statusEl.textContent = "WASM failed to load";
    output.classList.add("error");
    output.textContent = String(err);
  });
