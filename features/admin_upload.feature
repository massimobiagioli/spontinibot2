Feature: Manual document upload via admin API

  Scenario: Operator uploads, previews, and confirms a Markdown document
    Given a persona is configured in the knowledge base
    And the backend service has the upload API enabled
    When the operator uploads a file "test-article.md" with section "news"
    Then the upload returns a preview token
    When the operator requests the preview with that token
    Then the preview shows the extracted text and metadata
    When the operator confirms the upload with that token
    Then the confirm response includes document IDs and a chunk count

  Scenario: Operator uploads an unsupported format
    Given the backend service has the upload API enabled
    When the operator uploads a file "image.jpg" with section "news"
    Then the upload is rejected with an unsupported format error

  Scenario: Operator is rejected without admin key on upload
    Given the backend service has the upload API enabled
    When the operator uploads a file "test.txt" with section "news" without admin key
    Then the request is rejected with 401
