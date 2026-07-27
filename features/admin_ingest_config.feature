Feature: Ingest configuration management via admin API

  Scenario: Operator views an empty ingest configuration
    Given the ingest config API is available
    When the operator gets the ingest configuration
    Then the ingest configuration has no schedule
    And the ingest configuration has no sections

  Scenario: Operator sets an ingest schedule
    Given the ingest config API is available
    When the operator sets the ingest schedule to "0 */4 * * *" enabled
    Then the ingest configuration has a schedule with cron "0 */4 * * *"

  Scenario: Operator creates an ingest section
    Given the ingest config API is available
    When the operator creates an ingest section "news" with ordering 10
    Then the ingest configuration has 1 section named "news"

  Scenario: Operator creates a scrape source in a section
    Given the ingest config API is available
    And an ingest section "news" exists
    When the operator creates a scrape source "https://example.com/news" in section "news"
    Then the ingest configuration has 1 source in section "news"
    And the source in section "news" is enabled and not coming soon

  Scenario: Operator creates an API source in a section
    Given the ingest config API is available
    And an ingest section "data" exists
    When the operator creates an api source "https://api.example.com" in section "data"
    Then the ingest configuration has 1 source in section "data"
    And the source in section "data" is disabled and coming soon

  Scenario: Operator deletes a source from a section
    Given the ingest config API is available
    And an ingest section "news" exists
    And a scrape source exists in section "news"
    When the operator deletes the source from section "news"
    Then the ingest configuration has 0 sources in section "news"

  Scenario: Operator deletes a section and its sources are removed
    Given the ingest config API is available
    And an ingest section "news" exists
    And a scrape source exists in section "news"
    When the operator deletes section "news"
    Then the ingest configuration has no sections

  Scenario: Operator views the documents ingested into a section
    Given the ingest config API is available
    And an ingest section "news" exists
    And a document has been ingested into section "news" with source ref "https://example.com/news/1"
    When the operator lists the documents ingested into section "news"
    Then the ingested documents list for "news" contains "https://example.com/news/1" with 1 chunk

  Scenario: Operator looks up the ingested documents of an unknown section
    Given the ingest config API is available
    When the operator lists the documents ingested into unknown section 999999
    Then the request is rejected with 404

  Scenario: Operator sees the curation source for a section with an active curation bookmark
    Given the ingest config API is available
    And an ingest section "delibere" exists
    And a curation bookmark exists for section "delibere" at source "https://www.halleyweb.com/.../delibere"
    When the operator gets the ingest configuration
    Then the ingest configuration has 1 curation source in section "delibere"

  Scenario: Operator sees an honest empty state for a section with no source of any kind
    Given the ingest config API is available
    And an ingest section "news" exists
    When the operator gets the ingest configuration
    Then the ingest configuration has 0 sources in section "news"
    And the ingest configuration has 0 curation sources in section "news"
