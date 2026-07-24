Feature: Point-in-answer feedback on a training message

  Scenario: Operator leaves positive feedback on a span of a recorded answer
    Given the knowledge base contains a document titled "Orari sportello anagrafe"
    And the document contains the text "Lo sportello anagrafe e' aperto dal lunedi' al venerdi' dalle 9:00 alle 12:30"
    And an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione formazione anagrafe"
    And the operator has asked "A che ore apre l'anagrafe?" in that training session
    When the operator leaves positive feedback on the span "alle 9:00" of that message
    Then the feedback list for that message contains a positive entry for "alle 9:00"

  Scenario: Operator leaves negative feedback with a comment on the same message
    Given the knowledge base contains a document titled "Orari sportello anagrafe"
    And the document contains the text "Lo sportello anagrafe e' aperto dal lunedi' al venerdi' dalle 9:00 alle 12:30"
    And an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione formazione anagrafe"
    And the operator has asked "A che ore apre l'anagrafe?" in that training session
    And the operator has left positive feedback on the span "alle 9:00" of that message
    When the operator leaves negative feedback on the span "dal lunedi'" of that message with comment "giorni sbagliati"
    Then the feedback list for that message contains a negative entry for "dal lunedi'" with comment "giorni sbagliati"
    And the feedback list for that message contains 2 entries

  Scenario: Operator leaves feedback anchored to a specific cited chunk
    Given the knowledge base contains a document titled "Orari sportello anagrafe"
    And the document contains the text "Lo sportello anagrafe e' aperto dal lunedi' al venerdi' dalle 9:00 alle 12:30"
    And an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And a citable document exists in the knowledge base
    And the operator has created a training session titled "Sessione formazione anagrafe"
    And the operator has asked "A che ore apre l'anagrafe?" in that training session
    When the operator leaves positive feedback on the span "alle 9:00" of that message anchored to the cited chunk
    Then the feedback list for that message contains an entry anchored to a chunk

  Scenario: Operator leaving feedback with an invalid sentiment value is rejected
    Given an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione"
    And the operator has asked "domanda" in that training session
    When the operator leaves feedback with sentiment "neutral" on that message
    Then the request is rejected with 400

  Scenario: Operator leaving feedback on an unknown message
    Given the training messages API is available
    When the operator leaves feedback on unknown message 999999
    Then the training message is not found

  Scenario: Operator is rejected without admin key on submitting feedback
    Given an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione"
    And the operator has asked "domanda" in that training session
    When the operator leaves feedback without admin key
    Then the request is rejected with 401

  Scenario: Operator is rejected without admin key on listing feedback
    Given an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione"
    And the operator has asked "domanda" in that training session
    When the operator lists feedback for that message without admin key
    Then the request is rejected with 401
