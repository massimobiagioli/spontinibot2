use crate::error::IngestError;

pub fn version() -> &'static str {
    "chunking module 0.1.0"
}

pub struct Chunk {
    pub content: String,
    pub section_tag: String,
    pub chunk_index: usize,
    pub token_count: usize,
    pub metadata: Option<String>,
}

pub struct Chunker {
    pub chunk_size: usize,
    pub overlap: usize,
}

impl Chunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Result<Self, IngestError> {
        if overlap * 2 >= chunk_size {
            return Err(IngestError::Chunking(
                "overlap must be less than half the chunk size".into(),
            ));
        }
        Ok(Self {
            chunk_size,
            overlap,
        })
    }

    pub fn chunk(
        &self,
        text: &str,
        section_tag: &str,
        source_url: &str,
    ) -> Result<Vec<Chunk>, IngestError> {
        if text.is_empty() {
            return Err(IngestError::Chunking("empty input".into()));
        }

        let metadata =
            Some(serde_json::json!({"section": section_tag, "source_url": source_url}).to_string());

        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .flat_map(|p| {
                let trimmed = p.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .collect();

        let mut chunks: Vec<Chunk> = Vec::new();
        let mut current_parts: Vec<String> = Vec::new();
        let mut current_tokens: usize = 0;
        let overlap_chars = self.overlap * 4;

        for paragraph in &paragraphs {
            let para_tokens = naive_token_count(paragraph);

            if current_parts.is_empty() && para_tokens > self.chunk_size {
                let sub_chunks = split_long_paragraph(paragraph, self.chunk_size, &metadata);
                for sc in sub_chunks {
                    chunks.push(sc);
                }
                continue;
            }

            if current_tokens + para_tokens > self.chunk_size && !current_parts.is_empty() {
                let content = current_parts.join("\n\n");
                let token_count = naive_token_count(&content);
                chunks.push(Chunk {
                    chunk_index: chunks.len(),
                    content,
                    section_tag: section_tag.to_string(),
                    token_count,
                    metadata: metadata.clone(),
                });

                let overlap_text = if overlap_chars > 0 {
                    let joined = current_parts.join("\n\n");
                    let start = joined.len().saturating_sub(overlap_chars);
                    joined[start..].to_string()
                } else {
                    String::new()
                };

                current_parts = if overlap_text.is_empty() {
                    Vec::new()
                } else {
                    vec![overlap_text]
                };
                current_tokens =
                    naive_token_count(current_parts.first().map(|s| s.as_str()).unwrap_or(""));
            }

            current_parts.push((*paragraph).to_string());
            current_tokens += para_tokens;
        }

        if !current_parts.is_empty() {
            let content = current_parts.join("\n\n");
            let token_count = naive_token_count(&content);
            chunks.push(Chunk {
                chunk_index: chunks.len(),
                content,
                section_tag: section_tag.to_string(),
                token_count,
                metadata: metadata.clone(),
            });
        }

        Ok(chunks)
    }
}

pub fn naive_token_count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let count = text.len() / 4;
    if count == 0 { 1 } else { count }
}

fn split_long_paragraph(
    paragraph: &str,
    chunk_size: usize,
    metadata: &Option<String>,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let chunk_chars = chunk_size * 4;
    let mut start = 0;

    while start < paragraph.len() {
        let end = if start + chunk_chars >= paragraph.len() {
            paragraph.len()
        } else {
            let slice = &paragraph[start..start + chunk_chars];
            let last_space = slice.rfind(' ').unwrap_or(chunk_chars);
            start + last_space
        };

        let content = &paragraph[start..end];
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            chunks.push(Chunk {
                chunk_index: chunks.len(),
                content: trimmed.to_string(),
                section_tag: String::new(),
                token_count: naive_token_count(trimmed),
                metadata: metadata.clone(),
            });
        }

        start = if end == paragraph.len() {
            paragraph.len()
        } else {
            end + 1
        };
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_single_chunk_when_text_is_short() {
        let chunker = Chunker::new(512, 64).unwrap();
        let text = "A".repeat(100);
        let chunks = chunker.chunk(&text, "news", "https://example.com").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].section_tag, "news");
    }

    #[test]
    fn should_split_into_multiple_chunks_when_text_is_long() {
        let chunker = Chunker::new(512, 64).unwrap();
        let text = (0..200)
            .map(|i| {
                format!(
                    "Paragraph number {} with some extra padding text for good measure.",
                    i
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = chunker
            .chunk(&text, "sport", "https://example.com")
            .unwrap();
        assert!(
            chunks.len() > 1,
            "should have multiple chunks, got {}",
            chunks.len()
        );
    }

    #[test]
    fn should_include_overlap_between_chunks() {
        let chunker = Chunker::new(50, 10).unwrap();
        let text = (0..10)
            .map(|i| format!("This is paragraph number {} with some content.", i))
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = chunker.chunk(&text, "test", "https://example.com").unwrap();

        if chunks.len() >= 2 {
            let first_tail = &chunks[0].content[chunks[0].content.len().saturating_sub(40)..];
            let second_start = &chunks[1].content[..40.min(chunks[1].content.len())];
            assert!(
                chunks[1].content.contains(first_tail.trim()) || first_tail.contains(second_start),
                "expected overlap between chunks"
            );
        }
    }

    #[test]
    fn should_return_error_when_text_empty() {
        let chunker = Chunker::new(512, 64).unwrap();
        let result = chunker.chunk("", "news", "https://example.com");
        assert!(matches!(result, Err(IngestError::Chunking(_))));
    }

    #[test]
    fn should_have_increasing_chunk_indices() {
        let chunker = Chunker::new(100, 20).unwrap();
        let text = (0..30)
            .map(|i| {
                format!(
                    "Paragraph {} with enough words to fill multiple chunks okay.",
                    i
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = chunker.chunk(&text, "test", "https://example.com").unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }

    #[test]
    fn should_generate_json_metadata_with_section_and_url() {
        let chunker = Chunker::new(512, 64).unwrap();
        let chunks = chunker
            .chunk(
                "Hello world content test.",
                "delibere",
                "https://comune.example.it",
            )
            .unwrap();
        let meta = chunks[0]
            .metadata
            .as_ref()
            .expect("metadata should be Some");
        assert!(meta.contains("delibere"));
        assert!(meta.contains("comune.example.it"));
    }

    #[test]
    fn should_set_correct_section_tag_on_all_chunks() {
        let chunker = Chunker::new(50, 10).unwrap();
        let text = (0..10)
            .map(|i| format!("Paragraph number {} with content.", i))
            .collect::<Vec<_>>()
            .join("\n\n");
        let chunks = chunker
            .chunk(&text, "storia", "https://example.com")
            .unwrap();
        for chunk in &chunks {
            assert_eq!(
                chunk.section_tag, "storia",
                "all chunks should have section_tag 'storia'"
            );
        }
    }

    #[test]
    fn should_handle_single_paragraph_larger_than_chunk_size() {
        let chunker = Chunker::new(50, 10).unwrap();
        let long_para = "word ".repeat(500);
        let text = long_para.to_string();
        let chunks = chunker.chunk(&text, "test", "https://example.com").unwrap();
        assert!(
            chunks.len() > 1,
            "long paragraph should split into multiple chunks"
        );
        for chunk in &chunks {
            assert!(!chunk.content.is_empty(), "no empty chunks");
        }
    }

    #[test]
    fn should_return_error_when_overlap_too_large() {
        let result = Chunker::new(100, 60);
        assert!(result.is_err());
    }

    #[test]
    fn should_count_tokens_naively() {
        assert_eq!(naive_token_count(""), 0);
        assert_eq!(naive_token_count("a"), 1);
        assert_eq!(naive_token_count("abcd"), 1);
        let forty_chars = "a".repeat(40);
        assert_eq!(naive_token_count(&forty_chars), 10);
    }
}
