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
