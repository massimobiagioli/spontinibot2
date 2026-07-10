use async_trait::async_trait;

use crate::rag_engine::types::{
    AdminPersonaSnapshot, NewPersonaRequest, PersonaSnapshot, PromptParts, RagError, RetrievedChunk,
};

#[async_trait]
pub trait EmbeddingPort: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, RagError>;
}

#[async_trait]
pub trait RetrievalPort: Send + Sync {
    async fn retrieve(
        &self,
        query_embedding: &[f32],
        top_k: i64,
        min_score: f64,
    ) -> Result<Vec<RetrievedChunk>, RagError>;
}

#[async_trait]
pub trait PersonaPort: Send + Sync {
    async fn active_persona(&self) -> Result<Option<PersonaSnapshot>, RagError>;
    async fn reload_persona(&self) -> Result<(), RagError>;
}

#[async_trait]
pub trait GenerationPort: Send + Sync {
    async fn generate(&self, prompt: PromptParts) -> Result<String, RagError>;
}

#[async_trait]
pub trait PersonaAdminPort: Send + Sync {
    async fn list_versions(&self, name: &str) -> Result<Vec<AdminPersonaSnapshot>, RagError>;
    async fn insert_persona(
        &self,
        req: NewPersonaRequest,
    ) -> Result<AdminPersonaSnapshot, RagError>;
    async fn activate_persona(&self, id: i64) -> Result<(), RagError>;
    async fn reload_persona(&self) -> Result<(), RagError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _assert_dyn_embedding() -> Box<dyn EmbeddingPort> {
        todo!()
    }

    fn _assert_dyn_retrieval() -> Box<dyn RetrievalPort> {
        todo!()
    }

    fn _assert_dyn_persona() -> Box<dyn PersonaPort> {
        todo!()
    }

    fn _assert_dyn_generation() -> Box<dyn GenerationPort> {
        todo!()
    }

    fn _assert_dyn_persona_admin() -> Box<dyn PersonaAdminPort> {
        todo!()
    }

    fn _assert_arc_dyn_embedding() -> std::sync::Arc<dyn EmbeddingPort> {
        todo!()
    }

    fn _assert_arc_dyn_retrieval() -> std::sync::Arc<dyn RetrievalPort> {
        todo!()
    }

    fn _assert_arc_dyn_persona() -> std::sync::Arc<dyn PersonaPort> {
        todo!()
    }

    fn _assert_arc_dyn_generation() -> std::sync::Arc<dyn GenerationPort> {
        todo!()
    }
}
