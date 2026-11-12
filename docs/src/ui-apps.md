# Building apps

Romit apps are **reactive UIs in the SwiftUI style** that look native on both iPhone and Android: you describe what the screen shows for the current data, and Romit redraws it whenever the data changes. One `.rmt` file compiles to one self-contained `index.html` that runs in any browser, with no build tools, frameworks, or `node_modules`.

```text
use ui

app Counter
  state
    count is 0

  screen Home
    column center gap 20
      text "{count}" size 72 bold
      row
        button "−" do count -= 1
        button "+" do count += 1
          style prominent
```

```bash
romit run counter.rmt      # live preview at http://127.0.0.1:3847 (refresh after edits)
romit build counter.rmt    # writes dist/index.html
romit check counter.rmt    # errors only, no output
```

The [playground](https://romitlang.org/playground/) previews apps in the browser too.

## The shape of an app

| Block | What it does | SwiftUI |
|-------|--------------|---------|
| `app Name` | Names the app (optional; screens can sit at the top level) | `@main struct App` |
| `theme` | Accent color, fonts, corners, dark mode | `.tint`, `.preferredColorScheme` |
| `state` | Values the UI reads and changes | `@State`, `@AppStorage` |
| `computed` | Values worked out from state, always up to date | computed properties |
| `action name` | A reusable list of steps | methods |
| `screen Name` | A page; the first screen is the home screen | `View` in a `NavigationStack` |
| `view Name with a, b` | A reusable piece of UI | custom `View` struct |
| `on start` / `every 1 second` | App-wide events and timers | `.onAppear`, `Timer` |
| `document notes` | State saved and opened as a file | `DocumentGroup` |

Indent children two spaces. Anything after `#` or `note` is a comment.

## State

```text
state
  name is ""
  count is 0
  dark is false
  todos is list saved
    title "Try Romit", done true
    title "Build an app", done false
```

- `saved` keeps a value on the device between visits (like `@AppStorage`).
- `list` starts a list. Indented lines add items. A line like `title "x", done false` is a **record** with named fields.
- Names can't be modifier words (`size`, `color`, …). The checker suggests alternatives, such as `selectedSize`.

## Computed values

```text
computed
  left is count of todos where not done
  greeting is "Hello, {name}!"
```

Computed values update automatically and can't be assigned (**U112**).

## Showing things

Write an element name, then its content, then any modifiers, all on one line or on indented lines:

```text
text "Welcome back, {name}" bold color secondary
text "Total"
  size 24
  color red if total is below 0
```

`{…}` inside text inserts a value; it can hold any expression: `"{count of todos} left"`, `"{price as money}"`.

### Layout

| Romit | SwiftUI | Notes |
|-------|---------|-------|
| `column` | `VStack` | top to bottom |
| `row` | `HStack` | left to right |
| `stack` | `ZStack` | layered on top of each other |
| `grid 3` | `LazyVGrid` | `grid adaptive` with `min 140` for responsive |
| `wrap` | flow layout | wraps onto new lines |
| `scroll` / `scroll horizontal` | `ScrollView` | |
| `spacer` / `divider` | `Spacer` / `Divider` | |
| `list` | `List` | rows get separators; `on delete` adds a Delete button |
| `section "Title" footer "…"` | `Section` | inside `list` or `form` |
| `form` | `Form` | grouped settings style |
| `card "Title" "Subtitle"` | — | rounded surface |
| `box "Title"` | `GroupBox` | |
| `disclosure "More"` | `DisclosureGroup` | |
| `group` | `Group` | |

### Content

| Romit | SwiftUI |
|-------|---------|
| `text`, `title`, `heading`, `subheading`, `caption` | `Text` with `.font(...)` |
| `label "Inbox" icon "tray"` | `Label` |
| `icon "star.fill"` | `Image(systemName:)` (common SF Symbol names map to glyphs) |
| `image "https://…"` | `AsyncImage` |
| `video "movie.mp4"` | `VideoPlayer` |
| `circle`, `rectangle`, `rounded`, `capsule`, `ellipse` | shapes; color them with `fill` |
| `progress done of total` / `spinner` | `ProgressView` |
| `gauge value from 0 to 100 "Label"` | `Gauge` |
| `empty "No items" icon "tray" message "…"` | `ContentUnavailableView` |

### Controls

Controls change state through `bind` (SwiftUI's `$binding`):

| Romit | SwiftUI |
|-------|---------|
| `button "Save" do save` | `Button` |
| `link "Docs" to "https://…"` | `Link` |
| `link "Details" to Detail with item` | `NavigationLink` |
| `toggle "Dark mode" bind dark` | `Toggle` |
| `field "Name" bind name` | `TextField` |
| `secure "Password" bind password` | `SecureField` |
| `editor "Notes" bind notes` | `TextEditor` |
| `search "Search" bind query` | `.searchable` |
| `slider "Volume" bind volume from 0 to 100 step 5` | `Slider` |
| `stepper "Guests" bind guests from 1 to 10` | `Stepper` |
| `picker "Size" bind cupSize options "S", "M", "L"` | `Picker` (segmented for ≤ 4 options, a menu otherwise; force with `style menu`) |
| `date "Due" bind due` | `DatePicker` |
| `colorpicker "Tint" bind tintColor` | `ColorPicker` |
| `menu "Sort" icon "list.bullet"` + indented buttons | `Menu` |
| `share text "Share"` | `ShareLink` |

`bind` also reaches into records: `toggle bind todo.done`.

### Showing things conditionally and in loops

```text
if todos is empty
  empty "Nothing to do" icon "checkmark.circle"
otherwise
  list
    for each todo in todos where not done
      TodoRow todo
```

`for each n in 1 to 5` counts. Inside a loop, `index` is the position (starting at 1).

## Modifiers

Modifiers go after the content or on indented lines. Add `if <condition>` (or `unless`) to any modifier.

| Romit | SwiftUI |
|-------|---------|
| `padding`, `padding 24`, `padding horizontal 12` | `.padding` |
| `gap 12` | stack `spacing:` |
| `width 200`, `height 80`, `size 48`, `maxwidth 600`, `stretch` | `.frame` |
| `bold`, `italic`, `underline`, `strike`, `mono`, `serif`, `uppercase` | text styles |
| `font title` / `font "Inter"`, `weight semibold`, `lines 2` | `.font`, `.fontWeight`, `.lineLimit` |
| `color red`, `background "#eef"`, `gradient blue purple`, `tint green` | `.foregroundStyle`, `.background`, `.tint` |
| `fill orange`, `stroke gray` | shape fill and stroke |
| `corner 12`, `border gray 2`, `shadow`, `clip circle` | `.clipShape`, `.border`, `.shadow` |
| `opacity 0.5`, `blur 4`, `rotate 45`, `scale 1.2`, `offset 10 -4`, `z 2` | the same names |
| `center`, `leading`, `trailing`, `align top` | alignment |
| `hidden`, `disabled` | `.hidden()`, `.disabled` |
| `style prominent / bordered / plain / glass / capsule / large / fab`, `role destructive` | `.buttonStyle`, `role:` |
| `style checkbox` (toggles), `style segmented / menu` (pickers) | `.toggleStyle`, `.pickerStyle` |
| `badge 3`, `help "tooltip"`, `describe "label"`, `selectable`, `focused` | `.badge`, `.help`, `.accessibilityLabel`, `.textSelection`, `@FocusState` |
| `transition slide`, `animated` | `.transition`, `.animation` |
| `keyboard email` | `.keyboardType` |

Colors: `red orange yellow green mint teal cyan blue indigo purple pink brown gray black white clear`, plus `primary secondary accent danger success warning`, or any `"#hex"`.

## Events and actions

```text
button "Add" do add draft to names           # one action
button "Reset"
  on tap                                     # several actions
    count = 0
    toast "Reset"
text "Hold me"
  on long press do vibrate
```

Events: `on tap`, `on double tap`, `on long press`, `on hover`, `on submit` (fields, forms), `on change` (controls), `on delete` (list rows), `on appear` / `on disappear` (screens), `on change of x` (screens and the app), `every 2 seconds`, `on start`, `on background`, `on foreground`, `on url` (app).

Actions:

| Action | Example |
|--------|---------|
| change a value | `count = 0`, `set count to 0`, `count += 1`, `increase score by 10`, `toggle dark` |
| lists | `add draft to names`, `add to todos with title draft, done false`, `remove todo from todos`, `remove from todos where done`, `clear todos`, `sort todos by title`, `shuffle cards` |
| navigation | `go to Detail with todo`, `go back`, `go home` |
| sheets | `show Compose`, `show X as half / full / popover / inspector`, `close` |
| windows and files | `open window Screen with x`, `open file into x as json`, `save x to file "x.json"`, `save document`, `open document`, `new document` |
| feedback | `alert "Saved"`, `alert "Oops" message "Try again"`, `toast "Copied"`, `vibrate` |
| confirmation | `ask "Delete everything?"` with indented actions to run on OK |
| time | `wait 1 second` |
| outside world | `open "https://…"`, `copy text`, `share text`, `load "https://api…" into posts`, `say value` (console) |
| logic | `if … / otherwise`, `for each … in …`, `repeat 3 times`, `run myAction`, `animate` |

## Screens and navigation

```text
screen Home "My Notes" subtitle "{count of notes} notes" tab "house"
  toolbar
    button "New" icon "plus" do show Compose
  ...

screen Detail with note titled note.title
  text note.body

screen Compose "New note" inline
  ...
```

- The first screen is the home screen.
- `go to Detail with note` pushes a screen with a back button. `go back` and `go home` return.
- Screens with `tab "icon"` form a tab bar (`TabView`).
- `subtitle …` adds a line under the title (`navigationSubtitle`). `inline` keeps the title small (`.navigationBarTitleDisplayMode(.inline)`).
- Every screen has an address. The browser's back and forward buttons work, and a link such as `app.html#/Detail~…` opens that screen directly (`NavigationPath`, deep links).

### Toolbars

```text
screen Editor
  toolbar                     # trailing (right), the default
    button "Done" do close
  toolbar leading
    button "Cancel" do go back
  toolbar title               # replaces the title (principal)
    picker bind mode options "Edit", "Preview"
  toolbar bottom              # bottom bar
    button "Share" icon "square.and.arrow.up" do share text
```

### Split view (sidebar)

A `sidebar` block turns a screen into a `NavigationSplitView`. On wide screens the sidebar sits on the left and links in it fill the detail column. On phones the sidebar is the first page and links push, as on an iPhone.

```text
screen Mailboxes "Mail"
  sidebar
    list
      for each f in folders
        link f to Folder with f
  empty "Pick a mailbox" icon "envelope"      # shown until something is picked

screen Folder with box titled box
  ...
```

The screen's toolbars move onto the sidebar in split view.

### Sheets, covers, popovers, inspectors

| Romit | SwiftUI |
|-------|---------|
| `show Compose` | `.sheet` |
| `show Compose as half` | `.sheet` + `.presentationDetents([.medium])` |
| `show Compose as full` (or `cover Compose`) | `.fullScreenCover` |
| `show Filters as popover` | `.popover` (anchored to what was tapped) |
| `show Details with item as inspector` | `.inspector` (side panel on wide screens, half sheet on phones) |
| add `locked`, e.g. `show Terms as sheet locked` | `.interactiveDismissDisabled()`: only a `close` action dismisses it |
| `close` | `dismiss()` |

### Settings

Add `settings` to a screen line (`screen Prefs "Settings" settings`). Romit puts a ⚙ button on the home screen and opens it with ⌘, or Ctrl+, like SwiftUI's `Settings` scene.

### Windows

`open window Message with m` opens a screen in a new browser tab or window (`openWindow`). Values passed with `with` are linked back to your saved lists, so edits made in either window go to the same item.

### Search suggestions and scopes

```text
search "Search mail" bind query suggest "Ada", "Grace" scopes "All", "Starred" scope scope
```

`suggest` lists completions (`searchSuggestions`), and `scopes` with `scope` adds a scope bar (`searchScopes`).

### App lifecycle and links

```text
on start                      # once, when the app opens
on background                 # the tab or app was hidden (scenePhase .background)
  lastSeen = now
on foreground                 # back again (scenePhase .active)
  toast "Welcome back"
on url                        # opened with a link the app doesn't route itself (onOpenURL)
  say url
```

`on background` and `on foreground` also work on a single screen.

### Files and documents

```text
document notes, title named "Notes.json"      # DocumentGroup: File ▸ New / Open… / Save, ⌘O, ⌘S

button "Import" do open file into notes as json        # fileImporter (text, json, image)
button "Export" do save notes to file "notes.json"     # fileExporter
button "Save" do save document                          # also: open document, new document
```

`document` names the state values that make up a document. The home screen gets a **File** menu, and `new document` resets those values to their starting state.

## Reusable views

```text
view TodoRow with todo
  row
    toggle bind todo.done
      style checkbox
    text todo.title
      strike if todo.done

screen Home
  for each t in todos
    TodoRow t
```

## Expressions

| Write | Means |
|-------|-------|
| `a + b`, `a plus b`, `minus`, `times`, `divided by`, `remainder` | math (`+` joins text) |
| `is`, `is not`, `is above`, `is below`, `is at least`, `is at most`, `==`, `<` … | comparisons |
| `and`, `or`, `not` | logic |
| `is empty`, `is not empty` | blank text, empty list, or nothing |
| `list contains x`, `x is in list` | membership (text: case-insensitive "contains") |
| `count of`, `sum of`, `average of`, `min of`, `max of`, `first of`, `last of` | list summaries |
| `upper of`, `lower of`, `trim of`, `round of`, `floor of`, `abs of`, `reverse of`, `unique of`, `words of` | helpers |
| `todos where done` | filter (field names refer to each item) |
| `todos sorted by title`, `… sorted by date descending` | sort |
| `item 2 of names` | position (starting at 1) |
| `price as money`, `as percent`, `as date`, `as time`, `as ago`, `as number` | formatting |
| `if dark then "Night" else "Day"` | choose a value |
| `random 1 to 6`, `now`, `today` | built-ins |
| `with title "x", done false` | a record |

## Looks native on iPhone and Android

The same `.rmt` file looks at home on every device. You don't change your code:

| Device | Look |
|--------|------|
| iPhone, iPad, Mac | **Apple**: large titles, `‹ Back` buttons, inset grouped lists, iOS switches and segmented controls, centered alerts |
| Android, Windows, Linux, Chromebook | **Material 3** (Material You): top app bar with a ← arrow, navigation bar with pill indicators, pill buttons, Material switches, outlined text fields, segmented buttons with ✓, bottom sheets, snackbars, Material dialogs |

To pick one look for every device:

```text
theme
  look material        # or: apple, auto (the default)
```

To preview the other look, add `?look=material` or `?look=apple` to the app's address, or use the **Look** menu in the playground. `style fab` turns a button into a floating action button (round on Apple, rounded-square on Material).

## Theme

```text
theme
  accent purple
  corner 16
  font "Inter"
  dark when darkMode       # or: appearance dark / light / auto
```

Settings: `look accent background surface text muted separator danger success warning corner spacing font titlefont width appearance dark`.

## Errors

Every mistake gets a code, the line, and a hint. Misspellings get "did you mean":

```text
error[U101]: unknown name `cuont`; did you mean `count`?
  --> todo.rmt:9:5
   = help: add it to `state` (e.g. `cuont is 0`), or check the spelling
```

See [Error codes](reference/errors.md) for the full list, and [SwiftUI → Romit](reference/swiftui-map.md) for a side-by-side map.

## Examples

- [`examples/apps/counter.rmt`](https://github.com/romit07/romitlang/blob/main/examples/apps/counter.rmt): state, buttons, conditional modifiers
- [`examples/apps/todos.rmt`](https://github.com/romit07/romitlang/blob/main/examples/apps/todos.rmt): saved lists, computed values, navigation, confirmation
- [`examples/apps/notes.rmt`](https://github.com/romit07/romitlang/blob/main/examples/apps/notes.rmt): search, sorting, menus, sheets, dates
- [`examples/apps/mail.rmt`](https://github.com/romit07/romitlang/blob/main/examples/apps/mail.rmt): split view, toolbars, every sheet style, popover, inspector, settings, windows, deep links, files, documents, scene phase
- [`examples/apps/showcase.rmt`](https://github.com/romit07/romitlang/blob/main/examples/apps/showcase.rmt): tabs, forms, every control, grids, timers, dark mode
