use redis::aio::ConnectionManager;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

// ============================================================
// Dragonfly Cache — High-Speed Cognitive RAM
// ============================================================
//
// Layer Role:
//   Qdrant  → Long-term semantic memory (meaning)
//   Meili   → Indexed factual memory (words + filters)
//   Dragon  → Working memory / attention buffer (speed + glue)
//
// What lives here:
//   - Session context (conversation summary, active goal, recent topics)
//   - Mental state (mood, energy, focus — the "emotion registers")
//   - Embedding cache (hash → vector, avoids re-computing via Ollama)
//   - Tool execution history
//   - Agent coordination queues (pub/sub between systems)
//   - Rate limiting / token budget tracking
// ============================================================

/// Session context — the agent's "attention buffer" for an active conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub chat_id: String,
    pub conversation_summary: String,
    pub active_goal: Option<String>,
    pub recent_topics: Vec<String>,
    pub turn_count: u32,
    pub user_persona_id: Option<String>,
    pub ai_persona_id: Option<String>,
    pub last_user_message: String,
    pub last_assistant_message: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Mental state stored in Dragonfly (synced to agent state on tick)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMentalState {
    pub mood: f32,
    pub energy: f32,
    pub focus_level: f32,
    pub mood_label: String,
    pub is_dreaming: bool,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Tool execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub success: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct CacheService;

impl CacheService {
    // ── Primitive operations ──────────────────────────────────

    /// Get cached value
    pub async fn get(cache: &ConnectionManager, key: &str) -> Result<Option<String>> {
        let mut con = cache.clone();
        let value: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut con)
            .await?;
        Ok(value)
    }

    /// Set cached value with TTL
    pub async fn set(cache: &ConnectionManager, key: &str, value: &str, ttl_secs: usize) -> Result<()> {
        let mut con = cache.clone();
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl_secs)
            .query_async::<_, ()>(&mut con)
            .await?;
        Ok(())
    }

    /// Set cached value without TTL (persistent)
    pub async fn set_persistent(cache: &ConnectionManager, key: &str, value: &str) -> Result<()> {
        let mut con = cache.clone();
        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .query_async::<_, ()>(&mut con)
            .await?;
        Ok(())
    }

    /// Delete a key
    #[allow(dead_code)]
    pub async fn del(cache: &ConnectionManager, key: &str) -> Result<()> {
        let mut con = cache.clone();
        redis::cmd("DEL")
            .arg(key)
            .query_async::<_, ()>(&mut con)
            .await?;
        Ok(())
    }

    /// Push signal to queue
    pub async fn queue_signal(cache: &ConnectionManager, queue: &str, signal: &str) -> Result<()> {
        let mut con = cache.clone();
        redis::cmd("RPUSH")
            .arg(queue)
            .arg(signal)
            .query_async::<_, ()>(&mut con)
            .await?;
        Ok(())
    }

    /// Pop signal from queue
    pub async fn dequeue_signal(cache: &ConnectionManager, queue: &str) -> Result<Option<String>> {
        let mut con = cache.clone();
        let value: Option<String> = redis::cmd("LPOP")
            .arg(queue)
            .query_async(&mut con)
            .await?;
        Ok(value)
    }

    // ── Mental State (Emotion Registers) ─────────────────────

    /// Store full mental state
    pub async fn set_mental_state(cache: &ConnectionManager, state: &CachedMentalState) -> Result<()> {
        let json = serde_json::to_string(state)?;
        // No TTL — mental state persists until explicitly updated
        Self::set_persistent(cache, "cognitive:mental_state", &json).await
    }

    /// Get full mental state
    pub async fn get_mental_state(cache: &ConnectionManager) -> Result<Option<CachedMentalState>> {
        if let Some(json) = Self::get(cache, "cognitive:mental_state").await? {
            Ok(serde_json::from_str(&json).ok())
        } else {
            Ok(None)
        }
    }

    /// Quick mood update (called after infer_mood)
    pub async fn update_mood(cache: &ConnectionManager, mood_value: f32, mood_label: &str, energy_delta: f32) -> Result<()> {
        let mut state = Self::get_mental_state(cache).await?.unwrap_or(CachedMentalState {
            mood: 0.5,
            energy: 0.7,
            focus_level: 0.8,
            mood_label: "content".to_string(),
            is_dreaming: false,
            last_active: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        state.mood = mood_value;
        state.mood_label = mood_label.to_string();
        state.energy = (state.energy + energy_delta).clamp(0.0, 1.0);
        state.last_active = chrono::Utc::now();
        state.updated_at = chrono::Utc::now();
        Self::set_mental_state(cache, &state).await
    }

    // ── Session Context (Attention Buffer) ───────────────────

    fn session_key(chat_id: &str) -> String {
        format!("cognitive:session:{}", chat_id)
    }

    /// Store session context for a chat
    pub async fn set_session(cache: &ConnectionManager, session: &SessionContext) -> Result<()> {
        let json = serde_json::to_string(session)?;
        // Sessions expire after 24h of inactivity
        Self::set(cache, &Self::session_key(&session.chat_id), &json, 86400).await
    }

    /// Get session context for a chat
    pub async fn get_session(cache: &ConnectionManager, chat_id: &str) -> Result<Option<SessionContext>> {
        if let Some(json) = Self::get(cache, &Self::session_key(chat_id)).await? {
            Ok(serde_json::from_str(&json).ok())
        } else {
            Ok(None)
        }
    }

    /// Update session after an exchange (user msg + assistant response)
    pub async fn update_session_after_exchange(
        cache: &ConnectionManager,
        chat_id: &str,
        user_msg: &str,
        assistant_msg: &str,
        summary: &str,
        topics: Vec<String>,
    ) -> Result<()> {
        let mut session = Self::get_session(cache, chat_id).await?.unwrap_or(SessionContext {
            chat_id: chat_id.to_string(),
            conversation_summary: String::new(),
            active_goal: None,
            recent_topics: Vec::new(),
            turn_count: 0,
            user_persona_id: None,
            ai_persona_id: None,
            last_user_message: String::new(),
            last_assistant_message: String::new(),
            updated_at: chrono::Utc::now(),
        });
        session.conversation_summary = summary.to_string();
        session.last_user_message = user_msg.to_string();
        session.last_assistant_message = assistant_msg.to_string();
        session.turn_count += 1;
        // Keep last 10 topics, deduped
        for topic in topics {
            if !session.recent_topics.contains(&topic) {
                session.recent_topics.push(topic);
            }
        }
        if session.recent_topics.len() > 10 {
            let drain_count = session.recent_topics.len() - 10;
            session.recent_topics.drain(..drain_count);
        }
        session.updated_at = chrono::Utc::now();
        Self::set_session(cache, &session).await
    }

    // ── Embedding Cache (Avoid Recomputing) ──────────────────

    fn embedding_key(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hex::encode(hasher.finalize());
        format!("emb:{}", &hash[..16])
    }

    /// Cache an embedding vector
    pub async fn cache_embedding(cache: &ConnectionManager, text: &str, embedding: &[f32]) -> Result<()> {
        use base64::Engine;
        let key = Self::embedding_key(text);
        // Store as compact binary: 4 bytes per f32
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        // Embeddings cache for 7 days
        Self::set(cache, &key, &encoded, 604800).await
    }

    /// Get cached embedding vector
    pub async fn get_cached_embedding(cache: &ConnectionManager, text: &str) -> Result<Option<Vec<f32>>> {
        use base64::Engine;
        let key = Self::embedding_key(text);
        if let Some(encoded) = Self::get(cache, &key).await? {
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&encoded) {
                let embedding: Vec<f32> = bytes.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                return Ok(Some(embedding));
            }
        }
        Ok(None)
    }

    // ── Tool Execution History ───────────────────────────────

    /// Record a tool execution
    /// TODO(future): Wire up when agent tool pipeline is active
    #[allow(dead_code)]
    pub async fn record_tool_execution(cache: &ConnectionManager, exec: &ToolExecution) -> Result<()> {
        let json = serde_json::to_string(exec)?;
        let mut con = cache.clone();
        redis::cmd("RPUSH")
            .arg("cognitive:tool_history")
            .arg(&json)
            .query_async::<_, ()>(&mut con)
            .await?;
        // Keep last 50 tool executions
        redis::cmd("LTRIM")
            .arg("cognitive:tool_history")
            .arg(-50i64)
            .arg(-1i64)
            .query_async::<_, ()>(&mut con)
            .await?;
        Ok(())
    }

    /// Get recent tool executions
    /// TODO(future): Wire up when agent tool pipeline is active
    #[allow(dead_code)]
    pub async fn get_tool_history(cache: &ConnectionManager, count: isize) -> Result<Vec<ToolExecution>> {
        let mut con = cache.clone();
        let items: Vec<String> = redis::cmd("LRANGE")
            .arg("cognitive:tool_history")
            .arg(-count)
            .arg(-1i64)
            .query_async(&mut con)
            .await
            .unwrap_or_default();
        Ok(items.iter()
            .filter_map(|s| serde_json::from_str(s).ok())
            .collect())
    }

    // ── Token Budget Tracking ────────────────────────────────

    /// Track token usage for rate limiting
    /// TODO(future): Wire up for token budget tracking per model
    #[allow(dead_code)]
    pub async fn track_tokens(cache: &ConnectionManager, model: &str, tokens: u64) -> Result<u64> {
        let key = format!("tokens:{}:{}", model, chrono::Utc::now().format("%Y-%m-%d"));
        let mut con = cache.clone();
        let total: u64 = redis::cmd("INCRBY")
            .arg(&key)
            .arg(tokens)
            .query_async(&mut con)
            .await?;
        // Expire daily counters after 48h
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(172800u64)
            .query_async::<_, ()>(&mut con)
            .await?;
        Ok(total)
    }

    /// Get today's token usage for a model
    /// TODO(future): Wire up for token budget tracking per model
    #[allow(dead_code)]
    pub async fn get_token_usage(cache: &ConnectionManager, model: &str) -> Result<u64> {
        let key = format!("tokens:{}:{}", model, chrono::Utc::now().format("%Y-%m-%d"));
        let mut con = cache.clone();
        let count: u64 = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut con)
            .await
            .unwrap_or(0);
        Ok(count)
    }

    // ── Legacy compatibility ─────────────────────────────────

    /// Update mood state in cache (legacy — prefer update_mood)
    #[allow(dead_code)]
    pub async fn set_mood(cache: &ConnectionManager, mood: f32, energy: f32) -> Result<()> {
        Self::update_mood(cache, mood, "content", energy - 0.7).await
    }

    /// Get mood state from cache (legacy — prefer get_mental_state)
    #[allow(dead_code)]
    pub async fn get_mood(cache: &ConnectionManager) -> Result<Option<(f32, f32)>> {
        if let Some(state) = Self::get_mental_state(cache).await? {
            Ok(Some((state.mood, state.energy)))
        } else {
            Ok(None)
        }
    }
}
