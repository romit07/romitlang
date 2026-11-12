#!/usr/bin/env python3
"""Generate syntaxes/romit.tmLanguage.json.

Element and modifier names are read from the compiler's own vocabulary
(crates/romit_ui/src/app/vocab.rs), so highlighting stays in sync with the
language. Run from editors/vscode:  python3 scripts/build_grammar.py
"""
import json
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent.parent
VOCAB = (HERE / "../../crates/romit_ui/src/app/vocab.rs").resolve().read_text()


def const_list(name):
    m = re.search(r"pub const %s: &\[&str\] = &\[(.*?)\];" % name, VOCAB, re.S)
    return re.findall(r'"([a-z0-9]+)"', m.group(1))


def alt(words):
    """Longest-first alternation so `subheading` wins over `heading`."""
    return "|".join(sorted(set(words), key=lambda s: (-len(s), s)))


ELEMENTS = const_list("ELEMENTS")
MODIFIERS = sorted(set(const_list("MODIFIERS") + re.findall(r'\("([a-z0-9]+)", "[a-z]+"\)', VOCAB)))
DECL_BLOCKS = ["state", "computed", "derived", "theme", "toolbar", "sidebar", "document", "remember", "data", "template"]
CONTROL = ["if", "otherwise", "else", "for", "each", "in", "while", "repeat", "times", "give", "stop", "skip", "do", "then", "unless", "when"]
ACTIONS = ["say", "print", "set", "toggle", "flip", "increase", "decrease", "add", "append", "insert", "remove", "delete",
           "clear", "reset", "sort", "shuffle", "go", "back", "home", "show", "present", "cover", "popover", "inspector",
           "close", "alert", "ask", "confirm", "toast", "notify", "wait", "open", "visit", "copy", "share", "load", "fetch",
           "into", "save", "export", "import", "vibrate", "run", "call", "animate", "window", "file"]
OP_WORDS = ["is", "isnt", "not", "and", "or", "plus", "minus", "times", "divided", "by", "remainder", "mod", "above",
            "below", "at", "least", "most", "more", "less", "than", "contains", "has", "where", "sorted", "descending",
            "ascending", "as", "of", "with", "to", "from", "bind", "options", "step", "suggest", "scopes", "scope",
            "message", "footer", "header", "icon", "placeholder", "named", "titled", "subtitle", "inline", "settings",
            "tab", "locked", "equals", "empty", "list", "random", "item"]
SPEC = ["spec", "version", "about", "enum", "config", "entity", "keyed", "event", "stores", "output", "define",
        "returns", "machine", "rule", "emit", "reject", "invariant", "limit", "per", "test", "given", "send", "expect",
        "emitted", "rejected", "final", "all", "any", "count", "sum", "min", "max"]
EVENTS = ["tap", "click", "press", "double", "long", "hover", "appear", "start", "load", "disappear", "change", "submit",
          "enter", "delete", "swipe", "background", "foreground", "active", "url"]
CONSTS = ["true", "false", "yes", "no", "nothing", "none", "now", "today", "index", "it", "url"]

NUMBER = {"name": "constant.numeric.romit", "match": r"\b\d[\d_]*(?:\.\d+)?\b"}
MEMBER = {"match": r"(\.)([A-Za-z_]\w*)",
          "captures": {"1": {"name": "punctuation.accessor.romit"}, "2": {"name": "variable.other.property.romit"}}}

grammar = {
    "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
    "name": "Romit",
    "scopeName": "source.romit",
    "fileTypes": ["rmt"],
    "patterns": [{"include": "#comments"}, {"include": "#declarations"}, {"include": "#line-start"}, {"include": "#expression"}],
    "repository": {
        "comments": {"patterns": [
            {"name": "comment.line.note.romit", "match": r"^\s*note(\s.*)?$"},
            {"name": "comment.line.number-sign.romit", "match": r"(?<![\w#\"])#(?=\s|#|$).*$"},
            {"name": "comment.line.double-slash.romit", "match": r"//.*$"},
        ]},
        "declarations": {"patterns": [
            {"match": r"^\s*(use)\s+(ui|template)\b(?:\s+(\w+))?",
             "captures": {"1": {"name": "keyword.control.import.romit"}, "2": {"name": "keyword.other.romit"},
                          "3": {"name": "entity.name.type.romit"}}},
            {"match": r"^\s*(app|screen|page|view|component|spec)\s+([A-Za-z_]\w*)",
             "captures": {"1": {"name": "storage.type.romit"}, "2": {"name": "entity.name.type.romit"}}},
            {"match": r"^\s*(action|fn|function|func)\s+([A-Za-z_]\w*)",
             "captures": {"1": {"name": "storage.type.function.romit"}, "2": {"name": "entity.name.function.romit"}}},
            {"match": r"^\s*([A-Za-z_]\w*)(:)(?=\s|$)",
             "captures": {"1": {"name": "entity.name.function.romit"}, "2": {"name": "punctuation.separator.romit"}}},
            {"match": r"^\s*(%s)\b" % alt(DECL_BLOCKS), "captures": {"1": {"name": "storage.type.romit"}}},
            {"match": r"^\s*(on)\s+(%s)\b" % alt(EVENTS),
             "captures": {"1": {"name": "keyword.control.event.romit"}, "2": {"name": "keyword.control.event.romit"}}},
            {"match": r"^\s*(every)\b", "captures": {"1": {"name": "keyword.control.event.romit"}}},
            {"match": r"^\s*(%s)\b" % alt(SPEC), "captures": {"1": {"name": "keyword.other.spec.romit"}}},
        ]},
        "line-start": {"patterns": [
            {"match": r"^\s*(%s)\b" % alt(ELEMENTS), "captures": {"1": {"name": "entity.name.tag.romit"}}},
            {"match": r"^\s*(%s)\b" % alt(MODIFIERS), "captures": {"1": {"name": "support.function.modifier.romit"}}},
            {"match": r"^\s*(%s)\b" % alt(ACTIONS), "captures": {"1": {"name": "keyword.other.action.romit"}}},
            {"match": r"^\s*([A-Z]\w*)\b", "captures": {"1": {"name": "entity.name.type.component.romit"}}},
        ]},
        "expression": {"patterns": [
            {"include": "#strings"},
            {"name": "constant.other.color.romit", "match": r"#[0-9a-fA-F]{3,8}\b"},
            NUMBER,
            {"match": r"\b(do)\s+(%s)\b" % alt(ACTIONS),
             "captures": {"1": {"name": "keyword.control.romit"}, "2": {"name": "keyword.other.action.romit"}}},
            {"name": "keyword.control.romit", "match": r"\b(%s)\b" % alt(CONTROL)},
            {"name": "constant.language.romit", "match": r"\b(%s)\b" % alt(CONSTS)},
            {"name": "keyword.operator.word.romit", "match": r"\b(%s)\b" % alt(OP_WORDS)},
            {"name": "support.function.modifier.romit", "match": r"(?<=\s)(%s)\b" % alt(MODIFIERS)},
            {"name": "entity.name.type.romit", "match": r"\b[A-Z]\w*\b"},
            MEMBER,
            {"name": "keyword.operator.romit", "match": r"\+=|-=|==|!=|<=|>=|[+\-*/%<>=]"},
            {"name": "punctuation.separator.comma.romit", "match": ","},
            {"name": "invalid.illegal.bracket.romit", "match": r"[()\[\]{}]"},
            {"name": "variable.other.romit", "match": r"\b[a-z_]\w*\b"},
        ]},
        "strings": {"patterns": [
            {"name": "string.quoted.double.romit", "begin": "\"", "end": "\"",
             "beginCaptures": {"0": {"name": "punctuation.definition.string.begin.romit"}},
             "endCaptures": {"0": {"name": "punctuation.definition.string.end.romit"}},
             "patterns": [{"include": "#string-content"}]},
            {"name": "string.quoted.double.romit", "begin": "“", "end": "”",
             "patterns": [{"include": "#string-content"}]},
        ]},
        "string-content": {"patterns": [
            {"name": "constant.character.escape.romit", "match": r"\{\{|\}\}|\\."},
            {"name": "meta.embedded.interpolation.romit", "begin": r"\{", "end": r"\}",
             "beginCaptures": {"0": {"name": "punctuation.section.interpolation.begin.romit"}},
             "endCaptures": {"0": {"name": "punctuation.section.interpolation.end.romit"}},
             "contentName": "meta.embedded.expression.romit",
             "patterns": [{"include": "#interp-expression"}]},
        ]},
        "interp-expression": {"patterns": [
            NUMBER,
            {"name": "keyword.operator.word.romit", "match": r"\b(%s)\b" % alt(OP_WORDS + CONTROL)},
            {"name": "constant.language.romit", "match": r"\b(%s)\b" % alt(CONSTS)},
            MEMBER,
            {"name": "keyword.operator.romit", "match": r"==|!=|<=|>=|[+\-*/%<>]"},
            {"name": "variable.other.romit", "match": r"\b[A-Za-z_]\w*\b"},
        ]},
    },
}

out = HERE / "syntaxes" / "romit.tmLanguage.json"
out.write_text(json.dumps(grammar, indent=2, ensure_ascii=False) + "\n")
print(f"wrote {out.relative_to(HERE)}: {len(ELEMENTS)} elements, {len(MODIFIERS)} modifiers")
