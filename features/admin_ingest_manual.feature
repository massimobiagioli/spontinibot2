Feature: Scoped manual ingestion, shared by the CLI and the admin UI

  Scenario: Operator runs a scoped manual ingest of a scrapeable source
    Given the manual ingest API is available
    When the operator runs a manual ingest for section "storia", source "https://it.wikipedia.org/wiki/Maiolati_Spontini", window "30d"
    Then the manual ingest succeeds
    And the audit log contains an entry for action "ingest_manual"

  Scenario: Operator's manual ingest is rejected for a robots.txt-disallowed source
    Given the manual ingest API is available
    When the operator runs a manual ingest for section "delibere", source "https://www.halleyweb.com/c042023/zf/index.php/atti-amministrativi/delibere", window "2026-02"
    Then the manual ingest is rejected as disallowed by robots.txt

  Scenario: Operator's manual ingest is rejected for an invalid window
    Given the manual ingest API is available
    When the operator runs a manual ingest for section "storia", source "https://it.wikipedia.org/wiki/Maiolati_Spontini", window "not-a-window"
    Then the manual ingest is rejected as an invalid window

  Scenario: Operator is rejected without admin key on manual ingest
    Given the manual ingest API is available
    When the operator runs a manual ingest without admin key
    Then the request is rejected with 401
