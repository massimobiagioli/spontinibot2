Feature: Triggering an immediate ingest run via admin API

  Scenario: Operator triggers an immediate ingest run
    Given the ingest run API is available
    When the operator triggers an ingest run
    Then a new ingest run is queued with pending status

  Scenario: Operator polls a triggered run through to completion
    Given the ingest run API is available
    When the operator triggers an ingest run
    Then a new ingest run is queued with pending status
    When the ingest service picks up and completes that run
    And the operator checks the status of that ingest run
    Then the ingest run status is "done"

  Scenario: Operator checks the status of an unknown ingest run
    Given the ingest run API is available
    When the operator checks the status of ingest run 999999
    Then the ingest run is not found

  Scenario: Operator is rejected without admin key on trigger
    Given the ingest run API is available
    When the operator triggers an ingest run without admin key
    Then the request is rejected with 401

  Scenario: Operator is rejected without admin key on status check
    Given the ingest run API is available
    When the operator checks the status of ingest run 1 without admin key
    Then the request is rejected with 401
