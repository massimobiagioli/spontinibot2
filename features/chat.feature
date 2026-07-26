Feature: Answering citizen questions from the knowledge base

  Scenario: A citizen asks a question answerable from a municipal document
    Given the knowledge base contains a document titled "Orari sportello anagrafe"
    And the document contains the text "Lo sportello anagrafe e' aperto dal lunedi' al venerdi' dalle 9:00 alle 12:30"
    And an active persona is configured with a system prompt and a fallback message
    When the citizen asks "A che ore apre l'anagrafe?"
    Then Spontini answers using the content of the retrieved document
    And Spontini cites the source document by title
    And the final prompt keeps the persona, retrieved context, and question as three separate parts

  Scenario: A citizen asks a question not answerable from any document
    Given the knowledge base contains no document about "tasse comunali"
    And an active persona is configured with a system prompt and a fallback message
    When the citizen asks "Quanto pago di tasse comunali?"
    Then Spontini answers with the fallback message
    And Spontini does not cite any document
    And Spontini does not invent any detail
    And the final prompt keeps the persona, retrieved context, and question as three separate parts

  Scenario: A citizen asks who Spontini is
    Given an active persona is configured with a system prompt and a fallback message
    When the citizen asks "Chi sei?"
    Then Spontini answers instantly from its own persona, without retrieval or generation
