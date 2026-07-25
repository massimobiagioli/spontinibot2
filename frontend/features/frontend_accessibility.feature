# Manual accessibility smoke test — Feature 0023 (frontend accessibility +
# reduced-motion + keyboard audit). These scenarios are NOT wired to the
# backend cucumber runner (they document a screen-reader pass over the
# citizen-facing chat widget, not a RAG behavior) and are not executed by
# `cargo test -p backend --test bdd`. They live under frontend/ deliberately,
# separate from the root features/ directory swept by that runner, mirroring
# admin-ui/features/admin_accessibility.feature (feature 0019).
#
# Methodology (2026-07-25): the implementing environment had no live macOS
# VoiceOver access and no connected browser automation (the Claude-in-Chrome
# extension was not available in this session), so every claim below was
# verified by source-level review of the rendered markup, ARIA roles, live
# regions, and focus management — not against a live accessibility tree or an
# actual screen reader. This audit found and fixed one real gap:
#   1. The honest-unknown fallback callout (`ChatMessage.vue`, `DsCallout
#      variant="primary"`) rendered with `role="note"`, a static role that
#      `DsCallout.vue` itself documents as never auto-announced — a citizen
#      who received no answer would never hear that fact. `DsCallout` gained
#      an explicit `role` override prop, and `ChatMessage.vue` now requests
#      `role="status"` for this callout while keeping its `primary` styling.
# A live human VoiceOver session (or a connected browser automation tool
# reading the real accessibility tree) is still recommended as a final
# confirmation pass before this scenario is considered fully verified; until
# then treat this as source-verified, not human-verified.

Feature: Screen-reader smoke test of the public chat widget

  Scenario: Citizen hears the pending state announced without moving focus
    Given a citizen opens the chat widget with a screen reader running
    When the citizen submits a question
    Then the screen reader announces "Sto rispondendo…" once it appears
    And this relies on the pending message using a "status" role, so it is announced politely without stealing focus from the input

  Scenario: Citizen hears an answered response and can reach its citations by keyboard alone
    Given a citizen has asked a question answerable from a municipal document
    When the answer with its citation disclosure appears
    Then the screen reader announces the answer text
    And tabbing onward reaches a "Fonti (N)" disclosure control implemented as a native <summary> element
    And pressing Enter or Space on it expands the citation list, announced as its expanded state changing
    And each citation is reachable and read as a list item

  Scenario: Citizen hears the honest-unknown fallback announced without moving focus
    Given the knowledge base contains no document matching the citizen's question
    When Spontini answers that no information was found
    Then the screen reader interrupts politely and announces the fallback message, without the citizen tabbing to it
    And this relies on the fallback callout using a "status" role, not the static "note" role its "primary" styling would otherwise default to

  Scenario: Citizen hears the error state announced immediately
    Given the backend chat endpoint fails with a 502 or 503 response
    When the honest error state appears
    Then the screen reader interrupts and announces "Non riesco a rispondere ora. Riprova tra qualche minuto." immediately
    And this relies on the error callout using an "alert" role, consistent with the danger variant's default
    And the screen reader never announces the raw backend error text

  Scenario: Citizen reaches the component catalog link and every catalog example by keyboard
    Given a citizen tabs through the chat page from the top of the document
    When the citizen reaches the "Catalogo componenti" link
    Then it is announced as a link with a visible focus ring
    And activating it with Enter navigates to the /dev catalog
    And every control in the catalog (buttons, inputs, callouts, citation disclosures) remains reachable and operable by Tab and Enter/Space alone
