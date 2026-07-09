Feature: Backend service health

  Scenario: Operator verifies the backend is running
    Given the backend service is running
    When the operator checks the service health
    Then the service reports it is ok
