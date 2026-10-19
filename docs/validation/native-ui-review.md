## verdict

1. **Resolved — operation feedback.** The recaptured Send and maintenance views keep “Operation failed” and Details visible outside the scrolling content on both platforms. The new error-detail captures expose the full SDK validation failure in native dismissible presentations; dismissal returns to the editable task for correction and resubmission. Source confirms the same pinned region handles busy progress and completion status, with UIKit announcements on iOS and a polite Compose live region on Android. Announcement delivery and transient busy rendering were checked in source, not observed with assistive technology or captured mid-operation.
2. **Resolved — persistent iOS input labels.** Onboarding retains Ark server and Bitcoin RPC beside populated URLs. The Send error capture retains Amount in sats beside its value. The large-text maintenance capture retains Board amount in sats above its value without clipping. Source uses native LabeledContent for all four fields.

All eight original screenshot paths were reread after replacement, together with the four new Send/error-detail captures. No material regression from the fix batch is visible in the reviewed evidence. This ship disposition covers the two scored fixes, not the whole surface.

## remaining

Clear for the two scored fixes. Original evidence limits remain: simulator/emulator captures do not establish hardware behavior, gesture quality, or actual screen-reader announcement delivery; no comp, five-block direction/seed record, or QUALITY BAR card was supplied.

disposition: ship

## Capture provenance

Actual screenshots from the built Swift6/Compose example apps on iPhone18Pro
(iOS27.0 simulator,402pt) and Pixel7 (Android15 ARM64 emulator,1080px), using the
T3 agent-device launcher. Captured2026-10-18 America/New_York. Dark-large uses
iOS accessibility-large and Android font_scale1.3. Send-error/details captures
show actual SDK invalid-address failures. Captures are persisted in screenshots/
next to this report. This verifies UI changes; current-master runtime acceptance
is recorded separately after rebuilding the SDKs.
