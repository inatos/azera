use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vector database service for semantic search (Qdrant)
pub struct VectorService {
    pub client: reqwest::Client,
    pub base_url: String,
}

/// Internal Qdrant point representation
/// TODO(future): Used for direct Qdrant operations
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct QdrantPoint {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub payload: HashMap<String, serde_json::Value>,
}

impl VectorService {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Initialize the collection for Azera's memory
    pub async fn init_collection(&self, collection_name: &str, vector_size: usize) -> Result<()> {
        // Check if collection exists
        let check_url = format!("{}/collections/{}", self.base_url, collection_name);
        let check_response = self.client.get(&check_url).send().await?;
        
        if check_response.status().is_success() {
            tracing::info!("Collection '{}' already exists", collection_name);
            return Ok(());
        }

        // Create collection
        let create_url = format!("{}/collections/{}", self.base_url, collection_name);
        let body = serde_json::json!({
            "vectors": {
                "size": vector_size,
                "distance": "Cosine"
            }
        });

        let response = self.client
            .put(&create_url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow::anyhow!("Failed to create collection: {}", error));
        }

        tracing::info!("Created collection '{}'", collection_name);
        Ok(())
    }

    /// Upsert a vector into the collection
    pub async fn upsert(
        &self,
        collection_name: &str,
        id: &str,
        vector: Vec<f32>,
        payload: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let url = format!("{}/collections/{}/points", self.base_url, collection_name);
        
        tracing::debug!("🧠 Upserting to Qdrant: url={}, id={}, vector_len={}", url, id, vector.len());
        
        let body = serde_json::json!({
            "points": [{
                "id": id,
                "vector": vector,
                "payload": payload
            }]
        });

        let response = self.client
            .put(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            tracing::error!("🧠 Qdrant upsert failed: {}", error);
            return Err(anyhow::anyhow!("Failed to upsert vector: {}", error));
        }

        Ok(())
    }

    /// Search for similar vectors
    pub async fn search(
        &self,
        collection_name: &str,
        query_vector: Vec<f32>,
        limit: usize,
        filter: Option<serde_json::Value>,
    ) -> Result<Vec<SearchResult>> {
        let url = format!("{}/collections/{}/points/search", self.base_url, collection_name);
        
        let mut body = serde_json::json!({
            "vector": query_vector,
            "limit": limit,
            "with_payload": true
        });

        if let Some(f) = filter {
            body["filter"] = f;
        }

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow::anyhow!("Search failed: {}", error));
        }

        let json: serde_json::Value = response.json().await?;
        let results = json["result"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(SearchResult {
                            id: item["id"].as_str()?.to_string(),
                            score: item["score"].as_f64()? as f32,
                            payload: serde_json::from_value(item["payload"].clone()).ok()?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    /// Delete a vector by ID
    pub async fn delete(&self, collection_name: &str, id: &str) -> Result<()> {
        let url = format!("{}/collections/{}/points/delete", self.base_url, collection_name);
        
        let body = serde_json::json!({
            "points": [id]
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow::anyhow!("Failed to delete vector: {}", error));
        }

        Ok(())
    }

    /// Generate embeddings using Ollama (with Dragonfly cache)
    pub async fn generate_embedding_cached(
        &self,
        ollama_host: &str,
        text: &str,
        cache: &redis::aio::ConnectionManager,
    ) -> Result<Vec<f32>> {
        // Check Dragonfly cache first
        if let Ok(Some(cached)) = crate::cache::CacheService::get_cached_embedding(cache, text).await {
            tracing::debug!("⚡ Embedding cache hit");
            return Ok(cached);
        }

        // Cache miss — compute via Ollama
        let embedding = self.generate_embedding(ollama_host, text).await?;

        // Store in Dragonfly for next time (fire-and-forget)
        let cache_clone = cache.clone();
        let embedding_clone = embedding.clone();
        let text_owned = text.to_string();
        tokio::spawn(async move {
            let _ = crate::cache::CacheService::cache_embedding(&cache_clone, &text_owned, &embedding_clone).await;
        });

        Ok(embedding)
    }

    /// Generate embeddings using Ollama
    pub async fn generate_embedding(&self, ollama_host: &str, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", ollama_host);
        
        let body = serde_json::json!({
            "model": "nomic-embed-text",  // Use embedding model
            "prompt": text
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            // Fallback to simple hash-based embedding if Ollama doesn't have embedding model
            tracing::warn!("Ollama embedding failed, using fallback");
            return Ok(self.simple_embedding(text));
        }

        let json: serde_json::Value = response.json().await?;
        let embedding = json["embedding"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            })
            .unwrap_or_else(|| self.simple_embedding(text));

        Ok(embedding)
    }

    /// Simple fallback embedding using hash (for testing)
    fn simple_embedding(&self, text: &str) -> Vec<f32> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();
        
        // Convert hash to 384-dim vector (typical embedding size)
        let mut embedding = Vec::with_capacity(384);
        for i in 0..384 {
            let byte = hash[i % 32] as f32;
            embedding.push((byte / 255.0) * 2.0 - 1.0); // Normalize to [-1, 1]
        }
        
        embedding
    }
}

/// Memory types for storing in vector DB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Conversation,
    Dream,
    Reflection,
    Fact,
    Emotion,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Conversation => write!(f, "conversation"),
            MemoryType::Dream => write!(f, "dream"),
            MemoryType::Reflection => write!(f, "reflection"),
            MemoryType::Fact => write!(f, "fact"),
            MemoryType::Emotion => write!(f, "emotion"),
        }
    }
}

pub struct StoreMemoryRequest {
    pub collection: String,
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Store a memory in the vector database (non-cached variant)
/// Prefer `store_memory_cached` for production use; kept for testing/direct access
#[allow(dead_code)]
pub async fn store_memory(
    vector_service: &VectorService,
    ollama_host: &str,
    collection: &str,
    id: &str,
    content: &str,
    memory_type: MemoryType,
    metadata: HashMap<String, serde_json::Value>,
) -> Result<()> {
    let embedding = vector_service.generate_embedding(ollama_host, content).await?;
    
    let mut payload = metadata;
    payload.insert("content".to_string(), serde_json::json!(content));
    payload.insert("type".to_string(), serde_json::json!(memory_type.to_string()));
    payload.insert("timestamp".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
    
    vector_service.upsert(collection, id, embedding, payload).await?;
    
    Ok(())
}

/// Store a memory with embedding cache (Dragonfly-accelerated)
pub async fn store_memory_cached(
    vector_service: &VectorService,
    ollama_host: &str,
    cache: &redis::aio::ConnectionManager,
    request: &StoreMemoryRequest,
) -> Result<()> {
    let embedding = vector_service
        .generate_embedding_cached(ollama_host, &request.content, cache)
        .await?;
    
    let mut payload = request.metadata.clone();
    payload.insert("content".to_string(), serde_json::json!(request.content));
    payload.insert("type".to_string(), serde_json::json!(request.memory_type.to_string()));
    payload.insert("timestamp".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
    
    vector_service
        .upsert(&request.collection, &request.id, embedding, payload)
        .await?;
    
    Ok(())
}

/// Search memories by semantic similarity (non-cached variant)
/// Prefer `search_memories_cached` for production use; kept for testing/direct access
#[allow(dead_code)]
pub async fn search_memories(
    vector_service: &VectorService,
    ollama_host: &str,
    collection: &str,
    query: &str,
    limit: usize,
    memory_type: Option<MemoryType>,
) -> Result<Vec<SearchResult>> {
    let query_embedding = vector_service.generate_embedding(ollama_host, query).await?;
    
    let filter = memory_type.map(|t| {
        serde_json::json!({
            "must": [{
                "key": "type",
                "match": { "value": t.to_string() }
            }]
        })
    });
    
    vector_service.search(collection, query_embedding, limit, filter).await
}

/// Search memories with Dragonfly embedding cache
pub async fn search_memories_cached(
    vector_service: &VectorService,
    ollama_host: &str,
    cache: &redis::aio::ConnectionManager,
    collection: &str,
    query: &str,
    limit: usize,
    memory_type: Option<MemoryType>,
) -> Result<Vec<SearchResult>> {
    let query_embedding = vector_service.generate_embedding_cached(ollama_host, query, cache).await?;
    
    let filter = memory_type.map(|t| {
        serde_json::json!({
            "must": [{
                "key": "type",
                "match": { "value": t.to_string() }
            }]
        })
    });
    
    vector_service.search(collection, query_embedding, limit, filter).await
}

/// Search memories with a custom filter (for global persona memory, non-cached variant)
/// Prefer `search_memories_with_filter_cached` for production use; kept for testing/direct access
#[allow(dead_code)]
pub async fn search_memories_with_filter(
    vector_service: &VectorService,
    ollama_host: &str,
    collection: &str,
    query: &str,
    limit: usize,
    filter: Option<serde_json::Value>,
) -> Result<Vec<SearchResult>> {
    let query_embedding = vector_service.generate_embedding(ollama_host, query).await?;
    vector_service.search(collection, query_embedding, limit, filter).await
}

/// Search memories with a custom filter + Dragonfly embedding cache
pub async fn search_memories_with_filter_cached(
    vector_service: &VectorService,
    ollama_host: &str,
    cache: &redis::aio::ConnectionManager,
    collection: &str,
    query: &str,
    limit: usize,
    filter: Option<serde_json::Value>,
) -> Result<Vec<SearchResult>> {
    let query_embedding = vector_service.generate_embedding_cached(ollama_host, query, cache).await?;
    vector_service.search(collection, query_embedding, limit, filter).await
}
