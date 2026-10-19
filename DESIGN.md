---
name: Bark native examples
description: Native phone interfaces for exercising the Swift and Kotlin SDKs on RegTest.
colors:
  primary: "#006C4C"
  primary-dark: "#70DBAF"
---

# Design System: Bark native examples

## Overview

**Creative North Star: "Native wallet example"** (descriptive; no named brand direction was approved).

Mode: Operate. The SwiftUI and Compose examples use their platform's controls, type and semantic surfaces to expose real RegTest wallet state. The green action tint is the only custom palette decision.

**Key Characteristics:**

- Native navigation and controls.
- System-scaled text and selectable wallet data.
- Persistent operation feedback with dismissible error details.

This merges the incumbent native contract with the shipped source and twelve phone captures in `docs/validation/screenshots/`. The [review](docs/validation/native-ui-review.md) clears persistent iOS field labels and operation feedback only. No approved comp, FORM seed or QUALITY BAR card exists; hardware behavior and delivered screen-reader announcements remain unverified.

## Colors

### Primary

The frontmatter's deep green and light green are the light/dark action accents. Android overrides only Material 3 `primary`; iOS provides an appearance-aware tint using rounded RGB channels. That small iOS rounding drift is not a new palette token.

**The Action Tint Rule.** Use green for actions; use platform error and destructive roles for failures and exit actions.

### Neutral

iOS uses default List surfaces, primary/secondary text, separators and the `.bar` footer material. Android inherits Material 3 light/dark schemes, including surface, on-surface, on-surface-variant, outline and error. These are platform roles, not fixed project hex values. Android does not currently enable wallpaper-derived Dynamic Color.

## Typography

The system fonts and default metrics remain authoritative; no custom typeface or fixed type scale is defined.

| Platform | Observed roles |
| --- | --- |
| iOS | Native navigation title and section headers; default body for fields and balances; headline/caption for activity; subheadline for address labels and pinned feedback; monospaced caption for addresses and invoices. |
| Android | Material 3 headlineMedium for available balance; titleLarge for sections; titleMedium for network/activity; default body for fields and supporting text; bodyMedium for pinned feedback; labelLarge for address labels; monospaced bodySmall for wallet values. |

**The Native Type Rule.** Keep semantic text styles and system scaling; let long labels and values wrap.

## Layout

Phones use one scrolling wallet surface. Connection/network state appears first; opening a wallet reveals grouped tasks, balances and receive data. iOS uses `NavigationStack`, List sections and native safe areas. Android uses `Scaffold`, `TopAppBar` and `LazyColumn` with scaffold, navigation-bar and IME insets.

Android content padding is (20dp), vertical item spacing is (12dp), and address stacks use (6dp). Its pinned feedback row uses (20dp) horizontal/(8dp) vertical padding and (12dp) gaps. iOS retains system List spacing, default footer padding and (6pt) address-stack spacing. Do not turn platform defaults into fixed shared measurements. The contract requires minimum touch targets of (44pt) on iOS and (48dp) on Android; these remain platform control responsibilities rather than custom hit-box tokens. No tablet layout or custom breakpoint is established.

## Elevation & Depth

iOS separates grouped rows through native surfaces, separators and bar material. Android uses standard Material surfaces and dividers; the pinned feedback Surface has tonal elevation (3dp). Sheets and dialogs retain platform presentation. No custom shadows or motion tokens exist.

## Shapes

iOS uses native grouped sections, rows and sheets. Android uses default Material 3 filled/outlined/text buttons, outlined fields and AlertDialog shapes. No custom radius scale is defined.

## Components

- **Actions:** iOS uses tinted List buttons and native ShareLink/SF Symbols. Android uses a filled button to open the wallet, outlined task buttons and a text button for Details. Busy operations disable wallet controls. Starting an exit requires a native confirmation dialog.
- **Fields:** iOS `LabeledContent` keeps Ark server, Bitcoin RPC, send amount and board amount visible when populated; recovery uses SecureField. Android uses persistent OutlinedTextField labels and masked recovery input. Amount fields request numeric keyboards. Preserve the native focus and disabled treatments.
- **Wallet data:** addresses/invoices use wrapping, selectable monospaced text; activity shows real SDK data or the explicit empty message. iOS additionally offers native sharing.
- **Operation feedback:** a pinned native footer remains outside scrolling content. It shows status, a busy indicator with “Working…”, or “Operation failed” with Details. The full selectable SDK error opens in an iOS sheet or Android scrollable AlertDialog; Done returns to the editable wallet. iOS posts accessibility announcements for busy/status/error changes; Android marks feedback as a polite live region. Announcement delivery and transient progress were checked in source, not demonstrated by the captures.

## Do's and Don'ts

### Do:

- **Do** preserve native controls, safe areas, text scaling and minimum touch targets.
- **Do** keep populated field labels, operation status and error details available while tasks scroll.
- **Do** show actual RegTest balances, addresses, outcomes and empty states.

### Don't:

- **Don't** replace native navigation with custom navigation or decorative card stacks.
- **Don't** add decorative charts, synthetic activity or an invented brand type scale.
- **Don't** treat simulator captures or source inspection as hardware or screen-reader verification.
