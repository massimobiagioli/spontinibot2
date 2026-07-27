Feature: Training session lifecycle via admin API

  Scenario: Operator creates a training session and it appears in the list
    Given the training sessions API is available
    When the operator creates a training session titled "Sessione formazione anagrafe"
    Then the training session list contains a session titled "Sessione formazione anagrafe"

  Scenario: Operator retrieves a single training session by id
    Given the training sessions API is available
    And the operator has created a training session titled "Sessione di prova"
    When the operator retrieves that training session
    Then the retrieved training session is titled "Sessione di prova"
    And the retrieved training session is open

  Scenario: Operator closes an open training session
    Given the training sessions API is available
    And the operator has created a training session titled "Sessione di prova"
    When the operator closes that training session
    Then the training session is closed

  Scenario: Operator closing an already-closed training session is a no-op
    Given the training sessions API is available
    And the operator has created a training session titled "Sessione di prova"
    And the operator has closed that training session
    When the operator closes that training session again
    Then closing the training session has no effect

  Scenario: Operator looks up an unknown training session
    Given the training sessions API is available
    When the operator retrieves training session 999999
    Then the training session is not found

  Scenario: Operator is rejected without admin key on create
    Given the training sessions API is available
    When the operator creates a training session without admin key
    Then the request is rejected with 401

  Scenario: Operator is rejected without admin key on list
    Given the training sessions API is available
    When the operator lists training sessions without admin key
    Then the request is rejected with 401

  Scenario: Operator is rejected without admin key on get
    Given the training sessions API is available
    When the operator retrieves training session 1 without admin key
    Then the request is rejected with 401

  Scenario: Operator is rejected without admin key on close
    Given the training sessions API is available
    When the operator closes training session 1 without admin key
    Then the request is rejected with 401

  Scenario: Operator closes a session with closing notes
    Given the training sessions API is available
    And the operator has created a training session titled "Sessione di prova"
    When the operator closes that training session with notes "Buona sessione, nessun problema"
    Then the training session is closed
    And the retrieved training session has notes "Buona sessione, nessun problema"

  Scenario: Operator deletes a training session
    Given the training sessions API is available
    And the operator has created a training session titled "Sessione di prova"
    When the operator deletes that training session
    Then the training session is deleted
