---
name: spontini-ui-craft
description: Design, build, or modify any admin-ui or frontend UI — components, views, dialogs, layout, spacing, button choices. Use WHEN touching any .vue file, any admin-ui/src or frontend/src template/style, or adding/changing a `ds/` primitive. Enforces reuse of `ds/` components, Bootstrap Italia tokens only, native `<dialog>`/`<details>` conventions, spacing between adjacent interactive elements, and consistent variant pairing — the concrete failure modes this project has repeatedly shipped.
---

# Spontini UI Craft

This project has repeatedly shipped UI that is functionally correct but visually cramped, inconsistent, or half-styled: buttons touching a textarea with no gap, a "confirm" action styled filled while its paired "cancel" is outline (or vice versa, arbitrarily), a `<dialog>` with `border: none` and no shadow that blends into the page. None of these are complex fixes — they are attention failures. This skill is a checklist to stop making them, not a new abstraction to learn.

Load this skill **before** writing a single line of template or style for admin-ui/frontend work, and re-check the "Before you ship" list **after**.

## 1. Reuse before you invent

- **Grep `ds/` first.** `admin-ui/src/components/ds/` (`DsButton`, `DsInput`, `DsCallout`, `DsConfirmDialog`, `DsNav`) is the only component vocabulary this app has. If what you need is close to an existing `ds/` component, extend it (a new prop, a new slot) — do not hand-roll raw `<button class="btn ...">` markup next to it.
- **If no `ds/` primitive fits, build one**, following the existing ones' shape: a single-purpose `.vue` file under `ds/`, typed props with `withDefaults`, scoped or token-driven styles, exported from `ds/index.ts`, and **added to `admin-ui/src/views/DevCatalog.vue`** so it has a live rendered example next to every other primitive. A `ds/` component nobody can see in the catalog is a component nobody will reuse correctly next time.
- **Grep for the same pattern elsewhere before styling it from scratch.** Confirm/cancel pairs, empty states, error callouts, badge usage — this codebase already has 5+ examples of each. Match the existing one; don't invent a sixth variant.

## 2. Tokens only, no bespoke hex/px

- Every color, spacing, and radius value comes from Bootstrap Italia's `--it-...`/`--bs-...` custom properties or this project's own `--spontini-...` tokens (`admin-ui/src/styles/_tokens.scss`). Never write a literal hex color or pixel value in a new rule — grep the existing `<style scoped>` blocks in `ds/` or `views/` for the token name you need before inventing a number.
- This is not a style preference — Bootstrap Italia's tokens already encode contrast-safe pairings and dark/light handling. A hardcoded value silently drifts from them the next time the design tokens change.

## 3. Dialogs: native `<dialog>`, and make it visibly a dialog

- Modals in this app are native `<dialog>` elements driven imperatively (`showModal()`/`close()`), never a hand-rolled `position: fixed` overlay div. Follow `DsConfirmDialog.vue`'s pattern exactly (including its comment explaining *why* `open` is never template-bound).
- **A bare `border: none; border-radius: 8px;` on a `<dialog>` is not enough** — without a `box-shadow` (or a visible border in a different color from the page background), the panel has no visual separation from `::backdrop` on some renderers/themes and reads as "broken", not "borderless by design". Give every dialog's content box a real elevation shadow using a `--it-elevation-*`/`--bs-*` shadow token (or the concrete `box-shadow` already used by Bootstrap Italia's own `.modal-content` — check `node_modules/bootstrap-italia/src/scss/components/_modal.scss` for the token names) so it visibly floats above the backdrop, in both light and dark.
- Footer actions live in a bordered footer strip (`border-top`), matching `DocumentDetail.vue`/`QuestionDetail.vue` — don't let action buttons float loose in the body.

## 4. Accordions: native `<details>`/`<summary>`, not a hand-rolled toggle

- This codebase already has a working accordion pattern with zero JS: `QuestionDetail.vue`'s `.question-detail__accordion` (`<details>` + `<summary>` + a rotating `::before` arrow, `list-style: none` to kill the default marker). Reuse that exact pattern for any new collapsible section instead of writing `v-if`/click-handler toggle state.
- When multiple accordion items on one page should behave as "opening one closes the others" (a classic accordion, not just independent collapsibles), give each `<details>` the same `name="..."` attribute — modern browsers make them mutually exclusive natively, no JS required.
- If you build a reusable `DsAccordion.vue` wrapper for this, add it to `DevCatalog.vue` per §1.

## 5. Spacing between adjacent interactive elements

- **Never let a button (or button group) sit directly against the element above it with zero gap** — this is the single most common defect reported against this app's UI (a "Conferma"/"Annulla" pair glued to the textarea it follows). Any block that stacks a text input/textarea and then action buttons needs an explicit `margin-top` (`var(--it-spacing-s)`/`1rem`-scale token, not an eyeballed number) on the actions row, or a `gap` on the flex/grid container wrapping both.
- Apply the same rule to callouts (error/success messages) that appear conditionally above or below controls — they need breathing room on both sides, not to be flush against a neighboring element that happened to be adjacent in the template.

## 6. Same semantic weight, same visual treatment

- If two actions are peers in the same decision (confirm vs. cancel, positive vs. negative feedback, save vs. discard), they must use the **same fill-vs-outline scale** — both outline, or the primary one filled and the secondary one outline, applied *consistently* across every place that pairing appears in the app. Don't let one instance of "positive/negative feedback" render positive as filled `btn-success` and negative as `btn-outline-danger` — that reads as "negative is a lesser, second-class action" when both are equally valid operator input.
- When auditing an existing pair for this, check every `DsButton` invocation in the component, not just the ones you're actively editing — a fix that corrects one button and leaves its pair inconsistent is not a fix.

## 7. State honesty, and one card = one truth

- Loading, empty, and error states are always explicit (this project's existing convention — see `DsCallout` usage throughout `ingest/` and `training/` components). Never leave a silently blank area where a state message belongs.
- If a piece of UI represents a single fact that can only have one current value (e.g., "this message's feedback"), the UI must show **one** current value, not a growing history of every value it ever had, and must not re-offer the input controls once that fact is set — re-offering them invites the user to wonder whether they're allowed to change their mind, which the data model may not even support. Match the UI's affordances to what the underlying action actually allows (one-shot vs. mutable vs. append-only).

## Before you ship — recheck this list

1. Does every new/changed interactive element have visible breathing room from its neighbors (§5)?
2. Do paired actions match in visual weight (§6)?
3. Did you reuse a `ds/` component or an existing native-element pattern (`<dialog>`, `<details>`) instead of inventing new markup (§1, §3, §4)?
4. Is every color/spacing value a token, not a literal (§2)?
5. If you added a `ds/` primitive, is it in `DevCatalog.vue`?
6. Run the component/view's Vitest suite (`npm run test -- --run <Name>`) — green tests don't prove good UI, but they prove you didn't break the existing contract while fixing the visual one.
7. Where feasible, actually look at the rendered page (via the `run` skill or `claude-in-chrome`) — a diff that "should" look right is not the same as having looked at it.
