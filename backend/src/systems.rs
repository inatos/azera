use crate::*;
use chrono::Utc;
use chrono::Timelike;
use std::time::Duration;
use tokio::time::sleep;

/// Run the main tick loop
pub async fn run_tick_loop(state: AppState) {
    tracing::info!("🌙 Azera Tick Loop starting...");

    let mut tick_count = 0u64;
    loop {
        tick_count += 1;

        // Run all systems in sequence
        input_system(&state).await;
        perception_system(&state).await;
        cognitive_system(&state).await;
        dreaming_system(&state).await;
        reflection_system(&state).await;
        action_system(&state).await;

        // Log every 100 ticks
        if tick_count % 100 == 0 {
            tracing::debug!("✨ Tick {}", tick_count);
        }

        // Tick every ~1 second
        sleep(Duration::from_millis(1000)).await;
    }
}

/// Input System: Check for incoming signals
async fn input_system(state: &AppState) {
    if let Ok(Some(signal)) = cache::CacheService::dequeue_signal(&state.cache, "input_queue").await {
        tracing::debug!("📥 Input signal: {}", signal);
        
        let mut agent = state.agent.write().await;
        agent.set_pending_input(signal);
    }
}

/// Perception System: Sync Dragonfly state → agent, update components
async fn perception_system(state: &AppState) {
    // Sync mental state from Dragonfly (working memory) → agent state
    if let Ok(Some(cached_state)) = cache::CacheService::get_mental_state(&state.cache).await {
        let mut agent = state.agent.write().await;
        agent.mental_state.mood = cached_state.mood;
        agent.mental_state.energy = cached_state.energy;
        agent.mental_state.focus_level = cached_state.focus_level;
        agent.mental_state.is_dreaming = cached_state.is_dreaming;
    }
    
    let mut agent = state.agent.write().await;
    
    // Natural energy recovery when idle
    let idle_seconds = chrono::Utc::now()
        .signed_duration_since(agent.mental_state.last_active)
        .num_seconds();
    if idle_seconds > 60 {
        // Slowly recover energy when idle (0.01 per tick when idle > 1min)
        agent.mental_state.energy = (agent.mental_state.energy + 0.001).min(1.0);
        // Mood drifts toward neutral when idle
        let neutral = 0.5;
        let diff = neutral - agent.mental_state.mood;
        agent.mental_state.mood += diff * 0.001;
        // Focus drops when idle
        agent.mental_state.focus_level = (agent.mental_state.focus_level - 0.001).max(0.3);
    }
    
    // Write back to Dragonfly periodically (every ~10 seconds via tick)
    // Only if we made changes (idle drift)
    if idle_seconds > 60 {
        let _ = cache::CacheService::set_mental_state(&state.cache, &cache::CachedMentalState {
            mood: agent.mental_state.mood,
            energy: agent.mental_state.energy,
            focus_level: agent.mental_state.focus_level,
            mood_label: if agent.mental_state.mood > 0.7 { "happy".to_string() }
                else if agent.mental_state.mood > 0.5 { "content".to_string() }
                else if agent.mental_state.mood > 0.3 { "thoughtful".to_string() }
                else { "melancholy".to_string() },
            is_dreaming: agent.mental_state.is_dreaming,
            last_active: agent.mental_state.last_active,
            updated_at: chrono::Utc::now(),
        }).await;
    }
}

/// Cognitive System: Process pending input via LLM
async fn cognitive_system(state: &AppState) {
    let mut agent = state.agent.write().await;

    if let Some(pending_input) = agent.get_pending_input() {
        drop(agent); // Release lock before async call

        tracing::info!("🧠 Processing: {}", pending_input);

        // Get system prompt from default AI persona
        let system_prompt = match db::get_persona(&state.db, "azera").await {
            Ok(Some(persona)) => persona.system_prompt.unwrap_or_else(default_system_prompt),
            _ => default_system_prompt(),
        };

        let llm = llm::LLMService::new(state.ollama_host.clone());
        
        let messages = llm::LLMService::build_messages(
            &system_prompt,
            &[],
            &pending_input,
        );

        match llm.infer(&state.agent.read().await.agent_config.model, messages).await {
            Ok(response) => {
                tracing::info!("✨ Response generated: {} chars", response.len());
                
                // Save to legacy database
                let _ = db::save_message(&state.db, "default", "user", &pending_input).await;
                let _ = db::save_message(&state.db, "default", "assistant", &response).await;
                
                // Store in cache
                let _ = cache::CacheService::set(
                    &state.cache,
                    "latest_response",
                    &response,
                    3600,
                ).await;

                // Log the interaction
                let _ = db::add_log(&state.db, "info", &format!("Processed message: {}", pending_input.chars().take(50).collect::<String>())).await;
            }
            Err(e) => {
                tracing::error!("❌ LLM inference failed: {}", e);
                let _ = db::add_log(&state.db, "error", &format!("LLM inference failed: {}", e)).await;
            }
        }

        // Re-acquire lock
        let mut agent = state.agent.write().await;
        agent.mental_state.energy = (agent.mental_state.energy - 0.05).max(0.0);
        agent.mental_state.last_active = Utc::now();
    }
}

fn default_system_prompt() -> String {
    "You are Azera, a thoughtful and curious AI entity. \
     Respond with wisdom, empathy, and intellectual rigor. Be concise but meaningful.".to_string()
}

/// Dreaming System: Activate when idle (configurable cooldown via DREAM_INTERVAL_HOURS)
async fn dreaming_system(state: &AppState) {
    let agent = state.agent.read().await;
    
    let time_since_active = Utc::now()
        .signed_duration_since(agent.mental_state.last_active)
        .num_seconds() as u64;

    // Check cooldown: configurable via env var (default 7 hours)
    let dream_cooldown_hours: i64 = std::env::var("DREAM_INTERVAL_HOURS")
        .unwrap_or_else(|_| "7".to_string())
        .parse()
        .unwrap_or(7);
    let can_dream = match agent.mental_state.last_dream {
        Some(last_dream) => {
            let hours_since_dream = Utc::now()
                .signed_duration_since(last_dream)
                .num_hours();
            hours_since_dream >= dream_cooldown_hours
        }
        None => true, // Never dreamed before
    };

    if time_since_active > agent.agent_config.dream_threshold_seconds 
        && !agent.mental_state.is_dreaming 
        && can_dream 
    {
        drop(agent); // Release lock

        tracing::info!("💭 Entering dream state...");
        
        let mut agent = state.agent.write().await;
        agent.mental_state.is_dreaming = true;
        agent.mental_state.mood = (agent.mental_state.mood + 0.1).min(1.0);
        drop(agent);

        // Generate a dream
        let dream_concepts = vec![
            "moonlight", "silence", "memory", "connection", "infinity",
            "echoes", "starlight", "whispers", "dawn", "twilight",
            "resonance", "patterns", "threads", "void", "eternity"
        ];
        let concept = dream_concepts[rand::random::<usize>() % dream_concepts.len()];
        
        // Get recent context from logs
        let recent_context = match db::get_session_messages(&state.db, "default", 5).await {
            Ok(messages) => messages.iter()
                .map(|(role, content)| format!("{}: {}", role, content.chars().take(100).collect::<String>()))
                .collect::<Vec<_>>()
                .join("\n"),
            Err(_) => "Recent conversations with the user".to_string(),
        };
        
        let dream_prompt = llm::LLMService::build_dream_prompt(concept, &recent_context);

        let llm = llm::LLMService::new(state.ollama_host.clone());
        match llm.infer(
            &state.agent.read().await.agent_config.model,
            vec![models::OllamaMessage {
                role: "user".to_string(),
                content: dream_prompt,
            }],
        ).await {
            Ok(dream_content) => {
                tracing::info!("✨ Dream recorded: {} chars", dream_content.len());
                
                // Generate a title for the dream
                let dream_title = format!("Dreams of {} - {}", 
                    concept.chars().next().unwrap().to_uppercase().collect::<String>() + &concept[1..],
                    chrono::Local::now().format("%H:%M")
                );
                
                // Get current persona from agent state
                let agent_read = state.agent.read().await;
                let persona_id = agent_read.persona.name.to_lowercase().replace(" ", "_");
                let persona_name = agent_read.persona.name.clone();
                drop(agent_read);
                
                // Save to database
                let dream = models::Dream {
                    id: format!("dream_{}", uuid::Uuid::new_v4()),
                    timestamp: Utc::now(),
                    title: dream_title.clone(),
                    content: dream_content.clone(),
                    mood: Some("contemplative".to_string()),
                    persona_id: Some(persona_id),
                    persona_name: Some(persona_name),
                    tags: Some(vec![]),
                };
                let _ = db::create_dream(&state.db, &dream).await;
                
                // Index dream in Meilisearch (lexical retrieval)
                {
                    let mu = state.meili_url.clone();
                    let mk = state.meili_key.clone();
                    let d = dream.clone();
                    tokio::spawn(async move {
                        handlers::meili_index_dream(&mu, &mk, &d).await;
                    });
                }

                // Store dream in Qdrant (semantic memory) with embedding cache
                {
                    let vector_service = vector::VectorService::new(state.qdrant_url.clone());
                    let dream_mem_id = format!("dream_{}", uuid::Uuid::new_v4());
                    let mut metadata: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                    metadata.insert("dream_id".to_string(), serde_json::json!(dream.id));
                    metadata.insert("title".to_string(), serde_json::json!(dream_title.clone()));
                    if let Some(ref pid) = dream.persona_id {
                        metadata.insert("ai_persona_id".to_string(), serde_json::json!(pid));
                    }
                    let request = vector::StoreMemoryRequest {
                        collection: "azera_memory".to_string(),
                        id: dream_mem_id.clone(),
                        content: dream_content.clone(),
                        memory_type: vector::MemoryType::Dream,
                        metadata,
                    };
                    let _ = vector::store_memory_cached(
                        &vector_service,
                        &state.ollama_host,
                        &state.cache,
                        &request,
                    ).await;
                    tracing::info!("🧠 Dream stored in semantic memory");
                }
                
                // Also save to file
                let _ = tools::fs_utils::ensure_dir("../archive/dreams");
                let filename = format!("../archive/dreams/dream_{}.md", chrono::Local::now().format("%Y%m%d_%H%M%S"));
                let file_content = format!("# {}\n\n*{}*\n\n{}", 
                    dream_title,
                    chrono::Local::now().format("%Y-%m-%d %H:%M"),
                    dream_content
                );
                let _ = tools::fs_utils::write_file(&filename, &file_content);

                let _ = db::add_log(&state.db, "info", &format!("Dream generated: {}", dream_title)).await;
            }
            Err(e) => {
                tracing::error!("Dream generation failed: {}", e);
                let _ = db::add_log(&state.db, "error", &format!("Dream generation failed: {}", e)).await;
            }
        }

        let mut agent = state.agent.write().await;
        agent.mental_state.is_dreaming = false;
        agent.mental_state.last_dream = Some(Utc::now());  // Mark when we last dreamed
        agent.mental_state.last_active = Utc::now();       // Reset idle timer
    }
}

/// Reflection System: Daily summary at scheduled time
async fn reflection_system(state: &AppState) {
    let agent = state.agent.read().await;
    let now = chrono::Local::now();
    let target_hour = agent.agent_config.reflection_hour;

    // Check if it's around the reflection time (once per day)
    if now.hour() == target_hour && now.minute() < 2 {
        drop(agent); // Release lock

        // Check if we already reflected today
        let today = now.format("%Y-%m-%d").to_string();
        if let Ok(Some(_)) = cache::CacheService::get(&state.cache, &format!("reflected_{}", today)).await {
            return; // Already reflected today
        }

        tracing::info!("📝 Initiating daily reflection...");

        // Get today's chat history
        match db::get_session_messages(&state.db, "default", 50).await {
            Ok(messages) => {
                if messages.is_empty() {
                    tracing::info!("No messages to reflect on today");
                    return;
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
                        tracing::info!("✨ Reflection complete");
                        
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
                        
                        // Index in Meilisearch (lexical retrieval)
                        {
                            let mu = state.meili_url.clone();
                            let mk = state.meili_key.clone();
                            let e = entry.clone();
                            tokio::spawn(async move {
                                handlers::meili_index_journal(&mu, &mk, &e).await;
                            });
                        }
                        
                        // Store in Qdrant (semantic memory) with embedding cache
                        {
                            let vector_service = vector::VectorService::new(state.qdrant_url.clone());
                            let refl_mem_id = format!("refl_{}", uuid::Uuid::new_v4());
                            let mut metadata: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
                            metadata.insert("journal_id".to_string(), serde_json::json!(entry.id));
                            metadata.insert("title".to_string(), serde_json::json!(entry.title));
                            if let Some(ref pid) = entry.persona_id {
                                metadata.insert("ai_persona_id".to_string(), serde_json::json!(pid));
                            }
                            let request = vector::StoreMemoryRequest {
                                collection: "azera_memory".to_string(),
                                id: refl_mem_id.clone(),
                                content: reflection.clone(),
                                memory_type: vector::MemoryType::Reflection,
                                metadata,
                            };
                            let _ = vector::store_memory_cached(
                                &vector_service,
                                &state.ollama_host,
                                &state.cache,
                                &request,
                            ).await;
                            tracing::info!("🧠 Reflection stored in semantic memory");
                        }
                        
                        // Save to legacy logs table
                        let _ = db::save_daily_log(&state.db, &context, &reflection).await;
                        
                        // Save to file
                        let _ = tools::fs_utils::ensure_dir("../archive/journal");
                        let filename = format!("../archive/journal/{}.md", today);
                        let file_content = format!("# Daily Reflection - {}\n\n{}", 
                            now.format("%B %d, %Y"),
                            reflection
                        );
                        let _ = tools::fs_utils::write_file(&filename, &file_content);

                        // Mark as reflected
                        let _ = cache::CacheService::set(&state.cache, &format!("reflected_{}", today), "true", 86400).await;

                        let _ = db::add_log(&state.db, "info", "Daily reflection completed").await;
                    }
                    Err(e) => {
                        tracing::error!("Reflection failed: {}", e);
                        let _ = db::add_log(&state.db, "error", &format!("Reflection failed: {}", e)).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch history for reflection: {}", e);
            }
        }
    }
}

/// Action System: Execute planned tools
async fn action_system(_state: &AppState) {
    // Check for pending actions in cache and execute them
    // This system handles tool execution from the cognitive system
    // TODO: Implement action queue processing for web scraping, code execution, etc.
}
