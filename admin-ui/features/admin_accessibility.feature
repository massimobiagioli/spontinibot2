# Manual accessibility smoke test — Feature 0019 (admin-ui accessibility +
# keyboard audit). These scenarios are NOT wired to the backend cucumber
# runner (they document a screen-reader pass over the Vue SPA, not a
# citizen-facing RAG behavior) and are not executed by `cargo test -p
# backend --test bdd`. They live under admin-ui/ deliberately, separate
# from the root features/ directory swept by that runner.
#
# Methodology (2026-07-24): the implementing environment had no live macOS
# VoiceOver/GUI access, so every claim below was verified against
# Chromium's accessibility tree via `page.accessibility.snapshot()` — the
# same platform AX API VoiceOver reads from — for every route of the built
# `admin-ui` app, plus source review of focus/live-region/keyboard behavior.
# This audit found and fixed three real gaps:
#   1. Error/success callouts used a static "note" role, never announced
#      when they appeared without moving focus — see DsCallout.vue.
#   2. The cron field's syntax breakdown lived in a mouse-only `title`
#      tooltip, unreachable by keyboard or screen reader — see
#      ScheduleEditor.vue (folded into the existing accessible hint).
#   3. See Task 2.4 in Plan 0019 for the touch-target fixes.
# A live human VoiceOver session is still recommended as a final
# confirmation pass before this scenario is considered fully verified;
# until then treat this as machine-verified, not human-verified.

Feature: Screen-reader smoke test of the operator console

  Scenario: Operator finds their way with the navigation landmark
    Given the operator opens the admin-ui with VoiceOver running
    When the operator uses VoiceOver's rotor to list landmarks
    Then VoiceOver announces a "navigation" landmark named "Navigazione principale"
    And VoiceOver announces a "main" landmark containing the page content
    And the three business sections are announced as links named "Ingest", "Imprinting", "Training"

  Scenario: Operator fills in the ingest schedule form by ear alone
    Given the operator navigates to the Ingest section
    When the operator tabs into the cron expression field
    Then VoiceOver announces the field's label "Espressione cron"
    And VoiceOver announces the field's hint, including the field-by-field breakdown "Minuto Ora GiornoMese Mese GiornoSettimana", which used to live in a mouse-only "?" tooltip unreachable by keyboard or screen reader
    And VoiceOver announces the field as required
    And tabbing onward reaches a checkbox announced as "Pianificazione attiva" with its checked state

  Scenario: Operator hears a failed save announced without moving focus
    Given the operator is on the Ingest section with the schedule form focused elsewhere
    When a save request fails and an error callout appears
    Then VoiceOver interrupts and announces the error text immediately, without the operator tabbing to it
    And this relies on the callout using an "alert" role for danger-variant messages, not a silent "note" role

  Scenario: Operator hears a successful save confirmed politely
    Given the operator is on the Imprinting section after saving a new persona version
    When the "Versione salvata" success callout appears
    Then VoiceOver announces the confirmation once it finishes the current utterance, without stealing focus
    And this relies on the callout using a "status" role for success-variant messages

  Scenario: Operator selects a span of an answer and leaves feedback by keyboard
    Given the operator is on a training session with a recorded answer
    When the operator tabs to a sentence segment button and presses Space to select it
    Then VoiceOver announces the segment's pressed state changing
    And a comment field and "Feedback positivo" / "Feedback negativo" buttons become reachable by continuing to tab
    And activating "Feedback negativo" with Enter records the feedback without a mouse

  Scenario: Operator confirms a destructive action inside a native dialog
    Given the operator triggers a destructive action guarded by a confirmation dialog
    When the dialog opens
    Then focus moves into the dialog and VoiceOver announces its content
    And pressing Escape cancels the dialog and returns focus to the triggering control
    And Tab cannot leave the dialog while it is open
