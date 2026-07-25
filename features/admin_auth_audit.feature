Feature: Operator authentication and audit log

  Scenario: An unauthenticated write is rejected
    Given the backend service is running
    When the operator creates a persona version without a session
    Then the request is rejected with 401

  Scenario: Operator logs in with the correct password
    When the operator logs in with the correct password
    Then the login succeeds and a session cookie is set

  Scenario: Operator login is rejected with an incorrect password
    When the operator logs in with an incorrect password
    Then the login is rejected with 401

  Scenario: An authenticated write is recorded in the audit log
    Given the knowledge base contains persona "gaspare" with version 1 active
    When the operator creates a new version of persona "gaspare" with activation
    Then the audit log contains an entry for action "create_persona"

  Scenario: Logging out invalidates the session
    When the operator logs out
    And the operator requests persona versions again with the same, now-stale cookie
    Then the request is rejected with 401
