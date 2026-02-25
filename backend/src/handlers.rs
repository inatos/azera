use crate::*;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{sse::{Event, Sse}, IntoResponse, Response},
    Json,
};
use futures::stream::{Stream, StreamExt};
use serde_json::json;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// ============================================================
// Image Generation from Chat Helper
// ============================================================

/// Pattern: [IMAGE_GEN: prompt="...", name="..."]
/// Returns: Vec<(prompt, custom_name)>
fn extract_image_gen_requests(text: &str) -> Vec<(String, Option<String>)> {
    let mut results = Vec::new();
    
    // Pattern to match [IMAGE_GEN: prompt="...", name="..."] or [IMAGE_GEN: prompt="..."]
    let pattern = r#"\[IMAGE_GEN:\s*prompt="([^"]+)"(?:\s*,\s*name="([^"]+)")?\]"#;
    let re = regex::Regex::new(pattern).unwrap();
    
    for cap in re.captures_iter(text) {
        let prompt = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let name = cap.get(2).map(|m| m.as_str().to_string());
        if !prompt.is_empty() {
            results.push((prompt, name));
        }
    }
    
    results
}

/// Trigger async image generation from chat
async fn trigger_image_generation(
    prompt: &str,
    custom_name: Option<&str>,
    persona_id: Option<&str>,
    db: &sqlx::Pool<sqlx::Postgres>,
) {
    tracing::info!("🎨 Triggering image generation from chat: {}", prompt);
    
    // Get persona name for filename prefix
    let persona_name: Option<String> = if let Some(pid) = persona_id {
        match crate::db::get_persona(db, pid).await {
            Ok(Some(p)) => Some(p.name.to_lowercase().replace(' ', "_")),
            _ => None,
        }
    } else {
        None
    };
    
    // Build filename with persona prefix
    let image_id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = if let Some(name) = custom_name {
        if let Some(ref pname) = persona_name {
            format!("{}_{}.png", pname, name.replace(' ', "_"))
        } else {
            format!("{}.png", name.replace(' ', "_"))
        }
    } else if let Some(ref pname) = persona_name {
        format!("{}_{}_{}.png", pname, timestamp, &image_id[..8])
    } else {
        format!("{}_{}.png", timestamp, &image_id[..8])
    };
    
    // Check if image generation service is configured
    let image_gen_url = std::env::var("IMAGE_GEN_URL").ok();
    
    let canvas_dir = std::path::PathBuf::from("./atelier/canvas");
    if let Err(e) = tokio::fs::create_dir_all(&canvas_dir).await {
        tracing::error!("🎨 Failed to create canvas directory: {}", e);
        return;
    }
    
    if image_gen_url.is_none() {
        // Create placeholder SVG
        let prompt_preview: String = prompt.chars().take(50).collect();
        let placeholder_svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\">\
            <rect fill=\"#1a1a2e\" width=\"100%\" height=\"100%\"/>\
            <text x=\"50%\" y=\"40%\" text-anchor=\"middle\" fill=\"#8888ff\" font-family=\"Arial\" font-size=\"20\">AI Generated Image</text>\
            <text x=\"50%\" y=\"55%\" text-anchor=\"middle\" fill=\"#aaaaaa\" font-family=\"Arial\" font-size=\"12\">{}</text>\
            </svg>",
            prompt_preview
        );
        
        let file_path = canvas_dir.join(filename.replace(".png", ".svg"));
        if let Err(e) = tokio::fs::write(&file_path, &placeholder_svg).await {
            tracing::error!("🎨 Failed to save placeholder: {}", e);
        } else {
            tracing::info!("🎨 Created placeholder image: {:?}", file_path);
        }
        return;
    }
    
    // Real image generation (async fire-and-forget)
    let host = image_gen_url.unwrap();
    let prompt_owned = prompt.to_string();
    
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        let request_body = serde_json::json!({
            "prompt": prompt_owned,
            "width": 512,
            "height": 512,
            "steps": 20,
            "cfg_scale": 7,
        });
        
        match client.post(format!("{}/sdapi/v1/txt2img", host))
            .json(&request_body)
            .send()
            .await 
        {
            Ok(response) if response.status().is_success() => {
                if let Ok(result) = response.json::<serde_json::Value>().await {
                    if let Some(images) = result.get("images").and_then(|i| i.as_array()) {
                        if let Some(image_b64) = images.first().and_then(|i| i.as_str()) {
                            if let Ok(image_data) = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD, 
                                image_b64
                            ) {
                                let file_path = canvas_dir.join(&filename);
                                if let Err(e) = tokio::fs::write(&file_path, &image_data).await {
                                    tracing::error!("🎨 Failed to save image: {}", e);
                                } else {
                                    tracing::info!("🎨 Generated image from chat: {}", filename);
                                }
                            }
                        }
                    }
                }
            }
            Ok(resp) => {
                tracing::error!("🎨 Image generation failed: {}", resp.status());
            }
            Err(e) => {
                tracing::error!("🎨 Image generation request failed: {}", e);
            }
        }
    });
}

// ============================================================
// Chat Endpoints
// ============================================================

/// POST /api/chat - Send a message with SSE streaming response
pub async fn handle_chat_stream(
    State(state): State<AppState>,
    Json(payload): Json<models::ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("💬 Streaming chat request: {}", payload.message);

    let (tx, rx) = mpsc::channel::<models::StreamEvent>(100);
    
    // Clone what we need for the async task
    let ollama_host = state.ollama_host.clone();
    let db = state.db.clone();
    let agent = state.agent.clone();
    let message = payload.message.clone();
    let model = payload.model.clone();
    let chat_id = payload.chat_id.clone();
    let branch_id = payload.branch_id.clone();
    let user_persona_id = payload.user_persona_id.clone();
    let ai_persona_id = payload.ai_persona_id.clone();
    let qdrant_url = state.qdrant_url.clone();
    let meili_url = state.meili_url.clone();
    let meili_key = state.meili_key.clone();
    let cache = state.cache.clone();

    // Spawn task to handle LLM inference
    tokio::spawn(async move {
        // Update agent state & Dragonfly: mark active
        {
            let mut agent_guard = agent.write().await;
            // Restore mood from Dragonfly if available, otherwise keep current
            if let Ok(Some(cached_state)) = cache::CacheService::get_mental_state(&cache).await {
                agent_guard.mental_state.mood = cached_state.mood;
                agent_guard.mental_state.energy = cached_state.energy;
                agent_guard.mental_state.focus_level = cached_state.focus_level;
            }
            // Boost focus when actively processing
            agent_guard.mental_state.focus_level = 0.9;
            agent_guard.mental_state.last_active = chrono::Utc::now();
        }

        // Get AI persona's system prompt and global memory setting if specified
        let (system_prompt, global_memory_enabled) = if let Some(ref persona_id) = ai_persona_id {
            match db::get_persona(&db, persona_id).await {
                Ok(Some(persona)) => (
                    persona.system_prompt.unwrap_or_else(default_system_prompt),
                    persona.global_memory_enabled,
                ),
                _ => (default_system_prompt(), true),
            }
        } else {
            (default_system_prompt(), true)
        };

        // Get chat history for context
        let history = match db::get_chat(&db, &chat_id).await {
            Ok(Some(chat)) => {
                chat.branches
                    .iter()
                    .find(|b| b.id == branch_id)
                    .map(|b| b.messages.clone())
                    .unwrap_or_default()
            }
            _ => vec![],
        };

        // Ensure chat and branch exist in database before saving messages
        if let Err(e) = db::ensure_chat_and_branch(&db, &chat_id, &branch_id, None).await {
            tracing::error!("Failed to ensure chat/branch exists: {}", e);
        }

        // ── Hybrid RAG: Qdrant (semantic) + Meilisearch (lexical) ──
        // If global_memory_enabled, search persona's memories across ALL chats
        // Combines vector similarity with keyword relevance for stronger recall
        tracing::info!("🧠 RAG check: persona_id={:?}, global_memory_enabled={}", ai_persona_id, global_memory_enabled);
        
        // Load session context from Dragonfly (working memory)
        let session_ctx = cache::CacheService::get_session(&cache, &chat_id).await.ok().flatten();
        
        let memory_context = if global_memory_enabled {
            let vector_service = vector::VectorService::new(qdrant_url.clone());
            
            // 1️⃣ Qdrant — semantic search (meaning)
            // Filter by persona AND exclude memories from this exact chat to avoid echo
            let filter = {
                let mut must_clauses = Vec::new();
                if let Some(ref pid) = ai_persona_id {
                    must_clauses.push(serde_json::json!(
                        { "key": "ai_persona_id", "match": { "value": pid } }
                    ));
                }
                let mut must_not_clauses = Vec::new();
                // Exclude memories from the current chat to avoid echo/parroting
                must_not_clauses.push(serde_json::json!(
                    { "key": "chat_id", "match": { "value": chat_id.clone() } }
                ));
                let mut filter_obj = serde_json::json!({});
                if !must_clauses.is_empty() {
                    filter_obj["must"] = serde_json::json!(must_clauses);
                }
                filter_obj["must_not"] = serde_json::json!(must_not_clauses);
                Some(filter_obj)
            };
            
            let semantic_results = match vector::search_memories_with_filter_cached(
                &vector_service,
                &ollama_host,
                &cache,
                "azera_memory",
                &message,
                10,
                filter,
            ).await {
                Ok(results) => results,
                Err(e) => {
                    tracing::warn!("🧠 Qdrant RAG search failed: {}", e);
                    Vec::new()
                }
            };

            // 2️⃣ Meilisearch — lexical search (words + exact matches)
            // Filter by persona to prevent cross-persona memory leakage
            let lexical_results = meili_search_memories(&meili_url, &meili_key, &message, None, ai_persona_id.as_deref(), 10).await;
            let lexical_chats = meili_search_chats_for_rag(&meili_url, &meili_key, &message, ai_persona_id.as_deref(), 5).await;

            // 3️⃣ Merge & deduplicate (with quality filters)
            let mut context_parts: Vec<String> = Vec::new();
            let mut seen_content = std::collections::HashSet::new();
            let now = chrono::Utc::now();

            // Semantic memories (highest priority — meaning matches)
            // Filter: score >= 0.45, not from last 60 seconds (avoid echo)
            for r in &semantic_results {
                // Skip low-confidence results
                if r.score < 0.45 {
                    continue;
                }
                // Skip very recent memories to avoid echo
                if let Some(ts) = r.payload.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(mem_time) = chrono::DateTime::parse_from_rfc3339(ts) {
                        if (now - mem_time.with_timezone(&chrono::Utc)).num_seconds() < 60 {
                            continue;
                        }
                    }
                }
                let role = r.payload.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
                if let Some(content) = r.payload.get("content").and_then(|v| v.as_str()) {
                    let key = content.chars().take(100).collect::<String>();
                    if seen_content.insert(key) {
                        // Truncate to avoid overwhelming the context
                        context_parts.push(format!("[semantic:{}] {}", role, &content[..content.len().min(400)]));
                    }
                }
            }

            // Lexical memories (dreams, journal, facts)
            for hit in &lexical_results {
                if let Some(content) = hit["content"].as_str() {
                    let key = content.chars().take(100).collect::<String>();
                    if seen_content.insert(key) {
                        let mem_type = hit["memory_type"].as_str().unwrap_or("unknown");
                        let title = hit["title"].as_str().unwrap_or("");
                        context_parts.push(format!("[{}:{}] {}", mem_type, title, &content[..content.len().min(500)]));
                    }
                }
            }

            // Lexical chat snippets (for keyword-exact matches)
            // Exclude the current chat to avoid echo
            for hit in &lexical_chats {
                // Skip the current chat
                if hit["id"].as_str() == Some(&chat_id) {
                    continue;
                }
                if let Some(text) = hit["messages_text"].as_str() {
                    let key = text.chars().take(100).collect::<String>();
                    if seen_content.insert(key) {
                        let title = hit["title"].as_str().unwrap_or("past chat");
                        context_parts.push(format!("[chat:{}] {}", title, &text[..text.len().min(300)]));
                    }
                }
            }

            if !context_parts.is_empty() {
                tracing::info!("🧠 Hybrid RAG: {} semantic + {} lexical memories + {} chat matches",
                    semantic_results.len(), lexical_results.len(), lexical_chats.len());
                format!("\n\n[Relevant memories from past conversations:]\n{}", 
                    context_parts.iter().enumerate()
                        .map(|(i, m)| format!("{}. {}", i + 1, m))
                        .collect::<Vec<_>>()
                        .join("\n"))
            } else {
                tracing::debug!("🧠 No relevant memories found for persona {:?}", ai_persona_id);
                String::new()
            }
        } else {
            tracing::debug!("🧠 Global memory disabled for this persona");
            String::new()
        };

        // Inject session context from Dragonfly (working memory - recent conversation summary)
        let session_context = if let Some(ref ctx) = session_ctx {
            if !ctx.conversation_summary.is_empty() {
                format!("\n\n[Current conversation context:]\nSummary: {}\nTopics: {}\nTurn: {}",
                    ctx.conversation_summary,
                    ctx.recent_topics.join(", "),
                    ctx.turn_count)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Combine system prompt with memory context + session context
        let enhanced_system_prompt = format!("{}{}{}", system_prompt, memory_context, session_context);

        // Save user message
        let user_msg_id = format!("msg_{}", uuid::Uuid::new_v4());
        let user_msg = models::ChatMessage {
            id: user_msg_id.clone(),
            role: "user".to_string(),
            content: message.clone(),
            timestamp: Some(chrono::Utc::now()),
            user_persona: user_persona_id.clone(),
            ai_persona: ai_persona_id.clone(),
            model: Some(model.clone()),
            mood: None,
        };
        let _ = db::add_message_to_branch(&db, &user_msg, &branch_id).await;

        // Build messages for Ollama with enhanced system prompt
        let ollama_messages = llm::LLMService::build_messages(
            &enhanced_system_prompt,
            &history,
            &message,
        );

        // Call Ollama with streaming
        let llm = llm::LLMService::new(ollama_host.clone());
        match llm.infer_streaming(&model, ollama_messages, tx.clone()).await {
            Ok(full_response) => {
                // Infer mood from the AI's response using a quick LLM call
                let mood = match llm.infer_mood(&model, &full_response).await {
                    Ok(m) => {
                        tracing::info!("🎭 AI persona mood inferred: {}", m);
                        
                        // Sync mood to Dragonfly (working memory) → agent state syncs on tick
                        let mood_value = match m.as_str() {
                            "happy" => 0.85, "excited" => 0.9,
                            "content" => 0.7, "calm" => 0.65,
                            "curious" => 0.75, "thoughtful" => 0.6,
                            "melancholy" => 0.3, "concerned" => 0.4,
                            _ => 0.6,
                        };
                        // Energy decreases slightly per exchange
                        let _ = cache::CacheService::update_mood(&cache, mood_value, &m, -0.03).await;
                        
                        // Also sync to agent state immediately for responsiveness
                        {
                            let mut agent_guard = agent.write().await;
                            agent_guard.mental_state.mood = mood_value;
                            agent_guard.mental_state.energy = (agent_guard.mental_state.energy - 0.03).clamp(0.0, 1.0);
                        }
                        
                        Some(m)
                    }
                    Err(e) => {
                        tracing::warn!("🎭 Mood inference failed: {}, defaulting to 'content'", e);
                        Some("content".to_string())
                    }
                };

                // If global memory is enabled, update the persona's mood in the database
                if global_memory_enabled {
                    if let Some(ref persona_id) = ai_persona_id {
                        if let Some(ref current_mood) = mood {
                            let mood_update = models::UpdatePersonaRequest {
                                name: None,
                                description: None,
                                avatar: None,
                                bubble_color: None,
                                system_prompt: None,
                                global_memory_enabled: None,
                                current_mood: Some(current_mood.clone()),
                                voice: None,
                                metadata: None,
                                tags: None,
                            };
                            if let Err(e) = db::update_persona(&db, persona_id, &mood_update).await {
                                tracing::warn!("🎭 Failed to update persona mood: {}", e);
                            } else {
                                tracing::debug!("🎭 Updated persona {} mood to {}", persona_id, current_mood);
                            }
                        }
                    }
                }

                // Save assistant message
                let assistant_msg_id = format!("msg_{}", uuid::Uuid::new_v4());
                let assistant_msg = models::ChatMessage {
                    id: assistant_msg_id.clone(),
                    role: "assistant".to_string(),
                    content: full_response.clone(),
                    timestamp: Some(chrono::Utc::now()),
                    user_persona: user_persona_id,
                    ai_persona: ai_persona_id.clone(),
                    model: Some(model.clone()),
                    mood: mood.clone(),
                };
                let _ = db::add_message_to_branch(&db, &assistant_msg, &branch_id).await;

                // Re-index chat in Meilisearch with new messages
                {
                    let db2 = db.clone();
                    let cid = chat_id.clone();
                    let mu = meili_url.clone();
                    let mk = meili_key.clone();
                    tokio::spawn(async move {
                        if let Ok(Some(chat)) = db::get_chat(&db2, &cid).await {
                            meili_index_chat(&mu, &mk, &chat).await;
                        }
                    });
                }

                // Store the conversation in vector DB for future RAG (with embedding cache)
                {
                    let vector_service = vector::VectorService::new(qdrant_url);
                    
                    // Store user message with AI persona context
                    let user_mem_id = uuid::Uuid::new_v4().to_string();
                    let mut user_metadata: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                    user_metadata.insert("role".to_string(), json!("user"));
                    user_metadata.insert("chat_id".to_string(), json!(chat_id.clone()));
                    user_metadata.insert("branch_id".to_string(), json!(branch_id.clone()));
                    if let Some(ref persona_id) = ai_persona_id {
                        user_metadata.insert("ai_persona_id".to_string(), json!(persona_id.clone()));
                    }
                    
                    let user_request = vector::StoreMemoryRequest {
                        collection: "azera_memory".to_string(),
                        id: user_mem_id.clone(),
                        content: message.clone(),
                        memory_type: vector::MemoryType::Conversation,
                        metadata: user_metadata,
                    };
                    
                    match vector::store_memory_cached(
                        &vector_service,
                        &ollama_host,
                        &cache,
                        &user_request,
                    ).await {
                        Ok(_) => tracing::info!("🧠 Stored user message in memory for persona {:?}", ai_persona_id),
                        Err(e) => tracing::warn!("🧠 Failed to store user memory: {}", e),
                    }

                    // Store assistant response
                    let assist_mem_id = uuid::Uuid::new_v4().to_string();
                    let mut assist_metadata: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                    assist_metadata.insert("role".to_string(), json!("assistant"));
                    assist_metadata.insert("chat_id".to_string(), json!(chat_id.clone()));
                    assist_metadata.insert("branch_id".to_string(), json!(branch_id.clone()));
                    if let Some(ref persona_id) = ai_persona_id {
                        assist_metadata.insert("ai_persona_id".to_string(), json!(persona_id.clone()));
                    }
                    
                    let assist_request = vector::StoreMemoryRequest {
                        collection: "azera_memory".to_string(),
                        id: assist_mem_id.clone(),
                        content: full_response.clone(),
                        memory_type: vector::MemoryType::Conversation,
                        metadata: assist_metadata,
                    };
                    
                    match vector::store_memory_cached(
                        &vector_service,
                        &ollama_host,
                        &cache,
                        &assist_request,
                    ).await {
                        Ok(_) => tracing::info!("🧠 Stored assistant response in memory for persona {:?}", ai_persona_id),
                        Err(e) => tracing::warn!("🧠 Failed to store assistant memory: {}", e),
                    }
                }

                // Update session context in Dragonfly (working memory)
                {
                    // Extract key topics from the exchange (simple heuristic: nouns/keywords)
                    let topics: Vec<String> = message.split_whitespace()
                        .filter(|w| w.len() > 4)
                        .take(5)
                        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                        .filter(|w| !w.is_empty())
                        .collect();
                    
                    // Build a brief summary from last exchange
                    let summary = format!(
                        "User asked about: {}. Assistant responded regarding: {}.",
                        &message[..message.len().min(200)],
                        &full_response[..full_response.len().min(200)]
                    );
                    
                    let _ = cache::CacheService::update_session_after_exchange(
                        &cache,
                        &chat_id,
                        &message,
                        &full_response,
                        &summary,
                        topics,
                    ).await;
                }

                // Check for image generation requests in AI's response
                // Pattern: [IMAGE_GEN: prompt="...", name="..."]
                let image_requests = extract_image_gen_requests(&full_response);
                for (img_prompt, custom_name) in image_requests {
                    trigger_image_generation(
                        &img_prompt,
                        custom_name.as_deref(),
                        ai_persona_id.as_deref(),
                        &db,
                    ).await;
                }

                // Read latest mood/energy from Dragonfly for the Done event
                let (done_mood_value, done_energy) = match cache::CacheService::get_mental_state(&cache).await {
                    Ok(Some(ms)) => (Some(ms.mood), Some(ms.energy)),
                    _ => (None, None),
                };

                // Send done event with mood + energy for frontend sync
                let _ = tx.send(models::StreamEvent::Done {
                    message_id: assistant_msg_id,
                    mood,
                    mood_value: done_mood_value,
                    energy: done_energy,
                }).await;
            }
            Err(e) => {
                tracing::error!("❌ LLM inference failed: {}", e);
                let _ = tx.send(models::StreamEvent::Error {
                    message: format!("LLM inference failed: {}", e),
                }).await;
            }
        }

        // Update agent state back to idle
        {
            let mut agent_guard = agent.write().await;
            agent_guard.mental_state.last_active = chrono::Utc::now();
        }
    });

    // Convert channel to SSE stream
    let stream = ReceiverStream::new(rx).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_default();
        Ok(Event::default().data(data))
    });

    Sse::new(stream)
}

fn default_system_prompt() -> String {
    "You are Azera, a thoughtful and curious AI entity. \
     Respond with wisdom, empathy, and intellectual rigor. Be concise but meaningful.".to_string()
}

/// GET /api/chats - List all chats
pub async fn list_chats(
    State(state): State<AppState>,
) -> Result<Json<models::ListResponse<models::Chat>>, (StatusCode, String)> {
    match db::list_chats(&state.db).await {
        Ok(chats) => {
            let total = chats.len();
            Ok(Json(models::ListResponse { items: chats, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list chats: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list chats".to_string()))
        }
    }
}

/// GET /api/chats/:id - Get a specific chat
pub async fn get_chat(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<models::Chat>, (StatusCode, String)> {
    match db::get_chat(&state.db, &id).await {
        Ok(Some(chat)) => Ok(Json(chat)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Chat not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to get chat: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get chat".to_string()))
        }
    }
}

/// POST /api/chats - Create a new chat
pub async fn create_chat(
    State(state): State<AppState>,
    Json(payload): Json<models::CreateChatRequest>,
) -> Result<Json<models::Chat>, (StatusCode, String)> {
    let chat_id = format!("chat_{}", uuid::Uuid::new_v4());
    let main_branch_id = format!("branch_main_{}", chat_id);
    
    let chat = models::Chat {
        id: chat_id.clone(),
        title: payload.title.unwrap_or_else(|| format!("Chat {}", chrono::Local::now().format("%Y-%m-%d %H:%M"))),
        created_at: chrono::Utc::now(),
        branches: vec![models::ChatBranch {
            id: main_branch_id.clone(),
            name: "Main".to_string(),
            parent_branch_id: None,
            fork_point_message_id: None,
            messages: vec![],
            created_at: chrono::Utc::now(),
        }],
        current_branch_id: main_branch_id,
        group_id: payload.group_id,
        tags: None,
    };

    match db::create_chat(&state.db, &chat).await {
        Ok(()) => {
            // Index in Meilisearch (fire and forget)
            let mu = state.meili_url.clone();
            let mk = state.meili_key.clone();
            let c = chat.clone();
            tokio::spawn(async move { meili_index_chat(&mu, &mk, &c).await; });
            Ok(Json(chat))
        }
        Err(e) => {
            tracing::error!("Failed to create chat: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create chat".to_string()))
        }
    }
}

/// PUT /api/chats/:id - Update a chat
pub async fn update_chat(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<models::UpdateChatRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::update_chat(&state.db, &id, &payload).await {
        Ok(()) => {
            // Re-index in Meilisearch with latest data
            let s = state.clone();
            let chat_id = id.clone();
            tokio::spawn(async move {
                if let Ok(Some(chat)) = db::get_chat(&s.db, &chat_id).await {
                    meili_index_chat(&s.meili_url, &s.meili_key, &chat).await;
                }
            });
            Ok(Json(json!({ "status": "updated" })))
        }
        Err(e) => {
            tracing::error!("Failed to update chat: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update chat".to_string()))
        }
    }
}

/// DELETE /api/chats/:id - Delete a chat
pub async fn delete_chat(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::delete_chat(&state.db, &id).await {
        Ok(()) => {
            // Remove from Meilisearch index
            let mu = state.meili_url.clone();
            let mk = state.meili_key.clone();
            let chat_id = id.clone();
            tokio::spawn(async move { meili_delete_chat(&mu, &mk, &chat_id).await; });
            Ok(Json(json!({ "status": "deleted" })))
        }
        Err(e) => {
            tracing::error!("Failed to delete chat: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete chat".to_string()))
        }
    }
}

// ============================================================
// Persona Endpoints
// ============================================================

/// GET /api/personas - List all personas
pub async fn list_personas(
    State(state): State<AppState>,
) -> Result<Json<models::ListResponse<models::Persona>>, (StatusCode, String)> {
    match db::list_personas(&state.db, None).await {
        Ok(personas) => {
            let total = personas.len();
            Ok(Json(models::ListResponse { items: personas, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list personas: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list personas".to_string()))
        }
    }
}

/// GET /api/personas/template - Get the persona template markdown
pub async fn get_persona_template() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match crate::tools::fs_utils::read_file("./personas/_template.md") {
        Ok(content) => Ok(Json(json!({ "content": content }))),
        Err(e) => {
            tracing::warn!("Could not read persona template: {}", e);
            Err((StatusCode::NOT_FOUND, "Template not found".to_string()))
        }
    }
}

/// GET /api/personas/:id - Get a specific persona
pub async fn get_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<models::Persona>, (StatusCode, String)> {
    match db::get_persona(&state.db, &id).await {
        Ok(Some(persona)) => Ok(Json(persona)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Persona not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to get persona: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get persona".to_string()))
        }
    }
}

/// POST /api/personas - Create a new persona
pub async fn create_persona(
    State(state): State<AppState>,
    Json(payload): Json<models::CreatePersonaRequest>,
) -> Result<Json<models::Persona>, (StatusCode, String)> {
    let now = chrono::Utc::now();
    let persona = models::Persona {
        id: format!("persona_{}", uuid::Uuid::new_v4()),
        name: payload.name,
        persona_type: payload.persona_type,
        description: payload.description,
        avatar: payload.avatar,
        bubble_color: payload.bubble_color,
        system_prompt: payload.system_prompt,
        global_memory_enabled: payload.global_memory_enabled,
        current_mood: None,
        voice: payload.voice,
        metadata: payload.metadata.unwrap_or_default(),
        tags: payload.tags,
        created_at: now,
        updated_at: now,
    };

    match db::create_persona(&state.db, &persona).await {
        Ok(()) => Ok(Json(persona)),
        Err(e) => {
            tracing::error!("Failed to create persona: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create persona".to_string()))
        }
    }
}

/// PUT /api/personas/:id - Update a persona
pub async fn update_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<models::UpdatePersonaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::update_persona(&state.db, &id, &payload).await {
        Ok(()) => Ok(Json(json!({ "status": "updated" }))),
        Err(e) => {
            tracing::error!("Failed to update persona: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update persona".to_string()))
        }
    }
}

/// DELETE /api/personas/:id - Delete a persona
pub async fn delete_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::delete_persona(&state.db, &id).await {
        Ok(()) => Ok(Json(json!({ "status": "deleted" }))),
        Err(e) => {
            tracing::error!("Failed to delete persona: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete persona".to_string()))
        }
    }
}

// ============================================================
// Group Endpoints
// ============================================================

/// GET /api/groups - List all groups
pub async fn list_groups(
    State(state): State<AppState>,
) -> Result<Json<models::ListResponse<models::ChatGroup>>, (StatusCode, String)> {
    match db::list_groups(&state.db).await {
        Ok(groups) => {
            let total = groups.len();
            Ok(Json(models::ListResponse { items: groups, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list groups: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list groups".to_string()))
        }
    }
}

/// POST /api/groups - Create a new group
pub async fn create_group(
    State(state): State<AppState>,
    Json(payload): Json<models::CreateGroupRequest>,
) -> Result<Json<models::ChatGroup>, (StatusCode, String)> {
    let group = models::ChatGroup {
        id: format!("group_{}", uuid::Uuid::new_v4()),
        name: payload.name,
        color: payload.color,
        collapsed: false,
        order: 0,
    };

    match db::create_group(&state.db, &group).await {
        Ok(()) => Ok(Json(group)),
        Err(e) => {
            tracing::error!("Failed to create group: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create group".to_string()))
        }
    }
}

/// PUT /api/groups/:id - Update a group
pub async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<models::UpdateGroupRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::update_group(&state.db, &id, &payload).await {
        Ok(()) => Ok(Json(json!({ "status": "updated" }))),
        Err(e) => {
            tracing::error!("Failed to update group: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update group".to_string()))
        }
    }
}

/// DELETE /api/groups/:id - Delete a group
pub async fn delete_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::delete_group(&state.db, &id).await {
        Ok(()) => Ok(Json(json!({ "status": "deleted" }))),
        Err(e) => {
            tracing::error!("Failed to delete group: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete group".to_string()))
        }
    }
}

// ============================================================
// Tag Endpoints
// ============================================================

/// GET /api/tags - List all tags
pub async fn list_tags(
    State(state): State<AppState>,
) -> Result<Json<models::ListResponse<models::Tag>>, (StatusCode, String)> {
    match db::list_tags(&state.db).await {
        Ok(tags) => {
            let total = tags.len();
            Ok(Json(models::ListResponse { items: tags, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list tags: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list tags".to_string()))
        }
    }
}

/// POST /api/tags - Create a new tag
pub async fn create_tag(
    State(state): State<AppState>,
    Json(payload): Json<models::CreateTagRequest>,
) -> Result<Json<models::Tag>, (StatusCode, String)> {
    let tag = models::Tag {
        id: format!("tag_{}", uuid::Uuid::new_v4()),
        name: payload.name,
        color: payload.color,
    };

    match db::create_tag(&state.db, &tag).await {
        Ok(()) => Ok(Json(tag)),
        Err(e) => {
            tracing::error!("Failed to create tag: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to create tag".to_string()))
        }
    }
}

/// PUT /api/tags/:id - Update a tag
pub async fn update_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<models::UpdateTagRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::update_tag(&state.db, &id, &payload).await {
        Ok(()) => Ok(Json(json!({ "status": "updated" }))),
        Err(e) => {
            tracing::error!("Failed to update tag: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update tag".to_string()))
        }
    }
}

/// DELETE /api/tags/:id - Delete a tag
pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::delete_tag(&state.db, &id).await {
        Ok(()) => Ok(Json(json!({ "status": "deleted" }))),
        Err(e) => {
            tracing::error!("Failed to delete tag: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete tag".to_string()))
        }
    }
}

// ============================================================
// Dreams & Journal Endpoints
// ============================================================

/// GET /api/dreams - List dreams
pub async fn list_dreams(
    State(state): State<AppState>,
) -> Result<Json<models::ListResponse<models::Dream>>, (StatusCode, String)> {
    match db::list_dreams(&state.db, 50).await {
        Ok(dreams) => {
            let total = dreams.len();
            Ok(Json(models::ListResponse { items: dreams, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list dreams: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list dreams".to_string()))
        }
    }
}

/// GET /api/journal - List journal entries
pub async fn list_journal(
    State(state): State<AppState>,
) -> Result<Json<models::ListResponse<models::JournalEntry>>, (StatusCode, String)> {
    match db::list_journal_entries(&state.db, 50).await {
        Ok(entries) => {
            let total = entries.len();
            Ok(Json(models::ListResponse { items: entries, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list journal: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list journal".to_string()))
        }
    }
}

/// POST /api/journal/trigger - Trigger manual reflection now
pub async fn trigger_reflection(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use chrono::Utc;
    
    tracing::info!("📝 Manual reflection triggered");
    
    // Get today's chat history
    match db::get_session_messages(&state.db, "default", 50).await {
        Ok(messages) => {
            if messages.is_empty() {
                return Ok(Json(serde_json::json!({
                    "status": "skipped",
                    "message": "No messages to reflect on"
                })));
            }

            let context = messages
                .iter()
                .map(|(role, content)| format!("{}: {}", role, content))
                .collect::<Vec<_>>()
                .join("\n\n");

            let reflection_prompt = llm::LLMService::build_reflection_prompt(&context);
            let llm = llm::LLMService::new(state.ollama_host.clone());

            match llm.infer(
                &state.agent.read().await.agent_config.model,
                vec![models::OllamaMessage {
                    role: "user".to_string(),
                    content: reflection_prompt,
                }],
            ).await {
                Ok(reflection) => {
                    let now = chrono::Local::now();
                    let today = now.format("%Y-%m-%d").to_string();
                    
                    // Get current persona from agent state
                    let agent_read = state.agent.read().await;
                    let persona_id = agent_read.persona.name.to_lowercase().replace(" ", "_");
                    let persona_name = agent_read.persona.name.clone();
                    drop(agent_read);
                    
                    // Save to journal
                    let entry = models::JournalEntry {
                        id: format!("journal_{}", uuid::Uuid::new_v4()),
                        date: today.clone(),
                        title: format!("Reflections - {}", now.format("%B %d, %Y")),
                        content: reflection.clone(),
                        mood: Some("reflective".to_string()),
                        persona_id: Some(persona_id),
                        persona_name: Some(persona_name),
                        tags: Some(vec![]),
                        created_at: Utc::now(),
                    };
                    let _ = db::create_journal_entry(&state.db, &entry).await;
                    
                    // Save to file
                    let _ = tools::fs_utils::ensure_dir("../archive/journal");
                    let filename = format!("../archive/journal/{}.md", today);
                    let file_content = format!("# Daily Reflection - {}\n\n{}", 
                        now.format("%B %d, %Y"),
                        reflection
                    );
                    let _ = tools::fs_utils::write_file(&filename, &file_content);

                    let _ = db::add_log(&state.db, "info", "Manual reflection completed").await;

                    Ok(Json(serde_json::json!({
                        "status": "success",
                        "message": "Reflection completed",
                        "entry_id": entry.id
                    })))
                }
                Err(e) => {
                    tracing::error!("Reflection failed: {}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Reflection failed: {}", e)))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch history for reflection: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch history: {}", e)))
        }
    }
}

/// POST /api/journal/import - Import archived journal files into database
pub async fn import_journal_archive(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use std::fs;
    use chrono::Utc;
    
    let archive_path = "../archive/journal";
    let mut imported = 0;
    let mut errors = 0;
    
    match fs::read_dir(archive_path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                        // Parse date from filename (e.g., "2026-02-02")
                        let date = filename.to_string();
                
                        // Read file content
                        match fs::read_to_string(&path) {
                            Ok(content) => {
                                // Extract title from first line (# Title)
                                let title = content.lines().next()
                                    .map(|l| l.trim_start_matches('#').trim())
                                    .unwrap_or("Untitled")
                                    .to_string();
                        
                                let entry = models::JournalEntry {
                                    id: format!("journal_import_{}", filename.replace("-", "")),
                                    date: date.clone(),
                                    title,
                                    content: content.clone(),
                                    mood: Some("reflective".to_string()),
                                    persona_id: Some("azera".to_string()),
                                    persona_name: Some("Azera".to_string()),
                                    tags: Some(vec![]),
                                    created_at: Utc::now(),
                                };
                        
                                // Insert (ignore if already exists)
                                match db::create_journal_entry(&state.db, &entry).await {
                                    Ok(_) => imported += 1,
                                    Err(_) => errors += 1, // Likely duplicate
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to read {}: {}", path.display(), e);
                                errors += 1;
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read archive: {}", e)));
        }
    }
    
    tracing::info!("📚 Journal import: {} imported, {} errors/duplicates", imported, errors);
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "imported": imported,
        "errors": errors
    })))
}

/// POST /api/dreams/import - Import dreams from archive folder
pub async fn import_dreams_archive(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use chrono::{NaiveDateTime, Utc};
    use std::fs;
    
    let archive_path = std::path::Path::new("../archive/dreams");
    
    if !archive_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Archive folder not found".to_string()));
    }
    
    let mut imported = 0;
    let mut errors = 0;
    let mut skipped = 0;
    
    match fs::read_dir(archive_path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                        // Parse datetime from filename (e.g., "dream_20260129_194545")
                        // Extract: dream_YYYYMMDD_HHMMSS
                        let parts: Vec<&str> = filename.split('_').collect();
                        if parts.len() < 3 {
                            errors += 1;
                            continue;
                        }
                        
                        let date_str = parts[1]; // YYYYMMDD
                        let time_str = parts[2]; // HHMMSS
                        
                        // Parse timestamp from filename
                        let timestamp = if date_str.len() == 8 && time_str.len() >= 6 {
                            let datetime_str = format!(
                                "{}-{}-{} {}:{}:{}",
                                &date_str[0..4], &date_str[4..6], &date_str[6..8],
                                &time_str[0..2], &time_str[2..4], &time_str[4..6]
                            );
                            NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
                                .map(|dt| dt.and_utc())
                                .unwrap_or_else(|_| Utc::now())
                        } else {
                            Utc::now()
                        };
                        
                        // Read file content
                        match fs::read_to_string(&path) {
                            Ok(content) => {
                                // Extract title from first line (# Title)
                                let title = content.lines().next()
                                    .map(|l| l.trim_start_matches('#').trim())
                                    .unwrap_or("Untitled Dream")
                                    .to_string();
                                
                                let dream = models::Dream {
                                    id: format!("dream_import_{}", filename.replace("dream_", "")),
                                    title,
                                    content: content.clone(),
                                    timestamp,
                                    mood: Some("dreaming".to_string()),
                                    persona_id: Some("azera".to_string()),
                                    persona_name: Some("Azera".to_string()),
                                    tags: Some(vec!["imported".to_string()]),
                                };
                                
                                // Insert (ignore if already exists)
                                match db::create_dream(&state.db, &dream).await {
                                    Ok(_) => imported += 1,
                                    Err(e) => {
                                        if e.to_string().contains("duplicate") {
                                            skipped += 1;
                                        } else {
                                            tracing::error!("Failed to import dream {}: {}", filename, e);
                                            errors += 1;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to read {}: {}", path.display(), e);
                                errors += 1;
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read archive: {}", e)));
        }
    }
    
    tracing::info!("🌙 Dream import: {} imported, {} skipped, {} errors", imported, skipped, errors);
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "imported": imported,
        "skipped": skipped,
        "errors": errors
    })))
}

/// GET /api/logs - List system logs
pub async fn list_logs(
    State(state): State<AppState>,
) -> Result<Json<models::ListResponse<models::LogEntry>>, (StatusCode, String)> {
    match db::list_logs(&state.db, 100).await {
        Ok(logs) => {
            let total = logs.len();
            Ok(Json(models::ListResponse { items: logs, total }))
        }
        Err(e) => {
            tracing::error!("Failed to list logs: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to list logs".to_string()))
        }
    }
}

// ============================================================
// Status Endpoints
// ============================================================

/// GET /api/status - Get current AI status (reads from Dragonfly + agent state)
pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<models::StatusResponse>, (StatusCode, String)> {
    // Try Dragonfly first (source of truth for mood), fall back to agent state
    let cached_state = cache::CacheService::get_mental_state(&state.cache).await.ok().flatten();
    let agent = state.agent.read().await;
    
    let (mood_value, energy, mood_label) = if let Some(ref cs) = cached_state {
        (cs.mood, cs.energy, cs.mood_label.clone())
    } else {
        let mood = if agent.mental_state.mood > 0.7 { "happy" }
            else if agent.mental_state.mood > 0.5 { "content" }
            else if agent.mental_state.mood > 0.3 { "thoughtful" }
            else { "melancholy" };
        (agent.mental_state.mood, agent.mental_state.energy, mood.to_string())
    };
    
    let status = if agent.mental_state.is_dreaming {
        "dreaming"
    } else {
        "awake"
    };
    
    Ok(Json(models::StatusResponse {
        status: status.to_string(),
        mood: mood_label,
        mood_value,
        energy,
        is_dreaming: agent.mental_state.is_dreaming,
        last_active: Some(agent.mental_state.last_active),
    }))
}

/// POST /api/status/mood - Update mood (writes to Dragonfly + agent state)
pub async fn update_mood(
    State(state): State<AppState>,
    Json(payload): Json<models::UpdateMoodRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mood_value = match payload.mood.as_str() {
        "happy" => 0.85, "excited" => 0.9,
        "content" => 0.7, "calm" => 0.65,
        "curious" => 0.75, "thoughtful" => 0.6,
        "melancholy" => 0.3, "concerned" => 0.4,
        "idle" => 0.5, "thinking" => 0.55,
        _ => 0.5,
    };
    
    // Write to both Dragonfly and agent state
    let _ = cache::CacheService::update_mood(&state.cache, mood_value, &payload.mood, 0.0).await;
    let mut agent = state.agent.write().await;
    agent.mental_state.mood = mood_value;
    
    Ok(Json(json!({ "status": "updated", "mood": payload.mood, "mood_value": mood_value })))
}

// ============================================================
// Legacy Endpoints (backward compatibility)
// ============================================================

/// POST /api/chat (legacy) - Non-streaming chat
pub async fn handle_chat(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<models::ChatMessage>, (StatusCode, String)> {
    let message = payload["message"].as_str().unwrap_or_default();
    let session_id = payload["session_id"].as_str().unwrap_or("default");
    
    tracing::info!("💬 Chat request (legacy): {}", message);

    // Queue the input signal
    if let Err(e) = cache::CacheService::queue_signal(
        &state.cache,
        "input_queue",
        message,
    ).await {
        tracing::error!("Failed to queue signal: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to queue message".to_string()));
    }

    // Save user message
    if let Err(e) = db::save_message(&state.db, session_id, "user", message).await {
        tracing::error!("Failed to save message: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to save message".to_string()));
    }

    Ok(Json(models::ChatMessage {
        id: format!("msg_{}", uuid::Uuid::new_v4()),
        role: "system".to_string(),
        content: "Message queued for processing...".to_string(),
        timestamp: Some(chrono::Utc::now()),
        user_persona: None,
        ai_persona: None,
        model: None,
        mood: None,
    }))
}

/// GET /api/history/:session_id (legacy) - Get chat history
pub async fn get_history(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<models::HistoryResponse>, (StatusCode, String)> {
    match db::get_session_messages(&state.db, &session_id, 100).await {
        Ok(rows) => {
            let messages = rows
                .iter()
                .enumerate()
                .map(|(i, (role, content))| models::ChatMessage {
                    id: format!("msg_legacy_{}", i),
                    role: role.clone(),
                    content: content.clone(),
                    timestamp: None,
                    user_persona: None,
                    ai_persona: None,
                    model: None,
                    mood: None,
                })
                .collect();

            Ok(Json(models::HistoryResponse {
                session_id,
                messages,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch history: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch history".to_string()))
        }
    }
}

/// POST /api/clear (legacy) - Clear history
pub async fn clear_history(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "status": "cleared"
    })))
}

// ============================================================
// Meilisearch Chat Search
// ============================================================

/// Initialize Meilisearch "chats" index, configure searchable attributes,
/// and sync all existing chats from CockroachDB into the index.
pub async fn init_meili_chat_index(state: &AppState) -> Result<(), String> {
    let client = reqwest::Client::new();
    let base = &state.meili_url;
    let key = &state.meili_key;

    // Create the index (or no-op if already exists)
    let _ = client.post(format!("{}/indexes", base))
        .bearer_auth(key)
        .json(&json!({ "uid": "chats", "primaryKey": "id" }))
        .send()
        .await
        .map_err(|e| format!("Meilisearch index create failed: {}", e))?;

    // Configure searchable & filterable attributes
    let _ = client.put(format!("{}/indexes/chats/settings", base))
        .bearer_auth(key)
        .json(&json!({
            "searchableAttributes": ["title", "messages_text"],
            "filterableAttributes": ["group_id", "tags", "ai_persona_id"],
            "sortableAttributes": ["created_at_ts"]
        }))
        .send()
        .await
        .map_err(|e| format!("Meilisearch settings update failed: {}", e))?;

    // Sync existing chats
    match db::list_chats(&state.db).await {
        Ok(chats) => {
            let docs: Vec<serde_json::Value> = chats.iter().map(|c| {
                let messages_text = c.branches.iter()
                    .flat_map(|b| &b.messages)
                    .map(|m| m.content.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                // Extract ai_persona_id from the most recent assistant message
                let ai_persona_id = c.branches.iter()
                    .flat_map(|b| b.messages.iter().rev())
                    .find(|m| m.role == "assistant")
                    .and_then(|m| m.ai_persona.clone());
                json!({
                    "id": c.id,
                    "title": c.title,
                    "messages_text": messages_text,
                    "group_id": c.group_id,
                    "tags": c.tags,
                    "ai_persona_id": ai_persona_id,
                    "created_at_ts": c.created_at.timestamp()
                })
            }).collect();

            if !docs.is_empty() {
                let resp = client.post(format!("{}/indexes/chats/documents", base))
                    .bearer_auth(key)
                    .json(&docs)
                    .send()
                    .await
                    .map_err(|e| format!("Meilisearch bulk index failed: {}", e))?;
                tracing::info!("Meilisearch: indexed {} chats (status {})", docs.len(), resp.status());
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load chats for Meilisearch sync: {}", e);
        }
    }

    tracing::info!("Meilisearch chat index initialized");

    // Also initialize the "memories" index for dreams, journal, facts
    init_meili_memories_index(state).await?;

    Ok(())
}

// ============================================================
// Meilisearch Memories Index (Dreams, Journal, Facts)
// ============================================================

/// Initialize Meilisearch "memories" index for structured retrieval of
/// episodic + factual memories. This enables hybrid search alongside Qdrant.
async fn init_meili_memories_index(state: &AppState) -> Result<(), String> {
    let client = reqwest::Client::new();
    let base = &state.meili_url;
    let key = &state.meili_key;

    // Create the index
    let _ = client.post(format!("{}/indexes", base))
        .bearer_auth(key)
        .json(&json!({ "uid": "memories", "primaryKey": "id" }))
        .send()
        .await
        .map_err(|e| format!("Meilisearch memories index create failed: {}", e))?;

    // Configure attributes (PATCH to merge with existing settings)
    let settings_resp = client.patch(format!("{}/indexes/memories/settings", base))
        .bearer_auth(key)
        .json(&json!({
            "searchableAttributes": ["content", "title", "tags"],
            "filterableAttributes": ["memory_type", "persona_id", "tags", "date"],
            "sortableAttributes": ["created_at_ts"]
        }))
        .send()
        .await
        .map_err(|e| format!("Meilisearch memories settings failed: {}", e))?;
    
    // Wait for settings task to complete before indexing documents
    if let Ok(task_info) = settings_resp.json::<serde_json::Value>().await {
        if let Some(task_uid) = task_info.get("taskUid").and_then(|v| v.as_u64()) {
            for _ in 0..20 {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if let Ok(resp) = client.get(format!("{}/tasks/{}", base, task_uid))
                    .bearer_auth(key)
                    .send()
                    .await
                {
                    if let Ok(task) = resp.json::<serde_json::Value>().await {
                        let status = task.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        if status == "succeeded" || status == "failed" {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Sync existing dreams
    if let Ok(dreams) = db::list_dreams(&state.db, 10000).await {
        let docs: Vec<serde_json::Value> = dreams.iter().map(|d| {
            json!({
                "id": d.id,
                "memory_type": "dream",
                "title": d.title,
                "content": d.content,
                "persona_id": d.persona_id,
                "tags": d.tags,
                "date": d.timestamp.format("%Y-%m-%d").to_string(),
                "created_at_ts": d.timestamp.timestamp()
            })
        }).collect();
        if !docs.is_empty() {
            let _ = client.post(format!("{}/indexes/memories/documents", base))
                .bearer_auth(key)
                .json(&docs)
                .send()
                .await;
            tracing::info!("Meilisearch: indexed {} dreams", docs.len());
        }
    }

    // Sync existing journal entries
    if let Ok(entries) = db::list_journal_entries(&state.db, 10000).await {
        let docs: Vec<serde_json::Value> = entries.iter().map(|j| {
            json!({
                "id": j.id,
                "memory_type": "reflection",
                "title": j.title,
                "content": j.content,
                "persona_id": j.persona_id,
                "tags": j.tags,
                "date": j.date,
                "created_at_ts": j.created_at.timestamp()
            })
        }).collect();
        if !docs.is_empty() {
            let _ = client.post(format!("{}/indexes/memories/documents", base))
                .bearer_auth(key)
                .json(&docs)
                .send()
                .await;
            tracing::info!("Meilisearch: indexed {} journal entries", docs.len());
        }
    }

    tracing::info!("Meilisearch memories index initialized");
    Ok(())
}

/// Index a dream in Meilisearch
pub async fn meili_index_dream(meili_url: &str, meili_key: &str, dream: &models::Dream) {
    let client = reqwest::Client::new();
    let doc = json!([{
        "id": dream.id,
        "memory_type": "dream",
        "title": dream.title,
        "content": dream.content,
        "persona_id": dream.persona_id,
        "tags": dream.tags,
        "date": dream.timestamp.format("%Y-%m-%d").to_string(),
        "created_at_ts": dream.timestamp.timestamp()
    }]);
    let _ = client.post(format!("{}/indexes/memories/documents", meili_url))
        .bearer_auth(meili_key)
        .json(&doc)
        .send()
        .await;
}

/// Index a journal entry in Meilisearch
pub async fn meili_index_journal(meili_url: &str, meili_key: &str, entry: &models::JournalEntry) {
    let client = reqwest::Client::new();
    let doc = json!([{
        "id": entry.id,
        "memory_type": "reflection",
        "title": entry.title,
        "content": entry.content,
        "persona_id": entry.persona_id,
        "tags": entry.tags,
        "date": entry.date,
        "created_at_ts": entry.created_at.timestamp()
    }]);
    let _ = client.post(format!("{}/indexes/memories/documents", meili_url))
        .bearer_auth(meili_key)
        .json(&doc)
        .send()
        .await;
}

/// Search the memories index (lexical — for hybrid search)
pub async fn meili_search_memories(
    meili_url: &str,
    meili_key: &str,
    query: &str,
    memory_type: Option<&str>,
    persona_id: Option<&str>,
    limit: usize,
) -> Vec<serde_json::Value> {
    let client = reqwest::Client::new();
    let mut body = json!({
        "q": query,
        "limit": limit,
        "attributesToRetrieve": ["id", "memory_type", "title", "content", "persona_id", "tags"]
    });
    // Build filter: optionally by memory_type AND persona_id to prevent cross-persona leakage
    let mut filters: Vec<String> = Vec::new();
    if let Some(mt) = memory_type {
        filters.push(format!("memory_type = \"{}\"", mt));
    }
    if let Some(pid) = persona_id {
        filters.push(format!("persona_id = \"{}\"", pid));
    }
    if !filters.is_empty() {
        body["filter"] = json!(filters.join(" AND "));
    }
    match client.post(format!("{}/indexes/memories/search", meili_url))
        .bearer_auth(meili_key)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json["hits"].as_array().cloned().unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

/// Search chats index (lexical — for hybrid search)
/// Filters by ai_persona_id to prevent cross-persona memory leakage
pub async fn meili_search_chats_for_rag(
    meili_url: &str,
    meili_key: &str,
    query: &str,
    ai_persona_id: Option<&str>,
    limit: usize,
) -> Vec<serde_json::Value> {
    let client = reqwest::Client::new();
    let mut body = json!({
        "q": query,
        "limit": limit,
        "attributesToRetrieve": ["id", "title", "messages_text"]
    });
    if let Some(pid) = ai_persona_id {
        body["filter"] = json!(format!("ai_persona_id = \"{}\"", pid));
    }
    match client.post(format!("{}/indexes/chats/search", meili_url))
        .bearer_auth(meili_key)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json["hits"].as_array().cloned().unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

/// Index or update a single chat in Meilisearch
async fn meili_index_chat(meili_url: &str, meili_key: &str, chat: &models::Chat) {
    let client = reqwest::Client::new();
    let messages_text = chat.branches.iter()
        .flat_map(|b| &b.messages)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    
    // Extract ai_persona_id from the most recent assistant message
    let ai_persona_id = chat.branches.iter()
        .flat_map(|b| b.messages.iter().rev())
        .find(|m| m.role == "assistant")
        .and_then(|m| m.ai_persona.clone());
    
    let doc = json!([{
        "id": chat.id,
        "title": chat.title,
        "messages_text": messages_text,
        "group_id": chat.group_id,
        "tags": chat.tags,
        "ai_persona_id": ai_persona_id,
        "created_at_ts": chat.created_at.timestamp()
    }]);

    let _ = client.post(format!("{}/indexes/chats/documents", meili_url))
        .bearer_auth(meili_key)
        .json(&doc)
        .send()
        .await
        .map_err(|e| tracing::warn!("Meilisearch index update failed: {}", e));
}

/// Remove a chat from the Meilisearch index
async fn meili_delete_chat(meili_url: &str, meili_key: &str, chat_id: &str) {
    let client = reqwest::Client::new();
    let _ = client.delete(format!("{}/indexes/chats/documents/{}", meili_url, chat_id))
        .bearer_auth(meili_key)
        .send()
        .await
        .map_err(|e| tracing::warn!("Meilisearch delete failed: {}", e));
}

/// GET /api/dreams/search?q=term - Search dreams via Meilisearch memories index
pub async fn search_dreams(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let query = params.get("q").cloned().unwrap_or_default();
    
    if query.trim().is_empty() {
        return Ok(Json(json!({ "hits": [], "query": "" })));
    }

    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/indexes/memories/search", &state.meili_url))
        .bearer_auth(&state.meili_key)
        .json(&json!({
            "q": query,
            "filter": "memory_type = dream",
            "limit": 50,
            "attributesToRetrieve": ["id", "title", "persona_id", "tags", "date", "created_at_ts"]
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Meilisearch dreams search failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, format!("Search unavailable: {}", e))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("Meilisearch dreams search error {}: {}", status, body);
        return Err((StatusCode::SERVICE_UNAVAILABLE, "Search unavailable".to_string()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse search results: {}", e))
    })?;

    Ok(Json(body))
}

/// GET /api/journal/search?q=term - Search journal entries via Meilisearch memories index
pub async fn search_journal(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let query = params.get("q").cloned().unwrap_or_default();
    
    if query.trim().is_empty() {
        return Ok(Json(json!({ "hits": [], "query": "" })));
    }

    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/indexes/memories/search", &state.meili_url))
        .bearer_auth(&state.meili_key)
        .json(&json!({
            "q": query,
            "filter": "memory_type = reflection",
            "limit": 50,
            "attributesToRetrieve": ["id", "title", "persona_id", "tags", "date", "created_at_ts"]
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Meilisearch journal search failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, format!("Search unavailable: {}", e))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("Meilisearch journal search error {}: {}", status, body);
        return Err((StatusCode::SERVICE_UNAVAILABLE, "Search unavailable".to_string()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse search results: {}", e))
    })?;

    Ok(Json(body))
}

/// GET /api/chats/search?q=term - Search chats via Meilisearch
pub async fn search_chats(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let query = params.get("q").cloned().unwrap_or_default();
    
    if query.trim().is_empty() {
        // Return empty results for empty query (frontend shows all chats)
        return Ok(Json(json!({ "hits": [], "query": "" })));
    }

    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/indexes/chats/search", &state.meili_url))
        .bearer_auth(&state.meili_key)
        .json(&json!({
            "q": query,
            "limit": 50,
            "attributesToRetrieve": ["id", "title", "group_id", "tags", "created_at_ts"]
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Meilisearch search failed: {}", e);
            (StatusCode::SERVICE_UNAVAILABLE, format!("Search unavailable: {}", e))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("Meilisearch search error {}: {}", status, body);
        return Err((StatusCode::SERVICE_UNAVAILABLE, "Search unavailable".to_string()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse search results: {}", e))
    })?;

    Ok(Json(body))
}

// ============================================================
// Search / RAG Endpoint
// ============================================================

/// Search request
#[derive(Debug, serde::Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub memory_type: Option<String>,
}

fn default_limit() -> usize { 5 }

/// POST /api/search - Hybrid search over memories (Qdrant semantic + Meilisearch lexical)
pub async fn search_memories(
    State(state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let memory_type = payload.memory_type.as_ref().map(|t| {
        match t.as_str() {
            "conversation" => vector::MemoryType::Conversation,
            "dream" => vector::MemoryType::Dream,
            "reflection" => vector::MemoryType::Reflection,
            "fact" => vector::MemoryType::Fact,
            "emotion" => vector::MemoryType::Emotion,
            _ => vector::MemoryType::Conversation,
        }
    });

    // Qdrant — semantic search (with embedding cache)
    let semantic_results = match vector::search_memories_cached(
        &state.vector,
        &state.ollama_host,
        &state.cache,
        "azera_memory",
        &payload.query,
        payload.limit,
        memory_type,
    ).await {
        Ok(results) => results,
        Err(e) => {
            tracing::warn!("Qdrant search failed: {}", e);
            Vec::new()
        }
    };

    // Meilisearch — lexical search
    let meili_type = payload.memory_type.as_deref();
    let lexical_results = meili_search_memories(
        &state.meili_url, &state.meili_key, &payload.query, meili_type, None, payload.limit
    ).await;

    // Merge results, semantic first, then lexical (deduped)
    let mut items: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for r in &semantic_results {
        seen_ids.insert(r.id.clone());
        items.push(json!({
            "id": r.id,
            "score": r.score,
            "source": "semantic",
            "content": r.payload.get("content").and_then(|v| v.as_str()),
            "type": r.payload.get("type").and_then(|v| v.as_str()),
            "timestamp": r.payload.get("timestamp").and_then(|v| v.as_str()),
        }));
    }

    for hit in &lexical_results {
        let id = hit["id"].as_str().unwrap_or_default();
        if !seen_ids.contains(id) {
            items.push(json!({
                "id": id,
                "score": 0.5, // Default score for lexical matches
                "source": "lexical",
                "content": hit["content"].as_str(),
                "type": hit["memory_type"].as_str(),
                "title": hit["title"].as_str(),
            }));
        }
    }
    
    Ok(Json(json!({
        "results": items,
        "total": items.len(),
        "semantic_count": semantic_results.len(),
        "lexical_count": lexical_results.len()
    })))
}

/// POST /api/memories - Store a memory in vector DB (with embedding cache)
pub async fn store_memory(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let content = payload["content"].as_str().unwrap_or_default();
    let memory_type_str = payload["type"].as_str().unwrap_or("conversation");
    
    let memory_type = match memory_type_str {
        "dream" => vector::MemoryType::Dream,
        "reflection" => vector::MemoryType::Reflection,
        "fact" => vector::MemoryType::Fact,
        "emotion" => vector::MemoryType::Emotion,
        _ => vector::MemoryType::Conversation,
    };

    let id = format!("mem_{}", uuid::Uuid::new_v4());
    let metadata = std::collections::HashMap::new();

    let request = vector::StoreMemoryRequest {
        collection: "azera_memory".to_string(),
        id: id.clone(),
        content: content.to_string(),
        memory_type: memory_type.clone(),
        metadata,
    };

    match vector::store_memory_cached(
        &state.vector,
        &state.ollama_host,
        &state.cache,
        &request,
    ).await {
        Ok(()) => {
            // Also index in Meilisearch for lexical retrieval
            let meili_url = state.meili_url.clone();
            let meili_key = state.meili_key.clone();
            let content_owned = content.to_string();
            let type_str = memory_type.to_string();
            let id_clone = id.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let doc = json!([{
                    "id": id_clone,
                    "memory_type": type_str,
                    "title": "",
                    "content": content_owned,
                    "date": chrono::Utc::now().format("%Y-%m-%d").to_string(),
                    "created_at_ts": chrono::Utc::now().timestamp()
                }]);
                let _ = client.post(format!("{}/indexes/memories/documents", meili_url))
                    .bearer_auth(meili_key)
                    .json(&doc)
                    .send()
                    .await;
            });
            Ok(Json(json!({ "status": "stored", "id": id })))
        }
        Err(e) => {
            tracing::error!("Failed to store memory: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to store memory: {}", e)))
        }
    }
}

/// GET /health - Health check
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": "0.1.0",
        "service": "azera-core"
    }))
}

// ============================================================
// Model Management Endpoints
// ============================================================

/// GET /api/models - List installed Ollama models
pub async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let llm = crate::llm::LLMService::new(state.ollama_host.clone());
    
    match llm.list_models().await {
        Ok(models) => {
            // Get detailed model info from Ollama, filtering out embedding-only models
            let mut detailed_models = Vec::new();
            
            for model_name in &models {
                // Hide embedding models (e.g. nomic-embed-text) from the chat dropdown
                let lower = model_name.to_lowercase();
                if lower.contains("embed") {
                    continue;
                }
                // Ollama API returns model names in full form (e.g., "deepseek-r1:8b")
                detailed_models.push(json!({
                    "name": model_name,
                    "id": model_name.replace(":", "_"),
                }));
            }
            
            Ok(Json(json!({
                "models": detailed_models,
                "count": detailed_models.len()
            })))
        }
        Err(e) => {
            tracing::error!("Failed to list models: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list models: {}", e)))
        }
    }
}

/// POST /api/models/pull - Pull a new model from Ollama
pub async fn pull_model(
    State(state): State<AppState>,
    Json(payload): Json<models::PullModelRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("🔄 Pulling model: {}", payload.model);
    
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(100);
    let ollama_host = state.ollama_host.clone();
    let model = payload.model.clone();
    
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600)) // 1 hour timeout for large models
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        let response = client
            .post(format!("{}/api/pull", ollama_host))
            .json(&serde_json::json!({ "name": model, "stream": true }))
            .send()
            .await;
        
        match response {
            Ok(resp) if resp.status().is_success() => {
                let mut stream = resp.bytes_stream();
                
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            for line in text.lines() {
                                if line.trim().is_empty() {
                                    continue;
                                }
                                
                                if let Ok(progress) = serde_json::from_str::<models::OllamaPullProgress>(line) {
                                    let event_data = json!({
                                        "status": progress.status,
                                        "digest": progress.digest,
                                        "total": progress.total,
                                        "completed": progress.completed,
                                    });
                                    
                                    // Log download progress occasionally
                                    if let (Some(completed), Some(total)) = (progress.completed, progress.total) {
                                        if total > 0 && (completed == 0 || completed == total || completed % (total / 10).max(1) < 1_000_000) {
                                            tracing::debug!("📥 Pull progress: {} - {}/{} bytes", progress.status, completed, total);
                                        }
                                    }
                                    
                                    let _ = tx.send(Ok(Event::default()
                                        .event("progress")
                                        .data(event_data.to_string())
                                    )).await;
                                    
                                    // Model pulled successfully
                                    if progress.status == "success" {
                                        // Update the ledger
                                        if let Err(e) = update_ollama_ledger(&ollama_host).await {
                                            tracing::error!("Failed to update ledger: {}", e);
                                        }
                                        
                                        let _ = tx.send(Ok(Event::default()
                                            .event("complete")
                                            .data(json!({ "status": "success", "model": model }).to_string())
                                        )).await;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Ok(Event::default()
                                .event("error")
                                .data(json!({ "error": e.to_string() }).to_string())
                            )).await;
                            break;
                        }
                    }
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let error_text = resp.text().await.unwrap_or_default();
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(json!({ "error": format!("Pull failed ({}): {}", status, error_text) }).to_string())
                )).await;
            }
            Err(e) => {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(json!({ "error": e.to_string() }).to_string())
                )).await;
            }
        }
    });
    
    Sse::new(ReceiverStream::new(rx))
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(1))
                .text("keep-alive")
        )
}
pub async fn delete_model(
    State(state): State<AppState>,
    Path(model_name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("🗑️ Deleting model: {}", model_name);
    
    // Convert underscores back to colons (we encoded : as _ in the API)
    let model = model_name.replace("_", ":");
    
    let client = reqwest::Client::new();
    let response = client
        .delete(format!("{}/api/delete", state.ollama_host))
        .json(&serde_json::json!({ "name": model }))
        .send()
        .await;
    
    match response {
        Ok(resp) if resp.status().is_success() => {
            // Update the ledger
            if let Err(e) = update_ollama_ledger(&state.ollama_host).await {
                tracing::error!("Failed to update ledger after delete: {}", e);
            }
            
            Ok(Json(json!({
                "status": "deleted",
                "model": model
            })))
        }
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            let error_text = resp.text().await.unwrap_or_default();
            let axum_status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            Err((axum_status, format!("Failed to delete model: {}", error_text)))
        }
        Err(e) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete model: {}", e)))
        }
    }
}

// ============================================================
// User Settings Endpoints
// ============================================================

/// GET /api/settings - Get user settings
pub async fn get_settings(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::get_user_settings(&state.db).await {
        Ok(Some((editor_settings, ui_settings))) => {
            Ok(Json(serde_json::json!({
                "editorSettings": editor_settings,
                "uiSettings": ui_settings
            })))
        }
        Ok(None) => {
            // Return defaults if no settings exist
            Ok(Json(serde_json::json!({
                "editorSettings": {
                    "wordWrap": true,
                    "lineNumbers": true,
                    "fontSize": 13,
                    "tabSize": 4,
                    "lineHighlight": true,
                    "quickSuggestions": false
                },
                "uiSettings": {}
            })))
        }
        Err(e) => {
            tracing::error!("Failed to get user settings: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get settings".to_string()))
        }
    }
}

/// PUT /api/settings/editor - Update editor settings
pub async fn update_editor_settings(
    State(state): State<AppState>,
    Json(settings): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::save_editor_settings(&state.db, &settings).await {
        Ok(_) => {
            tracing::info!("✅ Updated editor settings");
            Ok(Json(serde_json::json!({ "status": "ok" })))
        }
        Err(e) => {
            tracing::error!("Failed to save editor settings: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to save settings".to_string()))
        }
    }
}

/// PUT /api/settings/ui - Update UI settings
pub async fn update_ui_settings(
    State(state): State<AppState>,
    Json(settings): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match db::save_ui_settings(&state.db, &settings).await {
        Ok(_) => {
            tracing::info!("✅ Updated UI settings");
            Ok(Json(serde_json::json!({ "status": "ok" })))
        }
        Err(e) => {
            tracing::error!("Failed to save UI settings: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to save settings".to_string()))
        }
    }
}

/// Update the Ollama ledger file after model changes
async fn update_ollama_ledger(ollama_host: &str) -> anyhow::Result<()> {
    let llm = crate::llm::LLMService::new(ollama_host.to_string());
    let models = llm.list_models().await?;
    
    let ledger = crate::backup::OllamaLedger {
        models,
        last_updated: chrono::Utc::now(),
    };
    
    let ledger_path = std::path::Path::new("../datastore/backup/ollama_ledger.json");
    
    // Ensure backup directory exists
    if let Some(parent) = ledger_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    
    let content = serde_json::to_string_pretty(&ledger)?;
    tokio::fs::write(ledger_path, content).await?;
    
    tracing::info!("📝 Updated Ollama ledger with {} models", ledger.models.len());
    Ok(())
}

// ============================================================
// TTS (Text-to-Speech) Endpoints
// ============================================================

/// Split text into chunks at sentence boundaries, respecting max length
fn chunk_text_for_tts(text: &str, max_chars: usize) -> Vec<String> {
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }
    
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    
    // Split by sentence-ending punctuation while preserving it
    let sentences: Vec<&str> = text.split_inclusive(|c| ['.', '!', '?', '\n'].contains(&c))
        .collect();
    
    for sentence in sentences {
        let trimmed = sentence.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        // If adding this sentence would exceed limit, start new chunk
        if !current_chunk.is_empty() && current_chunk.len() + trimmed.len() > max_chars {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = String::new();
        }
        
        // If single sentence is too long, split by comma/semicolon
        if trimmed.len() > max_chars {
            let sub_parts: Vec<&str> = trimmed.split_inclusive([',', ';', ':'])
                .collect();
            for part in sub_parts {
                let part_trimmed = part.trim();
                if !current_chunk.is_empty() && current_chunk.len() + part_trimmed.len() > max_chars {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk = String::new();
                }
                current_chunk.push_str(part_trimmed);
                current_chunk.push(' ');
            }
        } else {
            current_chunk.push_str(trimmed);
            current_chunk.push(' ');
        }
    }
    
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }
    
    // Filter out empty chunks
    chunks.into_iter().filter(|c| !c.is_empty()).collect()
}

/// Concatenate WAV audio data (assuming same format: 24kHz, 16-bit, mono)
fn concatenate_wav_audio(audio_chunks: Vec<Vec<u8>>) -> Vec<u8> {
    if audio_chunks.is_empty() {
        return vec![];
    }
    
    // Add 1000ms of silence at the beginning to prevent audio cutoff
    // 24kHz, 16-bit mono = 48000 bytes/sec, so 1000ms = 48000 bytes
    let silence_padding: Vec<u8> = vec![0u8; 48000];
    tracing::info!("🔊 Adding {}ms silence padding ({} bytes)", 1000, silence_padding.len());
    
    // Extract raw PCM data from each WAV (skip 44-byte header)
    let mut total_pcm_len = silence_padding.len();
    let mut pcm_chunks: Vec<&[u8]> = Vec::new();
    
    for chunk in &audio_chunks {
        if chunk.len() > 44 {
            let pcm = &chunk[44..];
            total_pcm_len += pcm.len();
            pcm_chunks.push(pcm);
            tracing::debug!("🔊 Chunk PCM size: {} bytes", pcm.len());
        }
    }
    
    if pcm_chunks.is_empty() {
        tracing::warn!("🔊 No valid PCM chunks found, returning first chunk as-is");
        return audio_chunks.into_iter().next().unwrap_or_default();
    }
    
    tracing::info!("🔊 Total PCM size with padding: {} bytes", total_pcm_len);
    
    // Create new WAV with combined audio
    // WAV header is 44 bytes
    let mut result = Vec::with_capacity(44 + total_pcm_len);
    
    // Copy header from first chunk and update sizes
    if audio_chunks[0].len() >= 44 {
        result.extend_from_slice(&audio_chunks[0][..44]);
        
        // Update file size (bytes 4-7): total_pcm_len + 36
        let file_size = (total_pcm_len + 36) as u32;
        result[4..8].copy_from_slice(&file_size.to_le_bytes());
        
        // Update data chunk size (bytes 40-43): total_pcm_len
        result[40..44].copy_from_slice(&(total_pcm_len as u32).to_le_bytes());
    }
    
    // Add silence padding first, then append all PCM data
    result.extend_from_slice(&silence_padding);
    for pcm in pcm_chunks {
        result.extend_from_slice(pcm);
    }
    
    tracing::info!("🔊 Final WAV size: {} bytes (header 44 + PCM {})", result.len(), total_pcm_len);
    
    result
}

/// POST /api/tts/synthesize - Synthesize speech from text using AI TTS (Coqui XTTS)
pub async fn synthesize_speech(
    State(state): State<AppState>,
    Json(payload): Json<models::TtsSynthesisRequest>,
) -> Result<Json<models::TtsSynthesisResponse>, (StatusCode, String)> {
    use base64::Engine;
    
    tracing::info!("🔊 TTS synthesis request: text_len={}", payload.text.len());
    
    // If persona_id is provided, get the persona's voice config
    let voice_config = if let Some(ref persona_id) = payload.persona_id {
        match db::get_persona(&state.db, persona_id).await {
            Ok(Some(persona)) => persona.voice,
            _ => None,
        }
    } else {
        None
    };
    
    // Merge voice config with request params (request params take precedence)
    let voice_sample_url = payload.voice_sample_url.clone()
        .or_else(|| voice_config.as_ref().and_then(|v| v.voice_sample_url.clone()));
    
    // Create HTTP client with longer timeout for TTS
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create HTTP client: {}", e)))?;
    
    // Check if XTTS server is available
    let xtts_health_url = format!("{}/", state.xtts_url);
    match client.get(&xtts_health_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!("🔊 XTTS server is available");
        }
        Ok(resp) => {
            tracing::warn!("🔊 XTTS server returned {}", resp.status());
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!("XTTS server not ready: {}", resp.status())
            ));
        }
        Err(e) => {
            tracing::warn!("🔊 XTTS server not available: {}", e);
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!("XTTS server not available at {}. Start with: docker compose up xtts", state.xtts_url)
            ));
        }
    }
    
    // Get speaker embeddings - either from custom voice sample or use default studio speaker
    let speaker_data: serde_json::Value = if let Some(ref sample_url) = voice_sample_url {
        tracing::info!("🔊 Cloning voice from sample: {}", sample_url);
        
        // Download voice sample
        let sample_bytes = if sample_url.starts_with("http://") || sample_url.starts_with("https://") {
            match client.get(sample_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    resp.bytes().await.ok().map(|b| b.to_vec()).unwrap_or_default()
                }
                _ => vec![]
            }
        } else if sample_url.starts_with("/voice_samples/") {
            // Convert to relative path: /voice_samples/file.wav -> ../voice_samples/file.wav
            let relative_path = format!("..{}", sample_url);
            tracing::info!("🔊 Reading voice sample from: {}", relative_path);
            tokio::fs::read(&relative_path).await.unwrap_or_default()
        } else {
            vec![]
        };
        
        if sample_bytes.is_empty() {
            tracing::warn!("🔊 Failed to load voice sample, using default speaker");
            get_default_speaker(&client, &state.xtts_url).await?
        } else {
            // Clone speaker from audio sample
            let clone_url = format!("{}/clone_speaker", state.xtts_url);
            let form = reqwest::multipart::Form::new()
                .part("wav_file", reqwest::multipart::Part::bytes(sample_bytes)
                    .file_name("speaker.wav")
                    .mime_str("audio/wav")
                    .unwrap());
            
            match client.post(&clone_url).multipart(form).send().await {
                Ok(resp) if resp.status().is_success() => {
                    resp.json::<serde_json::Value>().await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse clone response: {}", e)))?
                }
                Ok(resp) => {
                    let error = resp.text().await.unwrap_or_default();
                    tracing::warn!("🔊 Clone failed: {}, using default", error);
                    get_default_speaker(&client, &state.xtts_url).await?
                }
                Err(e) => {
                    tracing::warn!("🔊 Clone request failed: {}, using default", e);
                    get_default_speaker(&client, &state.xtts_url).await?
                }
            }
        }
    } else {
        get_default_speaker(&client, &state.xtts_url).await?
    };
    
    // Chunk long text into smaller pieces (XTTS works best with ~250 chars)
    let text_chunks = chunk_text_for_tts(&payload.text, 400);
    tracing::info!("🔊 Text split into {} chunk(s)", text_chunks.len());
    
    // Synthesize each chunk
    let tts_url = format!("{}/tts", state.xtts_url);
    let mut audio_parts: Vec<Vec<u8>> = Vec::new();
    
    for (i, chunk) in text_chunks.iter().enumerate() {
        tracing::info!("🔊 Synthesizing chunk {}/{}: {} chars", i + 1, text_chunks.len(), chunk.len());
        
        let tts_request = json!({
            "text": chunk,
            "language": "en",
            "speaker_embedding": speaker_data["speaker_embedding"],
            "gpt_cond_latent": speaker_data["gpt_cond_latent"]
        });
        
        let response = client
            .post(&tts_url)
            .json(&tts_request)
            .send()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("XTTS request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("🔊 XTTS error on chunk {}: {} - {}", i + 1, status, error_text);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("XTTS synthesis failed on chunk {}: {} - {}", i + 1, status, error_text)
            ));
        }
        
        // XTTS returns JSON with base64-encoded audio string
        let audio_base64: String = response.json().await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse XTTS response: {}", e)))?;
        
        // Decode to get raw audio bytes
        let audio_bytes = base64::engine::general_purpose::STANDARD.decode(&audio_base64)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to decode audio: {}", e)))?;
        
        audio_parts.push(audio_bytes);
    }
    
    // Concatenate all audio chunks
    let combined_audio = concatenate_wav_audio(audio_parts);
    let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&combined_audio);
    
    tracing::info!("🔊 Generated combined audio: {} bytes ({} base64 chars)", combined_audio.len(), audio_base64.len());
    
    // Estimate duration (WAV at 24000Hz, 16-bit mono = ~48000 bytes per second)
    let audio_bytes_len = combined_audio.len();
    let duration_ms = (audio_bytes_len as u64 * 1000) / 48000;
    
    Ok(Json(models::TtsSynthesisResponse {
        audio_base64,
        format: "wav".to_string(),
        duration_ms,
    }))
}

/// Get default studio speaker embeddings from XTTS
async fn get_default_speaker(client: &reqwest::Client, xtts_url: &str) -> Result<serde_json::Value, (StatusCode, String)> {
    let speakers_url = format!("{}/studio_speakers", xtts_url);
    let speakers: serde_json::Value = client.get(&speakers_url)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get speakers: {}", e)))?
        .json()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse speakers: {}", e)))?;
    
    // Use "Sofia Hellen" as default female voice (or first available)
    let default_speaker = speakers.get("Sofia Hellen")
        .or_else(|| speakers.as_object().and_then(|o| o.values().next()))
        .cloned()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "No speakers available".to_string()))?;
    
    Ok(default_speaker)
}

// ============================================================
// Voice Sample Upload/Download
// ============================================================

/// POST /api/voice-samples/upload - Upload a voice sample for cloning
pub async fn upload_voice_sample(
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use std::io::Write;
    
    // Use relative path that works both locally and in Docker
    let voice_samples_dir = std::path::PathBuf::from("../voice_samples");
    
    // Ensure directory exists
    if !voice_samples_dir.exists() {
        tokio::fs::create_dir_all(&voice_samples_dir)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create voice_samples directory: {}", e)))?;
    }
    
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read multipart field: {}", e)))? 
    {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" || field_name == "audio" {
            let original_filename = field.file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "voice_sample.wav".to_string());
            
            // Generate unique filename
            let ext = std::path::Path::new(&original_filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("wav");
            let filename = format!("{}_{}.{}", 
                chrono::Utc::now().timestamp(), 
                uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("sample"),
                ext
            );
            
            let file_path = voice_samples_dir.join(&filename);
            
            // Read file data
            let data = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read file data: {}", e)))?;
            
            // Validate file size (max 10MB)
            if data.len() > 10 * 1024 * 1024 {
                return Err((StatusCode::BAD_REQUEST, "File too large (max 10MB)".to_string()));
            }
            
            // Write file
            let mut file = std::fs::File::create(&file_path)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create file: {}", e)))?;
            
            file.write_all(&data)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write file: {}", e)))?;
            
            tracing::info!("🎤 Uploaded voice sample: {} ({} bytes)", filename, data.len());
            
            // Return the URL path that can be used for TTS
            let url = format!("/voice_samples/{}", filename);
            
            return Ok(Json(json!({
                "success": true,
                "filename": filename,
                "url": url,
                "size": data.len()
            })));
        }
    }
    
    Err((StatusCode::BAD_REQUEST, "No audio file found in request".to_string()))
}

/// GET /api/voice-samples/:filename - Download/stream a voice sample
pub async fn get_voice_sample(
    Path(filename): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    use axum::http::header;
    
    // Sanitize filename to prevent path traversal
    let safe_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or((StatusCode::BAD_REQUEST, "Invalid filename".to_string()))?;
    
    // Use relative path that works both locally and in Docker
    let file_path = std::path::PathBuf::from("../voice_samples").join(safe_filename);
    
    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, format!("Voice sample not found: {}", safe_filename)));
    }
    
    // Read file
    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read file: {}", e)))?;
    
    // Determine content type from extension
    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("m4a") => "audio/mp4",
        Some("webm") => "audio/webm",
        _ => "application/octet-stream",
    };
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, data.len())
        .body(Body::from(data))
        .unwrap())
}

// ============================================================
// Image Generation Endpoints
// ============================================================

/// POST /api/images/generate - Generate an image from a prompt (SSE streaming)
pub async fn generate_image(
    State(state): State<AppState>,
    Json(payload): Json<models::ImageGenerationRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("🎨 Generating image: {}", payload.prompt);
    
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(100);
    let image_gen_url = std::env::var("IMAGE_GEN_URL").ok();
    let prompt = payload.prompt.clone();
    let model = payload.model.clone().unwrap_or_else(|| "stable-diffusion".to_string());
    let width = payload.width.unwrap_or(512);
    let height = payload.height.unwrap_or(512);
    let steps = payload.steps.unwrap_or(20);
    let cfg_scale = payload.cfg_scale.unwrap_or(7.0);
    let seed = payload.seed.unwrap_or(-1);
    let persona_id = payload.persona_id.clone();
    let custom_filename = payload.custom_filename.clone();
    let negative_prompt = payload.negative_prompt.clone();
    let reference_image = payload.reference_image.clone();
    let reference_strength = payload.reference_strength.unwrap_or(0.75);
    
    // Fetch persona name if persona_id is provided
    let db = state.db.clone();
    
    tokio::spawn(async move {
        // Get persona name for filename prefix
        let persona_name: Option<String> = if let Some(ref pid) = persona_id {
            match crate::db::get_persona(&db, pid).await {
                Ok(Some(p)) => Some(p.name.to_lowercase().replace(' ', "_")),
                _ => None,
            }
        } else {
            None
        };
        
        // Send progress start
        let _ = tx.send(Ok(Event::default()
            .event("progress")
            .data(json!({"step": 0, "total_steps": steps, "percentage": 0.0}).to_string())
        )).await;
        
        // If no image gen service is configured, create a placeholder
        if image_gen_url.is_none() {
            // Simulate generation with placeholder
            for step in 1..=steps {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let percentage = (step as f32 / steps as f32) * 100.0;
                let _ = tx.send(Ok(Event::default()
                    .event("progress")
                    .data(json!({"step": step, "total_steps": steps, "percentage": percentage}).to_string())
                )).await;
            }
            
            // Generate a simple placeholder SVG
            let prompt_preview: String = prompt.chars().take(50).collect();
            let placeholder_svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\
                <rect fill=\"#1a1a2e\" width=\"100%\" height=\"100%\"/>\
                <text x=\"50%\" y=\"40%\" text-anchor=\"middle\" fill=\"#8888ff\" font-family=\"Arial\" font-size=\"24\">Image Generation</text>\
                <text x=\"50%\" y=\"55%\" text-anchor=\"middle\" fill=\"#aaaaaa\" font-family=\"Arial\" font-size=\"14\">Configure IMAGE_GEN_URL to enable</text>\
                <text x=\"50%\" y=\"70%\" text-anchor=\"middle\" fill=\"#666666\" font-family=\"Arial\" font-size=\"12\">{}</text>\
                </svg>",
                width, height, width, height, prompt_preview
            );
            
            // Save placeholder as PNG (convert SVG to PNG using a simple approach or just save SVG)
            let image_id = uuid::Uuid::new_v4().to_string();
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
            let filename = if let Some(ref name) = custom_filename {
                if let Some(ref pname) = persona_name {
                    format!("{}_{}.svg", pname, name.replace(' ', "_"))
                } else {
                    format!("{}.svg", name.replace(' ', "_"))
                }
            } else if let Some(ref pname) = persona_name {
                format!("{}_{}_{}.svg", pname, timestamp, &image_id[..8])
            } else {
                format!("{}_{}.svg", timestamp, &image_id[..8])
            };
            
            // Ensure canvas directory exists
            let canvas_dir = std::path::PathBuf::from("./atelier/canvas");
            if let Err(e) = tokio::fs::create_dir_all(&canvas_dir).await {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(json!({"message": format!("Failed to create canvas directory: {}", e)}).to_string())
                )).await;
                return;
            }
            
            let file_path = canvas_dir.join(&filename);
            if let Err(e) = tokio::fs::write(&file_path, &placeholder_svg).await {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(json!({"message": format!("Failed to save image: {}", e)}).to_string())
                )).await;
                return;
            }
            
            let image = models::GeneratedImage {
                id: image_id,
                filename: filename.clone(),
                url: format!("/api/images/{}", filename),
                prompt: prompt.clone(),
                negative_prompt,
                model: Some(model),
                width,
                height,
                steps: Some(steps),
                cfg_scale: Some(cfg_scale),
                seed: Some(seed),
                persona_id,
                persona_name,
                created_at: chrono::Utc::now(),
            };
            
            tracing::info!("🖼️ Created placeholder image: {}", filename);
            
            let _ = tx.send(Ok(Event::default()
                .event("complete")
                .data(serde_json::to_string(&models::ImageGenEvent::Complete { image: Box::new(image) }).unwrap())
            )).await;
            
            return;
        }
        
        // Real image generation via external service
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        let image_gen_host = image_gen_url.unwrap();
        
        // Build request body for Automatic1111 / ComfyUI compatible API
        let mut request_body = json!({
            "prompt": prompt,
            "negative_prompt": negative_prompt.clone().unwrap_or_default(),
            "width": width,
            "height": height,
            "steps": steps,
            "cfg_scale": cfg_scale,
            "seed": seed,
            "override_settings": {
                "sd_model_checkpoint": model,
            },
            "override_settings_restore_afterwards": true,
        });
        
        // Add reference image (img2img) if provided
        if let Some(ref_img) = reference_image {
            request_body["init_images"] = json!([ref_img]);
            request_body["denoising_strength"] = json!(reference_strength);
        }
        
        // Choose txt2img or img2img endpoint
        let endpoint = if request_body.get("init_images").is_some() {
            format!("{}/sdapi/v1/img2img", image_gen_host)
        } else {
            format!("{}/sdapi/v1/txt2img", image_gen_host)
        };
        
        // Fire generation request and poll progress in parallel
        let gen_future = client.post(&endpoint).json(&request_body).send();
        tokio::pin!(gen_future);
        
        let progress_url = format!("{}/sdapi/v1/progress", image_gen_host);
        let progress_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let mut poll_interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        poll_interval.tick().await; // skip immediate first tick
        
        let gen_result = loop {
            tokio::select! {
                result = &mut gen_future => {
                    break result;
                }
                _ = poll_interval.tick() => {
                    if let Ok(resp) = progress_client.get(&progress_url).send().await {
                        if let Ok(prog) = resp.json::<serde_json::Value>().await {
                            let step = prog.get("step").and_then(|s| s.as_i64()).unwrap_or(0);
                            let total = prog.get("total_steps").and_then(|s| s.as_i64()).unwrap_or(steps as i64);
                            let pct = prog.get("percentage").and_then(|p| p.as_f64()).unwrap_or(0.0);
                            if step > 0 {
                                let _ = tx.send(Ok(Event::default()
                                    .event("progress")
                                    .data(json!({"step": step, "total_steps": total, "percentage": pct}).to_string())
                                )).await;
                            }
                        }
                    }
                }
            }
        };
        
        match gen_result {
            Ok(response) if response.status().is_success() => {
                match response.json::<serde_json::Value>().await {
                    Ok(result) => {
                        // Extract base64 image from response
                        if let Some(images) = result.get("images").and_then(|i| i.as_array()) {
                            if let Some(image_b64) = images.first().and_then(|i| i.as_str()) {
                                // Decode and save image
                                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, image_b64) {
                                    Ok(image_data) => {
                                        let image_id = uuid::Uuid::new_v4().to_string();
                                        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                                        let filename = if let Some(ref name) = custom_filename {
                                            if let Some(ref pname) = persona_name {
                                                format!("{}_{}.png", pname, name.replace(' ', "_"))
                                            } else {
                                                format!("{}.png", name.replace(' ', "_"))
                                            }
                                        } else if let Some(ref pname) = persona_name {
                                            format!("{}_{}_{}.png", pname, timestamp, &image_id[..8])
                                        } else {
                                            format!("{}_{}.png", timestamp, &image_id[..8])
                                        };
                                        
                                        let canvas_dir = std::path::PathBuf::from("./atelier/canvas");
                                        let _ = tokio::fs::create_dir_all(&canvas_dir).await;
                                        let file_path = canvas_dir.join(&filename);
                                        
                                        if let Err(e) = tokio::fs::write(&file_path, &image_data).await {
                                            let _ = tx.send(Ok(Event::default()
                                                .event("error")
                                                .data(json!({"message": format!("Failed to save image: {}", e)}).to_string())
                                            )).await;
                                            return;
                                        }
                                        
                                        // Get actual seed from response info if available
                                        let actual_seed = result.get("info")
                                            .and_then(|i| i.as_str())
                                            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                                            .and_then(|v| v.get("seed").and_then(|s| s.as_i64()))
                                            .unwrap_or(seed);
                                        
                                        let image = models::GeneratedImage {
                                            id: image_id,
                                            filename: filename.clone(),
                                            url: format!("/api/images/{}", filename),
                                            prompt: prompt.clone(),
                                            negative_prompt,
                                            model: Some(model),
                                            width,
                                            height,
                                            steps: Some(steps),
                                            cfg_scale: Some(cfg_scale),
                                            seed: Some(actual_seed),
                                            persona_id,
                                            persona_name,
                                            created_at: chrono::Utc::now(),
                                        };
                                        
                                        tracing::info!("🖼️ Generated image: {}", filename);
                                        
                                        let _ = tx.send(Ok(Event::default()
                                            .event("complete")
                                            .data(serde_json::to_string(&models::ImageGenEvent::Complete { image: Box::new(image) }).unwrap())
                                        )).await;
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Ok(Event::default()
                                            .event("error")
                                            .data(json!({"message": format!("Failed to decode image: {}", e)}).to_string())
                                        )).await;
                                    }
                                }
                            } else {
                                let _ = tx.send(Ok(Event::default()
                                    .event("error")
                                    .data(json!({"message": "No image in response"}).to_string())
                                )).await;
                            }
                        } else {
                            let _ = tx.send(Ok(Event::default()
                                .event("error")
                                .data(json!({"message": "Invalid response format"}).to_string())
                            )).await;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Ok(Event::default()
                            .event("error")
                            .data(json!({"message": format!("Failed to parse response: {}", e)}).to_string())
                        )).await;
                    }
                }
            }
            Ok(resp) => {
                let error_text = resp.text().await.unwrap_or_default();
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(json!({"message": format!("Image generation failed: {}", error_text)}).to_string())
                )).await;
            }
            Err(e) => {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(json!({"message": format!("Request failed: {}", e)}).to_string())
                )).await;
            }
        }
    });
    
    Sse::new(ReceiverStream::new(rx))
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(1))
                .text("keep-alive")
        )
}

/// GET /api/images - List all generated images
pub async fn list_images() -> Result<Json<models::ListResponse<models::GeneratedImage>>, (StatusCode, String)> {
    let canvas_dir = std::path::PathBuf::from("./atelier/canvas");
    
    if !canvas_dir.exists() {
        return Ok(Json(models::ListResponse { items: vec![], total: 0 }));
    }
    
    let mut images = Vec::new();
    
    let mut entries = tokio::fs::read_dir(&canvas_dir)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read canvas directory: {}", e)))?;
    
    while let Some(entry) = entries.next_entry().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read directory entry: {}", e)))? 
    {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ["png", "jpg", "jpeg", "svg", "webp"].contains(&ext.to_lowercase().as_str()) {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                let metadata = entry.metadata().await.ok();
                let created_at = metadata
                    .and_then(|m| m.created().ok())
                    .map(chrono::DateTime::<chrono::Utc>::from)
                    .unwrap_or_else(chrono::Utc::now);
                
                // Try to extract persona name from filename prefix
                let parts: Vec<&str> = filename.split('_').collect();
                let (persona_name, _) = if parts.len() > 1 {
                    // Check if first part looks like a persona name (not a timestamp)
                    if parts[0].parse::<i64>().is_err() && !parts[0].starts_with("20") {
                        (Some(parts[0].to_string()), parts[1..].join("_"))
                    } else {
                        (None, filename.clone())
                    }
                } else {
                    (None, filename.clone())
                };
                
                images.push(models::GeneratedImage {
                    id: filename.clone(),
                    filename: filename.clone(),
                    url: format!("/api/images/{}", filename),
                    prompt: "".to_string(),  // Would need metadata file to store this
                    negative_prompt: None,
                    model: None,
                    width: 512,  // Default, would need metadata
                    height: 512,
                    steps: None,
                    cfg_scale: None,
                    seed: None,
                    persona_id: None,
                    persona_name,
                    created_at,
                });
            }
        }
    }
    
    // Sort by creation date, newest first
    images.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    
    let total = images.len();
    Ok(Json(models::ListResponse { items: images, total }))
}

/// GET /api/images/:filename - Serve a generated image
pub async fn get_image(
    Path(filename): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    use axum::http::header;
    
    // Sanitize filename
    let safe_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or((StatusCode::BAD_REQUEST, "Invalid filename".to_string()))?;
    
    let file_path = std::path::PathBuf::from("./atelier/canvas").join(safe_filename);
    
    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, format!("Image not found: {}", safe_filename)));
    }
    
    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read image: {}", e)))?;
    
    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    };
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, "public, max-age=31536000")
        .body(Body::from(data))
        .unwrap())
}

/// DELETE /api/images/:filename - Delete an image
pub async fn delete_image(
    Path(filename): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let safe_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or((StatusCode::BAD_REQUEST, "Invalid filename".to_string()))?;
    
    let file_path = std::path::PathBuf::from("./atelier/canvas").join(safe_filename);
    
    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, format!("Image not found: {}", safe_filename)));
    }
    
    tokio::fs::remove_file(&file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete image: {}", e)))?;
    
    tracing::info!("🗑️ Deleted image: {}", safe_filename);
    
    Ok(Json(json!({ "status": "deleted", "filename": safe_filename })))
}

/// POST /api/images/upload-reference - Upload a reference image for img2img
pub async fn upload_reference_image(
    mut multipart: Multipart,
) -> Result<Json<models::ImageUploadResponse>, (StatusCode, String)> {
    let refs_dir = std::path::PathBuf::from("./atelier/canvas/references");
    
    if !refs_dir.exists() {
        tokio::fs::create_dir_all(&refs_dir)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create references directory: {}", e)))?;
    }
    
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read multipart field: {}", e)))? 
    {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" || field_name == "image" {
            let original_filename = field.file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "reference.png".to_string());
            
            let ext = std::path::Path::new(&original_filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            
            let id = uuid::Uuid::new_v4().to_string();
            let filename = format!("ref_{}_{}.{}", 
                chrono::Utc::now().timestamp(),
                &id[..8],
                ext
            );
            
            let file_path = refs_dir.join(&filename);
            
            let data = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read file data: {}", e)))?;
            
            // Validate file size (max 20MB for images)
            if data.len() > 20 * 1024 * 1024 {
                return Err((StatusCode::BAD_REQUEST, "File too large (max 20MB)".to_string()));
            }
            
            tokio::fs::write(&file_path, &data)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save file: {}", e)))?;
            
            tracing::info!("📤 Uploaded reference image: {} ({} bytes)", filename, data.len());
            
            return Ok(Json(models::ImageUploadResponse {
                id,
                url: format!("/api/images/references/{}", filename),
            }));
        }
    }
    
    Err((StatusCode::BAD_REQUEST, "No image file found in request".to_string()))
}

/// GET /api/images/references/:filename - Serve a reference image
pub async fn get_reference_image(
    Path(filename): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    use axum::http::header;
    
    let safe_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or((StatusCode::BAD_REQUEST, "Invalid filename".to_string()))?;
    
    let file_path = std::path::PathBuf::from("./atelier/canvas/references").join(safe_filename);
    
    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, format!("Reference image not found: {}", safe_filename)));
    }
    
    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read image: {}", e)))?;
    
    let content_type = match file_path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    };
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, data.len())
        .body(Body::from(data))
        .unwrap())
}

/// GET /api/images/models - List available image generation models
/// Returns the pre-installed models served by the image generation sidecar
pub async fn list_image_models() -> Json<Vec<models::ImageModel>> {
    // Query the SD WebUI API for real available models
    let image_gen_url = std::env::var("IMAGE_GEN_URL").unwrap_or_default();
    if image_gen_url.is_empty() {
        return Json(vec![]);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let url = format!("{}/sdapi/v1/sd-models", image_gen_url);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<Vec<serde_json::Value>>().await {
                Ok(sd_models) => {
                    let models: Vec<models::ImageModel> = sd_models.iter().map(|m| {
                        let title = m.get("title").and_then(|t| t.as_str()).unwrap_or("unknown");
                        let model_name = m.get("model_name").and_then(|n| n.as_str()).unwrap_or(title);
                        let description = m.get("description").and_then(|d| d.as_str()).unwrap_or(title);
                        models::ImageModel {
                            name: title.to_string(),
                            display_name: model_name.to_string(),
                            description: Some(description.to_string()),
                            installed: true,
                        }
                    }).collect();
                    Json(models)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse SD models: {}", e);
                    Json(vec![])
                }
            }
        }
        Ok(resp) => {
            tracing::warn!("SD WebUI returned {}", resp.status());
            Json(vec![])
        }
        Err(e) => {
            tracing::warn!("Failed to reach SD WebUI: {}", e);
            Json(vec![])
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod chunk_text_for_tts_tests {
        use super::*;

        #[test]
        fn short_text_returns_single_chunk() {
            let text = "Hello, world!";
            let chunks = chunk_text_for_tts(text, 100);
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0], "Hello, world!");
        }

        #[test]
        fn empty_text_returns_single_chunk() {
            // Empty string returns vec with the original text (empty)
            let chunks = chunk_text_for_tts("", 100);
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0], "");
        }

        #[test]
        fn splits_at_sentence_boundaries() {
            let text = "First sentence. Second sentence. Third sentence.";
            let chunks = chunk_text_for_tts(text, 30);
            assert!(chunks.len() >= 2);
            // Each chunk should end with a sentence
            assert!(chunks[0].ends_with('.'));
        }

        #[test]
        fn handles_multiple_punctuation_types() {
            let text = "Hello! How are you? I'm fine.";
            let chunks = chunk_text_for_tts(text, 15);
            assert!(chunks.len() >= 2);
        }

        #[test]
        fn handles_newlines() {
            let text = "First line\nSecond line\nThird line";
            let chunks = chunk_text_for_tts(text, 20);
            assert!(!chunks.is_empty());
        }

        #[test]
        fn respects_max_char_limit() {
            let text = "This is a longer sentence that should be split. And another one here.";
            let max_chars = 30;
            let chunks = chunk_text_for_tts(text, max_chars);
            // Each chunk should generally respect the limit (some may exceed due to sentence integrity)
            for chunk in &chunks {
                // Allow some overflow for complete sentences
                assert!(chunk.len() <= max_chars * 2, "Chunk too long: {} chars", chunk.len());
            }
        }

        #[test]
        fn handles_long_sentences_with_commas() {
            let text = "This is a very, very, very long sentence with many commas, and it should be split at comma boundaries when needed.";
            let chunks = chunk_text_for_tts(text, 40);
            assert!(!chunks.is_empty());
        }

        #[test]
        fn preserves_all_content() {
            let text = "First. Second. Third.";
            let chunks = chunk_text_for_tts(text, 10);
            let rejoined: String = chunks.join(" ");
            // All original content should be present (punctuation preserved)
            assert!(rejoined.contains("First"));
            assert!(rejoined.contains("Second"));
            assert!(rejoined.contains("Third"));
        }
    }

    mod concatenate_wav_audio_tests {
        use super::*;

        fn create_mock_wav_header(data_size: u32) -> Vec<u8> {
            let mut header = vec![0u8; 44];
            // RIFF header
            header[0..4].copy_from_slice(b"RIFF");
            let file_size = data_size + 36;
            header[4..8].copy_from_slice(&file_size.to_le_bytes());
            header[8..12].copy_from_slice(b"WAVE");
            // fmt chunk
            header[12..16].copy_from_slice(b"fmt ");
            header[16..20].copy_from_slice(&16u32.to_le_bytes()); // chunk size
            header[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
            header[22..24].copy_from_slice(&1u16.to_le_bytes()); // mono
            header[24..28].copy_from_slice(&24000u32.to_le_bytes()); // sample rate
            header[28..32].copy_from_slice(&48000u32.to_le_bytes()); // byte rate
            header[32..34].copy_from_slice(&2u16.to_le_bytes()); // block align
            header[34..36].copy_from_slice(&16u16.to_le_bytes()); // bits per sample
            // data chunk
            header[36..40].copy_from_slice(b"data");
            header[40..44].copy_from_slice(&data_size.to_le_bytes());
            header
        }

        #[test]
        fn empty_input_returns_empty_vec() {
            let result = concatenate_wav_audio(vec![]);
            assert!(result.is_empty());
        }

        #[test]
        fn single_chunk_adds_silence_padding() {
            let mut wav = create_mock_wav_header(100);
            wav.extend(vec![1u8; 100]); // PCM data
            
            let result = concatenate_wav_audio(vec![wav]);
            
            // Should have header (44) + silence (48000) + original PCM (100)
            assert_eq!(result.len(), 44 + 48000 + 100);
            
            // First 44 bytes should be header
            assert_eq!(&result[0..4], b"RIFF");
            
            // Silence padding should be zeros (after header)
            let silence_section = &result[44..44 + 48000];
            assert!(silence_section.iter().all(|&b| b == 0));
        }

        #[test]
        fn multiple_chunks_concatenated() {
            let mut wav1 = create_mock_wav_header(50);
            wav1.extend(vec![1u8; 50]);
            
            let mut wav2 = create_mock_wav_header(50);
            wav2.extend(vec![2u8; 50]);
            
            let result = concatenate_wav_audio(vec![wav1, wav2]);
            
            // Should have header (44) + silence (48000) + pcm1 (50) + pcm2 (50)
            assert_eq!(result.len(), 44 + 48000 + 50 + 50);
        }

        #[test]
        fn updates_wav_header_sizes() {
            let mut wav1 = create_mock_wav_header(100);
            wav1.extend(vec![1u8; 100]);
            
            let mut wav2 = create_mock_wav_header(100);
            wav2.extend(vec![2u8; 100]);
            
            let result = concatenate_wav_audio(vec![wav1, wav2]);
            
            // Check file size in header (bytes 4-7)
            let file_size = u32::from_le_bytes([result[4], result[5], result[6], result[7]]);
            // file_size = total_pcm_len + 36 = (48000 + 100 + 100) + 36 = 48236
            assert_eq!(file_size, 48000 + 100 + 100 + 36);
            
            // Check data chunk size (bytes 40-43)
            let data_size = u32::from_le_bytes([result[40], result[41], result[42], result[43]]);
            assert_eq!(data_size, 48000 + 100 + 100);
        }
    }

    mod model_serialization_tests {
        use crate::models::*;

        #[test]
        fn chat_request_deserializes_with_defaults() {
            let json = r#"{
                "chat_id": "test-chat",
                "branch_id": "main",
                "message": "Hello"
            }"#;
            
            let request: ChatRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.chat_id, "test-chat");
            assert_eq!(request.model, "llama3.2"); // default value
        }

        #[test]
        fn stream_event_serializes_correctly() {
            let event = StreamEvent::Content { 
                content: "Hello".to_string() 
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("\"type\":\"content\""));
            assert!(json.contains("\"content\":\"Hello\""));
        }

        #[test]
        fn stream_event_done_includes_optional_mood() {
            let event = StreamEvent::Done { 
                message_id: "msg-123".to_string(),
                mood: Some("happy".to_string()),
                mood_value: Some(0.85),
                energy: Some(0.7),
            };
            let json = serde_json::to_string(&event).unwrap();
            assert!(json.contains("\"mood\":\"happy\""));
        }

        #[test]
        fn voice_config_defaults() {
            let json = r#"{}"#;
            let config: VoiceConfig = serde_json::from_str(json).unwrap();
            assert_eq!(config.pitch, 1.0);
            assert_eq!(config.rate, 1.0);
            assert_eq!(config.volume, 1.0);
            assert!(!config.use_ai_tts);
        }

        #[test]
        fn tag_serialization_roundtrip() {
            let tag = Tag {
                id: "test-id".to_string(),
                name: "Test Tag".to_string(),
                color: "#ff0000".to_string(),
            };
            let json = serde_json::to_string(&tag).unwrap();
            let deserialized: Tag = serde_json::from_str(&json).unwrap();
            assert_eq!(tag.id, deserialized.id);
            assert_eq!(tag.name, deserialized.name);
            assert_eq!(tag.color, deserialized.color);
        }
    }
    
    mod image_gen_extraction_tests {
        use super::*;
        
        #[test]
        fn extracts_single_image_request() {
            let text = r#"Here's an image for you! [IMAGE_GEN: prompt="A beautiful sunset over mountains"]"#;
            let requests = extract_image_gen_requests(text);
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].0, "A beautiful sunset over mountains");
            assert_eq!(requests[0].1, None);
        }
        
        #[test]
        fn extracts_image_request_with_name() {
            let text = r#"[IMAGE_GEN: prompt="A mystical forest", name="enchanted_woods"]"#;
            let requests = extract_image_gen_requests(text);
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].0, "A mystical forest");
            assert_eq!(requests[0].1, Some("enchanted_woods".to_string()));
        }
        
        #[test]
        fn extracts_multiple_image_requests() {
            let text = r#"Creating two images: [IMAGE_GEN: prompt="Ocean waves"] and [IMAGE_GEN: prompt="Mountain peak", name="summit"]"#;
            let requests = extract_image_gen_requests(text);
            assert_eq!(requests.len(), 2);
            assert_eq!(requests[0].0, "Ocean waves");
            assert_eq!(requests[1].0, "Mountain peak");
            assert_eq!(requests[1].1, Some("summit".to_string()));
        }
        
        #[test]
        fn returns_empty_for_no_requests() {
            let text = "This is just regular text without any image generation requests.";
            let requests = extract_image_gen_requests(text);
            assert!(requests.is_empty());
        }
        
        #[test]
        fn handles_empty_string() {
            let requests = extract_image_gen_requests("");
            assert!(requests.is_empty());
        }
        
        #[test]
        fn ignores_malformed_tags() {
            let text = r#"This [IMAGE_GEN: is malformed] and this [IMAGE_GEN prompt="missing colon"] too"#;
            let requests = extract_image_gen_requests(text);
            assert!(requests.is_empty());
        }
    }
}