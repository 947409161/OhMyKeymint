---
version: alpha
name: Oh My Keymint WebUI
description: A strictly monochrome Android module WebUI. Hierarchy is carried by type, weight, hairlines, spacing and a grayscale surface ramp — never by hue.
colors:
  light:
    background: "#ffffff"
    surface: "#ffffff"
    container: "#f5f5f5"
    container-high: "#ebebeb"
    on-surface: "#000000"
    on-surface-muted: "#5c5c5c"
    on-surface-disabled: "#8a8a8a"
    divider: "#e0e0e0"
    accent: "#000000"
    on-accent: "#ffffff"
  dark:
    background: "#121212"
    surface: "#1a1a1a"
    container: "#242424"
    container-high: "#2e2e2e"
    on-surface: "#ffffff"
    on-surface-muted: "#a3a3a3"
    on-surface-disabled: "#6b6b6b"
    divider: "#333333"
    accent: "#ffffff"
    on-accent: "#000000"
  amoled:
    background: "#000000"
    surface: "#000000"
    container: "#0a0a0a"
    container-high: "#161616"
    on-surface: "#ffffff"
    on-surface-muted: "#9e9e9e"
    on-surface-disabled: "#5e5e5e"
    divider: "#262626"
    accent: "#ffffff"
    on-accent: "#000000"
---

# Oh My Keymint WebUI — Design Standard

## Overview

The WebUI is **strictly monochrome**. Every surface, divider and text tone resolves to a
grayscale value. There is no accent hue, no semantic red/green/amber, and no dynamic colour
extraction.

This is an intentional constraint, not an unfinished palette. It follows from the aim of the
rewrite: the previous build carried a second, hand-maintained copy of its component library's
colour system, and the two fought in the cascade. Removing colour entirely removes the whole
class of problem.

Because hue is unavailable, hierarchy must come from exactly five levers. Every component
specification below is expressed through them and nothing else:

| Lever | Tokens |
|---|---|
| Type size | 7 roles, see [Typography](#typography) |
| Weight | 400 regular / 500 medium / 600 semibold |
| Hairline | `--omk-hairline`, `--omk-hairline-strong` |
| Surface level | `background` → `surface` → `container` → `container-high` |
| Spacing | 4 px grid, see [Spacing](#spacing) |

### Machine enforcement

`tests/unit/monochrome.test.mjs` scans the **built stylesheet** — not the sources — and fails
the build when:

1. any hex literal is not achromatic;
2. any numeric `rgb()`/`rgba()` value is not achromatic;
3. any chromatic colour keyword reaches a rendered declaration (feature-detection probes inside
   `@supports` preludes are excluded, since they never render);
4. any of the three themes is missing a token from the required set;
5. body or muted text falls below the 4.5:1 WCAG AA floor on **any** surface it can land on —
   `background`, `container` and `container-high`, not just the page canvas;
6. a theme collapses `disabled` and `muted` onto the same value, which would allow the two to be
   used interchangeably.

Scanning build output rather than source means the guard also catches colour arriving through a
Tailwind utility or a third-party component. Check 5 exists because an earlier background-only
version passed a stylesheet whose muted-on-container text measured 3.16:1.

Runtime contrast is checked separately by `tests/site/a11y.spec.ts`, which runs axe over the
rendered gallery in all three themes. The static guard and axe catch different classes of
mistake: the guard reasons about token pairs, axe about what actually composed on screen
(including `opacity`, which the guard cannot see).

## Colors

### Surfaces

Four levels, applied fill-first. Elevation is expressed by filling a container with a lighter
(or, in light theme, slightly darker) gray — **not** by shadow.

| Level | Role |
|---|---|
| `background` | Page canvas. Sits behind everything. |
| `surface` | Persistent bars: top app bar, navigation bar, bottom sheets, dialogs. In light theme this equals `background`; the bar is separated by a hairline instead. |
| `container` | Cards, grouped list bodies, section bodies. This is the default "raised" treatment. |
| `container-high` | The top step: pressed/selected rows, input fields, nested panels, drag handles. |

### Text

| Token | Role |
|---|---|
| `on-surface` | Primary text and icons. |
| `on-surface-muted` | Summaries, captions, unselected navigation labels, placeholder text, and status values that are merely *absent* rather than *inactive*. Clears AA on every surface. |
| `on-surface-disabled` | Genuinely inactive controls only. Exempt from the contrast floor by WCAG, and the only token allowed below it. |
| `on-accent` | Text and icons drawn on an `accent` fill. |

`on-surface-disabled` is the one token WCAG lets below 4.5:1, and the exemption is narrow: it
covers *inactive user interface components*. Two consequences follow.

- **Never express disabled with `opacity`.** Opacity composites to a value neither the guard nor
  a reviewer can predict, and it also drops the text out of the exempt category as far as tooling
  is concerned, because there is nothing marking the element as inactive. Use the token, and mark
  the element `disabled` or `aria-disabled`.
- **An unavailable status value is not an inactive control.** "Not installed" is text the user must
  read, so it uses `on-surface-muted`. An earlier revision put it on `on-surface-disabled` and
  measured 3.16:1.

### Accent

`accent` is **pure black in light theme and pure white in dark/amoled**. It marks the single
most important action in view, selected states, and active controls. It is an inversion, not a
hue: a primary button is a solid black rectangle in light theme and a solid white one in dark
theme.

At most **one** accent-filled element should be visible per screen region. Two solid inverted
blocks in the same card is a specification violation.

### Boundaries

| Token | Value | Role |
|---|---|---|
| `divider` | solid, 1 px | List separators, card outlines, bar edges, inactive track fills. |
| `hairline` | 1 px `divider` | Default border. |
| `hairline-strong` | 1 px `on-surface` at 14% | Focus rings, selected outlines, error emphasis. |

### Contrast invariants

Verified at build time for every theme:

| Pair | Floor |
|---|---|
| `on-surface` on `background` / `container` / `container-high` | 4.5:1 |
| `on-surface-muted` on `background` / `container` / `container-high` | 4.5:1 |
| `on-accent` on `accent` | 4.5:1 (implied by the inversion) |
| `on-surface-disabled` | exempt; must differ from `on-surface-muted` |
| `divider` vs any surface | not asserted — decorative boundary, not text |

Both the token-pair table above and the rendered result are asserted, by the guard and by axe
respectively.

## Typography

One family (the platform UI font). No web fonts are loaded: the WebView must render offline and
the CSP forbids remote origins.

| Token | Size | Weight | Line height | Role |
|---|---|---|---|---|
| `text-display` | 28 px | 600 | 36 px | Single large figure on Home (target count). |
| `text-title` | 20 px | 600 | 28 px | Page title, top app bar title, dialog title. |
| `text-body` | 16 px | 400 | 24 px | Body copy, list primary text. |
| `text-body-strong` | 16 px | 500 | 24 px | Setting row titles, status values. |
| `text-label` | 14 px | 500 | 20 px | Button labels, field labels, section headers. |
| `text-caption` | 13 px | 400 | 18 px | Setting summaries, helper text, badges. |
| `text-mono` | 13 px | 400 | 18 px | Package names, fingerprints, dates. Monospace, tabular figures. |

**Rules**

- A component uses at most two type roles. A card with three sizes is a violation.
- Weight, not size, carries emphasis inside a row. `text-body` → `text-body-strong` is the
  preferred promotion; jumping to `text-title` inside a list is not allowed.
- `text-mono` is mandatory for anything a user may copy verbatim (package names, dates).

`text-mono` is declared as a utility rather than a `--text-*` theme token, because Tailwind's
type sub-tokens cover line-height, letter-spacing and weight but not font-family. A monospace
role defined without a monospace face silently renders in the UI font — which is exactly what
happened before the gallery was rendered and reviewed.

## Layout

### Spacing

4 px grid. Only these values are permitted: **4, 8, 12, 16, 20, 24, 32**.

| Context | Value |
|---|---|
| Inside a control (icon to label) | 8 |
| Card padding | 16 |
| Between rows in a group | 0 (separated by hairline) |
| Between groups | 16 |
| Page horizontal inset | 16 |
| Section top/bottom | 24 |

### Insets

KernelSU injects the safe-area variables through `/internal/insets.css`. Only the
`--window-inset-*` family exists — there is no `--top-inset` family, despite what an earlier
revision of this project assumed.

```
--omk-top-inset     : var(--window-inset-top, env(safe-area-inset-top, 0px))
--omk-bottom-inset  : var(--window-inset-bottom, env(safe-area-inset-bottom, 0px))
--omk-left-inset    : var(--window-inset-left, env(safe-area-inset-left, 0px))
--omk-right-inset   : var(--window-inset-right, env(safe-area-inset-right, 0px))
```

Bars add their inset to their own height; scrollable content pads by the inset only where it is
not already covered by a persistent bar.

### Radius

| Token | Value | Applied to |
|---|---|---|
| `radius-sm` | 6 px | Badges, checkboxes, small swatches |
| `radius-md` | 10 px | Buttons, inputs, snackbars |
| `radius-lg` | 14 px | Cards, dialogs, sheet tops |
| `radius-full` | 9999 px | Switches, pills, drag handles |

Radii are consistent per class. A card is never `radius-sm`; a badge is never `radius-lg`.

## Elevation & Depth

There are **no shadows**. On a `#000000` or `#121212` canvas a black shadow is invisible, and
on `#ffffff` it degrades into a grey blur that reads as grime rather than depth.

Depth is expressed by exactly three means:

1. **Surface step** — moving a block from `background` to `container` to `container-high`.
2. **Boundary** — a `hairline` where two adjacent surfaces are the same level.
3. **Scrim** — modal overlays only: `rgb(0 0 0 / 32%)` in light, `rgb(0 0 0 / 60%)` in
   dark and amoled. In amoled the scrim is effectively invisible against the pure-black canvas;
   the modal is separated by its `container-high` fill instead. This is accepted.

## Shapes

The system is **geometric, not organic**: small consistent radii, flat fills, straight hairlines.
No shape morphing, no capsule-shaped cards, no asymmetric corners.

## Components

Layering follows the repository convention: `atoms` → `molecules` → `views`. Imports only
point upward. A molecule may compose atoms; an atom never imports a molecule or a view.

### Tier A — required before any view can be rebuilt

**Atoms**

| Component | Surface | Boundary | Notes |
|---|---|---|---|
| `Button` | variant-dependent | variant-dependent | primary / secondary / ghost |
| `IconButton` | transparent | transparent | Square hit area, `aria-label` mandatory |
| `Card` | `container` | none | `pressable` adds a pressed fill step |
| `Divider` | — | `hairline` | Full-bleed or 16 px inset |
| `Icon` | — | — | Material Symbols, 16/20/24 px, inherits `currentColor` |
| `ProgressIndicator` | — | — | Circular (stroke ring) and linear (2 px track) |
| `Badge` | `accent` or transparent | optional `hairline` | 20 px tall, `text-caption` |
| `Switch` | track `container-high` → `accent` | `hairline` when off | Headless UI |
| `Checkbox` | `accent` when checked | `hairline` | Headless UI |
| `Slider` | track `container-high`, fill `accent` | — | Radix |

**Molecules**

| Component | Composes | Notes |
|---|---|---|
| `SettingRow` | `Icon`, `Divider`, trailing control | The workhorse. Six variants: switch, checkbox, navigate, value, slider, disabled |
| `SectionHeader` | — | `text-label`, `on-surface-muted`, 16 px inset |
| `TopAppBar` | `IconButton`, `Icon` | 56 px + `--omk-top-inset`, `surface` fill, bottom hairline |
| `NavigationBar` | `Icon` | 64 px + `--omk-bottom-inset`, 3 items, top hairline |
| `Dialog` | `Button` | Headless UI, max-width 360 px, `radius-lg`, `scrim` overlay |
| `SnackbarHost` | — | Inverse surface, 48 px min height, `radius-md`, queued, `aria-live` |
| `StatusField` | `Icon` | Label + value + tone; values wrap rather than truncate |
| `AppearanceModeRow` | `SettingRow`, `Dialog` | The night-mode control. Copy-free: all strings arrive as props |

`Dialog` carries the scrim and the panel, and its `role="dialog"` root spans the viewport. A root
sized only by its `position: relative` children collapses to zero height, which assistive
technology and automated checks both read as hidden.

### Tier B — needed by the view rewrite, not blocking the first slice

`SearchBar`, `AppListItem`, `TabRow`, `EmptyState`.

### Tier C

`VisuallyHidden` — likely a utility class rather than a component.

### Component specifications

**Button**

| Variant | Fill | Label | Boundary | Height | Padding |
|---|---|---|---|---|---|
| primary | `accent` | `on-accent` | none | 40 px | 0 20 px |
| secondary | transparent | `on-surface` | `hairline` | 40 px | 0 20 px |
| ghost | transparent | `on-surface` | none | 40 px | 0 12 px |

States: hover fills `container` (secondary/ghost only); pressed fills `container-high`;
`focus-visible` draws `hairline-strong` with 2 px offset; disabled uses `on-surface-disabled`
for the label and `divider` for the fill or boundary. Loading swaps the label for a circular
indicator at the leading edge and sets `aria-busy`.

**SettingRow**

```
[icon]  Title                          [trailing control]
        Summary
────────────────────────────────────────────────────
```

Minimum height 56 px, or 72 px when a summary is present. Horizontal inset 16 px. Title
`text-body-strong`; summary `text-caption` in `on-surface-muted`. The trailing control is
vertically centred. The separator hairline is inset 16 px on the start edge and full-bleed on the
end edge. A row is a `<button>`, `<label>` or `<div>` according to whether it navigates,
toggles or is inert — never a `<div>` with a click handler.

**StatusField** — the semantic-tone problem

Colour previously carried success/error/pending. With no hue available, tone is expressed by
glyph and weight:

| Tone | Treatment |
|---|---|
| normal | no prefix; value in `text-body-strong` |
| error | leading `!` icon; value in `text-body-strong`; field boundary `hairline-strong` |
| pending | leading circular indicator; value in `on-surface-muted` |
| unavailable | muted value text; the string itself states the absence ("Not installed") |

The tone must also be exposed as text, not only as a glyph: the value string itself states what
is wrong ("Invalid Keybox", "Not installed"). A screen reader user receives the same information
as a sighted one.

**Sizing a slider**

`Slider` always fills its container, and its track uses `divider` rather than a `container` step —
a container step is invisible against the `surface` row a slider normally sits on.

Size the **wrapper**, never the slider. Passing a width through the component's `className` puts
two width utilities in one class list, and CSS resolves that by stylesheet order rather than by
the order the classes appear, so the result is not something the caller can reason about. This
is not theoretical: it collapsed the control to the thumb's 16 px and left no visible track.

**Naming a control inside a row**

A row that owns a control renders as a `<label for>` so the whole row is a hit area. That label
does **not** name the control: Headless UI renders a `role="switch"`/`role="checkbox"` button,
and Chromium's accessible-name computation for a button does not consume an associated label.
Headless UI also drops `aria-labelledby` on those components.

The rule that follows: the control carries `aria-label` copied from the row title. Do not reach
for `aria-labelledby` here, and do not assume the surrounding label did the job — verify the
computed name, because the failure mode is silent and axe reports it only as
`aria-input-field-name`.

**SnackbarHost**

Inverse surface — `on-surface` fill with `background` text. A solid black bar on white in
light theme, a solid white bar on black in dark. This is the strongest available emphasis and is
reserved for transient feedback. Minimum height 48 px, `radius-md`, offset from the bottom by
`--omk-bottom-inset` + 16 px. Error snackbars add a leading `!` icon and a
`hairline-strong` boundary; they do not change hue.

## Do's and Don'ts

**Do**

- Reach for weight and spacing before reaching for a new size.
- Use `container-high` for a pressed state. It is the only permitted "darker on press" step.
- Keep exactly one accent-filled element per screen region.
- Give every icon-only control an accessible name.
- Add the component's keyboard and focus assertions in the same change that introduces it.

**Don't**

- Introduce any chromatic value. The build fails, and the failure is intentional.
- Use a shadow for elevation.
- Use colour alone to signal state.
- Use `opacity` to express a disabled or unavailable state.
- Put `on-surface-disabled` on text the user is expected to read.
- Add a token that is not on the grayscale ramp — including "just for the disabled state".
- Add a type size outside the seven roles.

## Resolved decisions

Each of these was open at review and is now fixed. The reasoning is recorded so a later change
does not silently undo it.

**Radius — soft (6/10/14 px).** Chosen over a tighter 4/6/8 px set. The system is already
geometric through its flat fills and hairlines; sharper corners on top of that read as severe
rather than modern, and the softer set stays closer to the platform surfaces the WebUI sits
inside. Only the soft set is defined — there is no runtime radius switch, because a second
character with no consumer is dead configuration.

**Navigation bar selected state — `container-high` pill plus medium label weight.** With no hue
available, the surface step is the strongest signal that does not compete with the label. An
indicator bar was rejected as a second, redundant signal; a small bottom stub in particular
looked fussy at 64 px row height.

**Snackbar — inverse surface.** `on-surface` fill with `background` text: a solid black bar in
light theme, a solid white bar in dark. It is the strongest emphasis the ramp can produce and is
reserved for transient feedback, so it does not compete with the `container` fill that cards use.
The quieter alternative (`container-high` plus a hairline) was rejected because it made a
transient message look like another card.

**Card — fill-based, no boundary.** A `container` fill on the page canvas, with `container-high`
as the pressed step. This keeps elevation entirely on the surface ladder, leaves the hairline
free to mean "separator" rather than "edge", and avoids drawing a border between two adjacent
levels that are already distinct.

**Night mode — an explicit four-way choice, not a single toggle.** The appearance row offers
Automatic, Light, Night and Night (pure black). A boolean would have to discard either the
follow-the-system behaviour or the pure-black theme, and both are worth keeping. The picker is a
dialog rather than a `<select>`: the WebUI is touch-only, and a native dropdown inside a WebView
renders as a desktop popup.

## Runtime environment

The WebUI normally runs inside a KernelSU WebView, where `kernelsu-alt` exposes the bridge the
native helpers are reached through. Two things follow, and both are enforced by
`tests/unit/bundle.test.mjs`.

**A release build never fabricates device state.** `src/bridge/dev.ts` answers the `--webui-*`
protocol so development and Playwright can exercise the production code path without a device. It
is installed only under `import.meta.env.DEV`, so Vite removes it from release builds. The
failure mode this prevents is the worst one available: a device with a broken or missing bridge
showing a healthy-looking keybox and package list that no native call produced, leaving the user
convinced an operation succeeded.

**A release build without a bridge says so.** Rather than rendering the ordinary shell — whose
every field would read "unavailable" for no stated reason — it shows the blocking page from
`renderBridgeUnavailablePage()`.

## Copy and translation

No component hard-codes user-visible copy. Strings arrive from `i18n`, and the components that
need them together (a picker's options, a row's title and summary) take them as a `labels` prop,
so the owning view supplies the translation rather than the component inventing one.

Three rules, all enforced by `tests/unit/i18n_keys.test.mjs`:

- **Every key used by the sources exists in `en.xml`.** A missing key does not crash — it renders
  the English fallback, which in a translated UI is a silent regression.
- **`en.xml` and `zh-CN.xml` define the same key set.** They are the reference locales; the other
  twenty-one are partial by design and fall back to English.
- **Placeholders match across locales.** A locale that drops or renames `%s` renders the
  placeholder verbatim.

The last rule exists because of a real defect: `home_selected_apps` is `"%s apps selected"`, and
it was used as a plain label, so the placeholder reached the screen. A parameterised string is
never a label — it is the whole sentence, with the value substituted.

The English fallback passed to `tr()` is defence in depth, not the source of truth. If it drifts
from `en.xml` the screen shows the `en.xml` text, so tests must assert the shipped copy rather
than the fallback.

## Validation

```sh
pnpm --dir webui exec biome ci ./src   # formatting and lint, no writes
pnpm --dir webui run type-check        # tsc --noEmit, strict
pnpm --dir webui run build             # emits template/webroot
pnpm --dir webui test                  # node --test, incl. the monochrome guard
pnpm --dir webui exec playwright test  # behaviour plus axe in all three themes
```
