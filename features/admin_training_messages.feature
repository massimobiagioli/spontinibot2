Feature: Ask/answer with recording in a training session

  Scenario: Operator asks a question answerable from a municipal document and it is recorded
    Given the knowledge base contains a document titled "Orari sportello anagrafe"
    And the document contains the text "Lo sportello anagrafe e' aperto dal lunedi' al venerdi' dalle 9:00 alle 12:30"
    And an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione formazione anagrafe"
    When the operator asks "A che ore apre l'anagrafe?" in that training session
    Then the training message answers using the content of the retrieved document
    And the training message cites the source document by title
    And the training message is not a fallback

  Scenario: The recorded exchange appears in the session's message list
    Given the knowledge base contains a document titled "Orari sportello anagrafe"
    And the document contains the text "Lo sportello anagrafe e' aperto dal lunedi' al venerdi' dalle 9:00 alle 12:30"
    And an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione formazione anagrafe"
    And the operator has asked "A che ore apre l'anagrafe?" in that training session
    When the operator lists that training session's messages
    Then the training message list contains a message with question "A che ore apre l'anagrafe?"

  Scenario: Operator asks a question with no matching knowledge base content and the honest-unknown fallback is recorded
    Given the knowledge base contains no document about "tasse comunali"
    And an active persona is configured with a system prompt and a fallback message
    And the training messages API is available
    And the operator has created a training session titled "Sessione formazione tasse"
    When the operator asks "Quanto pago di tasse comunali?" in that training session
    Then the training message is a fallback
    And the training message has no cited sources

  Scenario: Operator asks a question in an unknown training session
    Given the training messages API is available
    When the operator asks "Domanda" in training session 999999
    Then the training session is not found

  Scenario: Operator is rejected without admin key on ask
    Given the training messages API is available
    And the operator has created a training session titled "Sessione"
    When the operator asks a question in that training session without admin key
    Then the request is rejected with 401

  Scenario: Operator is rejected without admin key on listing messages
    Given the training messages API is available
    And the operator has created a training session titled "Sessione"
    When the operator lists that training session's messages without admin key
    Then the request is rejected with 401
