Feature: Non-interactive curation for an operator-authorized, robots-disallowed source

  Scenario: Operator curates an allow-listed source non-interactively
    Given the curation API is available for an allow-listed source
    When the operator runs a manual ingest for section "delibere", source "https://www.example-halley-instance.test/delibere", window "2026-03"
    Then the manual ingest reports "ingested 2 document(s)"
    And the audit log contains an entry for action "ingest_manual"

  Scenario: A second curation run only processes items newer than the bookmark
    Given the curation API is available for an allow-listed source
    When the operator runs a manual ingest for section "delibere", source "https://www.example-halley-instance.test/delibere", window "2026-03"
    Then the manual ingest reports "ingested 2 document(s)"
    When the operator runs a manual ingest for section "delibere", source "https://www.example-halley-instance.test/delibere", window "2026-03"
    Then the manual ingest reports "no new items"
