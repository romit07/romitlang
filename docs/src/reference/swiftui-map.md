# SwiftUI → Romit

This map walks through [Apple's SwiftUI documentation](https://developer.apple.com/documentation/swiftui) topic by topic and shows the Romit way to write each feature. Romit apps compile to the web, so platform-only features (watchOS complications, visionOS immersive spaces, AppKit/UIKit bridges) are out of scope and listed last.

**Status:** ✅ shipped · 🟡 partial · ⏳ planned

## Side by side

```swift
struct ContentView: View {
    @State private var count = 0
    var body: some View {
        VStack(spacing: 20) {
            Text("\(count)").font(.system(size: 72)).bold()
            HStack {
                Button("−") { count -= 1 }
                Button("+") { count += 1 }.buttonStyle(.borderedProminent)
            }
        }
    }
}
```

```text
state
  count is 0
screen Home
  column gap 20
    text "{count}" size 72 bold
    row
      button "−" do count -= 1
      button "+" do count += 1
        style prominent
```

## App structure

| SwiftUI | Romit | Status |
|---------|-------|--------|
| `App`, `WindowGroup` | `app Name` (optional) | ✅ |
| `NavigationStack`, `NavigationLink` | screens, `link "…" to Screen with value`, `go to Screen` | ✅ |
| `navigationDestination(for:)` | `screen Detail with item` | ✅ |
| `NavigationSplitView` | — | ⏳ |
| `TabView`, `Tab`, `.tabItem` | `screen Home tab "house"` | ✅ |
| `.sheet`, `.fullScreenCover`, `.popover` | `show Screen`, `close` | 🟡 one sheet style |
| `.alert` | `alert "Title" message "…"` | ✅ |
| `.confirmationDialog` | `ask "Delete?"` + indented actions | ✅ |
| `.toolbar`, `ToolbarItem` | `toolbar` block in a screen | ✅ |
| `.navigationTitle` | `screen Home "Title"` or `titled expr` | ✅ |
| `.searchable` | `search "Search" bind query` | ✅ |
| `@Environment(\.dismiss)` | `close`, `go back` | ✅ |
| `.fileImporter` / `.fileExporter` | — | ⏳ |

## Data and storage

| SwiftUI | Romit | Status |
|---------|-------|--------|
| `@State` | `state` block | ✅ |
| `@Binding`, `$value` | `bind value`, `bind item.field` | ✅ |
| `@Observable`, `@StateObject`, `@EnvironmentObject` | app-wide `state` is shared by every screen and view | ✅ |
| computed properties | `computed` block | ✅ |
| `@AppStorage`, `@SceneStorage` | `saved` after a state value | ✅ |
| `.onChange(of:)` | `on change of value` (screen or app) | ✅ |
| `.task { await … }`, `URLSession` | `on appear` + `load "url" into value` | ✅ |
| SwiftData / Core Data | `saved` lists of records | 🟡 |

## Views

| SwiftUI | Romit | Status |
|---------|-------|--------|
| `Text`, `.font(.largeTitle/.title/.headline/.caption)` | `text`, `title`, `heading`, `subheading`, `caption`, `font …` | ✅ |
| string interpolation `"\(x)"` | `"{x}"` | ✅ |
| `Label` | `label "Inbox" icon "tray"` | ✅ |
| `Image(systemName:)` | `icon "star.fill"` (common names) | 🟡 |
| `Image`, `AsyncImage` | `image "url"` | ✅ |
| `VideoPlayer` | `video "url"` | ✅ |
| `Button`, `ButtonRole`, `.buttonStyle` | `button`, `role destructive`, `style prominent/bordered/plain/glass` | ✅ |
| `Link`, `ShareLink` | `link "…" to "https://…"`, `share value "Share"` | ✅ |
| `Toggle`, `.toggleStyle` | `toggle "…" bind x`, `style checkbox` | ✅ |
| `TextField`, `SecureField`, `TextEditor` | `field`, `secure`, `editor` | ✅ |
| `.keyboardType`, `.onSubmit`, `@FocusState` | `keyboard email`, `on submit`, `focused` | ✅ |
| `Slider`, `Stepper` | `slider bind v from 0 to 100 step 5`, `stepper … from 1 to 10` | ✅ |
| `Picker`, `.pickerStyle` | `picker "…" bind x options a, b, c`, `style segmented/menu` | ✅ |
| `DatePicker`, `ColorPicker` | `date "…" bind d`, `colorpicker "…" bind c` | ✅ |
| `Menu` | `menu "…"` + indented buttons | ✅ |
| `ProgressView` | `progress x of total`, `spinner` | ✅ |
| `Gauge` | `gauge v from 0 to 100 "Label"` | ✅ |
| `ContentUnavailableView` | `empty "…" icon "…" message "…"` | ✅ |
| `Circle`, `Rectangle`, `RoundedRectangle`, `Capsule`, `Ellipse` | `circle`, `rectangle`, `rounded`, `capsule`, `ellipse` + `fill` | ✅ |
| `Path`, `Canvas`, Swift Charts | — | ⏳ |

## Layout

| SwiftUI | Romit | Status |
|---------|-------|--------|
| `VStack`, `HStack`, `ZStack` | `column`, `row`, `stack` | ✅ |
| `LazyVStack`, `LazyHStack` | `column`, `row` | ✅ |
| `LazyVGrid`, `Grid`, `GridItem(.adaptive)` | `grid 3`, `grid adaptive min 120` | ✅ |
| `ScrollView` | `scroll`, `scroll horizontal` | ✅ |
| `Spacer`, `Divider` | `spacer`, `divider` | ✅ |
| `List`, `Section`, `.swipeActions(.delete)` | `list`, `section "…" footer "…"`, `on delete` | ✅ |
| `Form`, `GroupBox`, `DisclosureGroup`, `Group` | `form`, `box`, `disclosure`, `group` | ✅ |
| `ForEach` | `for each x in xs` | ✅ |
| `if` / `else` in a view builder | `if` / `otherwise if` / `otherwise` | ✅ |
| `ViewThatFits`, `GeometryReader`, custom `Layout` | — | ⏳ |
| `Table` | `list` of `row`s | 🟡 |

## Modifiers

| SwiftUI | Romit | Status |
|---------|-------|--------|
| `.padding`, `.frame(width:height:maxWidth:)` | `padding`, `width`, `height`, `maxwidth`, `stretch` | ✅ |
| `.foregroundStyle`, `.background`, `.tint` | `color`, `background`, `gradient`, `tint` | ✅ |
| `.bold`, `.italic`, `.underline`, `.strikethrough`, `.monospaced`, `.textCase` | `bold`, `italic`, `underline`, `strike`, `mono`, `uppercase` | ✅ |
| `.fontWeight`, `.lineLimit`, `.multilineTextAlignment` | `weight`, `lines`, `center`/`leading`/`trailing` | ✅ |
| `.cornerRadius`, `.clipShape`, `.border`, `.shadow` | `corner`, `clip circle`, `border`, `shadow` | ✅ |
| `.opacity`, `.blur`, `.rotationEffect`, `.scaleEffect`, `.offset`, `.zIndex` | `opacity`, `blur`, `rotate`, `scale`, `offset`, `z` | ✅ |
| `.hidden`, `.disabled` | `hidden`, `disabled` (with `if`/`unless`) | ✅ |
| `.badge`, `.help`, `.accessibilityLabel`, `.textSelection` | `badge`, `help`, `describe`, `selectable` | ✅ |
| `.preferredColorScheme` | `theme` → `dark when x` / `appearance dark` | ✅ |
| conditional modifiers | any modifier + `if condition` | ✅ |

## Events, gestures, animation

| SwiftUI | Romit | Status |
|---------|-------|--------|
| `.onTapGesture`, `count: 2` | `on tap`, `on double tap` | ✅ |
| `.onLongPressGesture` | `on long press` | ✅ |
| `.onHover` | `on hover` | ✅ |
| `.onAppear`, `.onDisappear` | `on appear`, `on disappear` | ✅ |
| `Timer`, `TimelineView` | `every 1 second` | ✅ |
| `.sensoryFeedback` | `vibrate` | ✅ |
| `withAnimation`, `.animation`, `.transition` | `animate`, `animated`, `transition fade/slide/scale` | 🟡 CSS-based |
| `DragGesture`, `MagnifyGesture`, `matchedGeometryEffect` | — | ⏳ |
| `.keyboardShortcut`, `.onKeyPress` | Enter submits fields, Escape closes sheets | 🟡 |

## Out of scope (platform-specific)

Widgets, Live Activities, App Intents, watchOS and visionOS scenes, `UIViewRepresentable` and AppKit bridges, Apple Pencil, the Digital Crown, and the Touch Bar.
