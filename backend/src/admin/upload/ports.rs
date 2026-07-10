use async_trait::async_trait;

use super::UploadError;
use super::preview_store::UploadMetadata;

#[async_trait]
pub trait UploadPort: Send + Sync {
    async fn ingest_uploaded(
        &self,
        text: &str,
        section: &str,
        filename: &str,
        metadata: &UploadMetadata,
    ) -> Result<Vec<i64>, UploadError>;
}
