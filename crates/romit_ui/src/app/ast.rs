//! Syntax tree for Romit apps.

use super::expr::Expr;

#[derive(Debug, Clone, Default)]
pub struct App {
    pub name: String,
    pub theme: Vec<ThemeEntry>,
    pub states: Vec<StateVar>,
    pub computed: Vec<Computed>,
    pub actions: Vec<ActionDef>,
    pub screens: Vec<Screen>,
    pub views: Vec<Component>,
    pub events: Vec<EventBlock>,
    /// `document notes, title`: state saved/opened as a file (DocumentGroup)
    pub document: Vec<String>,
    pub document_name: Option<Expr>,
    pub document_line: usize,
}

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub key: String,
    pub value: Expr,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct StateVar {
    pub name: String,
    pub init: Expr,
    pub saved: bool,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Computed {
    pub name: String,
    pub value: Expr,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ActionDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Action>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Screen {
    pub name: String,
    pub title: Option<Expr>,
    pub subtitle: Option<Expr>,
    pub params: Vec<String>,
    pub tab_icon: Option<Expr>,
    /// small title in the bar instead of a large title (`inline`)
    pub inline: bool,
    /// opened from a gear button / ⌘, (`settings`, like SwiftUI's Settings scene)
    pub settings: bool,
    /// (placement, items): leading, trailing, title, bottom
    pub toolbar: Vec<(String, Vec<Node>)>,
    /// `sidebar` block: a split view (NavigationSplitView)
    pub sidebar: Vec<Node>,
    pub body: Vec<Node>,
    pub events: Vec<EventBlock>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Node>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Node {
    El(Element),
    If {
        branches: Vec<(Option<Expr>, Vec<Node>)>,
        line: usize,
    },
    For {
        var: String,
        iter: Expr,
        body: Vec<Node>,
        line: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Element {
    pub kind: String,
    pub args: Vec<Expr>,
    pub kw: Vec<(String, Vec<Expr>)>,
    pub mods: Vec<Modifier>,
    pub events: Vec<EventBlock>,
    pub children: Vec<Node>,
    pub line: usize,
}

impl Element {
    pub fn kw(&self, name: &str) -> Option<&Vec<Expr>> {
        self.kw.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }
    pub fn kw1(&self, name: &str) -> Option<&Expr> {
        self.kw(name).and_then(|v| v.first())
    }
}

#[derive(Debug, Clone)]
pub struct Modifier {
    pub name: String,
    pub values: Vec<Expr>,
    pub cond: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct EventBlock {
    /// tap, doubletap, longpress, hover, appear, change, submit, delete, every
    pub kind: String,
    pub arg: Option<Expr>,
    pub millis: Option<f64>,
    pub actions: Vec<Action>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Action {
    Set(Expr, Expr),
    Change(Expr, char, Expr),
    Toggle(Expr),
    Add(Expr, Expr),
    Remove(Expr, Expr),
    RemoveWhere(Expr, Expr),
    Clear(Expr),
    Sort(Expr, Option<String>, bool),
    Shuffle(Expr),
    GoTo(String, Vec<Expr>),
    GoBack,
    GoHome,
    /// screen, values, style (sheet/half/full/popover/inspector), locked
    Show(String, Vec<Expr>, String, bool),
    /// `open window Screen with …`
    OpenWindow(String, Vec<Expr>),
    /// `open file into x [as text|json|image]`
    OpenFile(Expr, String),
    /// `save x to file "name.json"`
    SaveFile(Expr, Expr),
    /// document File menu commands: new / open / save
    Document(String),
    Close,
    Alert(Expr, Option<Expr>),
    Ask(Expr, Option<Expr>, Vec<Action>),
    Toast(Expr),
    Wait(f64),
    Open(Expr),
    Copy(Expr),
    Share(Expr),
    Say(Expr),
    Load(Expr, Expr),
    Vibrate,
    Run(String, Vec<Expr>),
    If(Vec<(Option<Expr>, Vec<Action>)>),
    For(String, Expr, Vec<Action>),
    Animate(Vec<Action>),
    /// Keep the line number for diagnostics.
    At(usize, Box<Action>),
}
