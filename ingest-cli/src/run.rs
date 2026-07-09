use ingest_core::pipeline::{IngestPipeline, Pipeline};
use kb_store::{KbStore, SourceType};

pub struct RunConfig {
    pub kb_path: String,
    pub embedder_base_url: String,
    pub user_agent: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            kb_path: std::env::var("KB_PATH").unwrap_or_else(|_| "/data/kb.db".into()),
            embedder_base_url: std::env::var("EMBEDDER_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8081".into()),
            user_agent: std::env::var("USER_AGENT").unwrap_or_else(|_| "ingest-cli/0.1.0".into()),
            chunk_size: std::env::var("CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(512),
            chunk_overlap: std::env::var("CHUNK_OVERLAP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(64),
        }
    }
}

pub async fn run_url(
    url: &str,
    section: &str,
    config: &RunConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let kb = KbStore::open(&config.kb_path).await?;
    let pipeline = IngestPipeline::new(
        config.user_agent.clone(),
        config.embedder_base_url.clone(),
        config.chunk_size,
        config.chunk_overlap,
        kb,
    )?;
    pipeline.run(url, section).await?;
    Ok(())
}

pub async fn run_all_sources(
    section_name: &str,
    config: &RunConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let kb = KbStore::open(&config.kb_path).await?;
    let sections = kb.list_sections().await?;

    let section = sections
        .iter()
        .find(|s| s.name == section_name)
        .ok_or_else(|| format!("section '{}' not found in kb.db", section_name))?;

    let sources = kb.list_sources_by_section(section.id).await?;
    let enabled_scrape: Vec<_> = sources
        .iter()
        .filter(|s| s.source_type == SourceType::Scrape && s.enabled)
        .collect();

    if enabled_scrape.is_empty() {
        println!(
            "no enabled scrape sources found for section '{}'",
            section_name
        );
        return Ok(());
    }

    let pipeline = IngestPipeline::new(
        config.user_agent.clone(),
        config.embedder_base_url.clone(),
        config.chunk_size,
        config.chunk_overlap,
        kb,
    )?;

    for source in &enabled_scrape {
        println!("ingesting: {} (section {})", source.url, section_name);
        if let Err(e) = pipeline.run(&source.url, section_name).await {
            eprintln!("warning: failed to ingest {}: {}", source.url, e);
        } else {
            println!("ingested: {} -> section {}", source.url, section_name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_default_config_when_no_env_vars() {
        let config = RunConfig::default();
        assert_eq!(config.kb_path, "/data/kb.db");
        assert_eq!(config.embedder_base_url, "http://localhost:8081");
        assert_eq!(config.user_agent, "ingest-cli/0.1.0");
        assert_eq!(config.chunk_size, 512);
        assert_eq!(config.chunk_overlap, 64);
    }
}
