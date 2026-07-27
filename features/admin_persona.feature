Feature: Bot imprinting management via admin API

  Scenario: Operator creates a new persona version that becomes active
    Given the knowledge base contains persona "gaspare" with version 1 active
    When the operator creates a new version of persona "gaspare" with activation
    Then the new persona version is active
    And the previous persona version is inactive

  Scenario: Operator lists all versions of a persona
    Given the knowledge base contains persona "gaspare" with 2 versions
    When the operator requests all versions of persona "gaspare"
    Then 2 versions are returned
    And the latest version is listed first

  Scenario: Operator activates a specific older version of a persona
    Given persona "gaspare" has version 1 active and version 2 inactive
    When the operator activates version 2 of persona "gaspare"
    Then version 2 becomes active
    And version 1 becomes inactive

  Scenario: Operator reloads the persona cache
    Given the persona cache contains the active persona
    When the operator reloads the persona cache
    Then the persona cache is refreshed

  Scenario: Operator is rejected without valid admin key
    Given the backend service is running
    When the operator requests persona versions without admin key
    Then the request is rejected with 401

  Scenario: Operator deletes an inactive persona version from the history
    Given persona "gaspare" has version 1 active and version 2 inactive
    When the operator deletes version 2 of persona "gaspare"
    Then only 1 version of persona "gaspare" remains

  Scenario: Operator cannot delete the active persona version
    Given persona "gaspare" has version 1 active and version 2 inactive
    When the operator deletes version 1 of persona "gaspare"
    Then the request is rejected with 409

  Scenario: Operator deletes an unknown persona version
    Given the knowledge base contains persona "gaspare" with version 1 active
    When the operator deletes persona version 999999
    Then the request is rejected with 404
