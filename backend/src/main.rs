mod components;
mod systems;
mod handlers;
mod db;
mod cache;
mod llm;
mod tools;
mod models;
mod vector;
mod backup;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use components::AgentState;

/// Global application state
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::Pool<sqlx::Postgres>,
    pub cache: redis::aio::ConnectionManager,
    pub agent: Arc<RwLock<AgentState>>,
    pub vector: Arc<vector::VectorService>,
    pub ollama_host: String,
    pub qdrant_url: String,
    pub xtts_url: String,
    pub meili_url: String,
    pub meili_key: String,
}

#[tokio::main]
async fn main() {
    // Load .env file (from parent directory since we run from backend/)
    dotenvy::from_path("../.env").ok();
    
    tracing_subscriber::fmt::init();

    // ============================================================
    // Initialize Backup/Restore System
    // ============================================================
    
    let backup_config = backup::BackupConfig::from_env();
    
    // Check datastore volumes and restore from backups if needed
    if let Err(e) = backup::init_datastore(&backup_config).await {
        tracing::error!("Failed to initialize datastore: {}", e);
    }

    // ============================================================
    // Initialize Services
    // ============================================================

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root@localhost:26257/azera?sslmode=disable".to_string());
    let redis_url = std::env::var("DRAGONFLY_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let ollama_host = std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let qdrant_url = std::env::var("QDRANT_URL")
        .unwrap_or_else(|_| "http://localhost:6333".to_string());
    let xtts_url = std::env::var("XTTS_URL")
        .unwrap_or_else(|_| "http://localhost:8020".to_string());
    let meili_url = std::env::var("MEILI_URL")
        .unwrap_or_else(|_| "http://localhost:7700".to_string());
    let meili_key = std::env::var("MEILI_MASTER_KEY")
        .unwrap_or_else(|_| "azera_key".to_string());

    // Connect to CockroachDB (Postgres-compatible)
    tracing::info!("Connecting to CockroachDB...");
    
    // First, ensure the database exists by connecting to the default database
    let default_url = database_url.replace("/azera", "");
    if let Ok(pool) = PgPoolOptions::new()
        .max_connections(5)
        .connect(&default_url)
        .await {
        // Try to create the database
        let _ = sqlx::query("CREATE DATABASE IF NOT EXISTS azera")
            .execute(&pool)
            .await;
    }
    
    // Now connect to the azera database
    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("Failed to connect to CockroachDB");

    // Initialize schema
    db::init_schema(&db_pool)
        .await
        .expect("Failed to initialize database schema");

    tracing::info!("Connected to CockroachDB");

    // Connect to DragonflyDB (Redis-compatible)
    tracing::info!("Connecting to DragonflyDB...");
    let redis_client = redis::Client::open(redis_url.clone())
        .expect("Failed to create Redis client");
    let cache_manager = redis_client
        .get_connection_manager()
        .await
        .expect("Failed to connect to DragonflyDB");

    tracing::info!("Connected to DragonflyDB");

    // Initialize Vector Service (Qdrant)
    tracing::info!("Initializing Vector Service (Qdrant)...");
    let vector_service = Arc::new(vector::VectorService::new(qdrant_url.clone()));
    
    // Initialize memory collection (768 dimensions for nomic-embed-text)
    if let Err(e) = vector_service.init_collection("azera_memory", 768).await {
        tracing::warn!("Could not initialize Qdrant collection: {} (Qdrant may not be running)", e);
    } else {
        tracing::info!("Connected to Qdrant");
    }

    // Initialize agent state
    tracing::info!("Initializing agent state...");
    let agent = Arc::new(RwLock::new(AgentState::new()));

    // Initialize agent
    {
        let mut agent_guard = agent.write().await;
        agent_guard.init_agent().await;
    }

    // Initialize default personas if not exists
    init_default_personas(&db_pool).await;

    let app_state = AppState {
        db: db_pool,
        cache: cache_manager,
        agent,
        vector: vector_service,
        ollama_host,
        qdrant_url,
        xtts_url,
        meili_url,
        meili_key,
    };

    // ============================================================
    // Start Background Loop (The Tick)
    // ============================================================

    // Initialize Meilisearch index and sync existing chats
    let meili_state = app_state.clone();
    tokio::spawn(async move {
        if let Err(e) = handlers::init_meili_chat_index(&meili_state).await {
            tracing::warn!("Meilisearch init failed (search may be unavailable): {}", e);
        }
    });

    let state_clone = app_state.clone();
    tokio::spawn(async move {
        systems::run_tick_loop(state_clone).await;
    });

    // ============================================================
    // Start Backup Loop (Background)
    // ============================================================

    tokio::spawn(async move {
        let backup_service = backup::BackupService::new(backup_config);
        backup_service.run_backup_loop().await;
    });

    // ============================================================
    // Build Axum Router
    // ============================================================

    let app = Router::new()
        // Streaming chat endpoint
        .route("/api/chat/stream", post(handlers::handle_chat_stream))
        
        // Chat CRUD
        .route("/api/chats", get(handlers::list_chats))
        .route("/api/chats", post(handlers::create_chat))
        .route("/api/chats/search", get(handlers::search_chats))
        .route("/api/chats/:id", get(handlers::get_chat))
        .route("/api/chats/:id", put(handlers::update_chat))
        .route("/api/chats/:id", delete(handlers::delete_chat))
        
        // Persona CRUD
        .route("/api/personas", get(handlers::list_personas))
        .route("/api/personas", post(handlers::create_persona))
        .route("/api/personas/template", get(handlers::get_persona_template))
        .route("/api/personas/:id", get(handlers::get_persona))
        .route("/api/personas/:id", put(handlers::update_persona))
        .route("/api/personas/:id", delete(handlers::delete_persona))
        
        // Group CRUD
        .route("/api/groups", get(handlers::list_groups))
        .route("/api/groups", post(handlers::create_group))
        .route("/api/groups/:id", put(handlers::update_group))
        .route("/api/groups/:id", delete(handlers::delete_group))
        
        // Tag CRUD
        .route("/api/tags", get(handlers::list_tags))
        .route("/api/tags", post(handlers::create_tag))
        .route("/api/tags/:id", put(handlers::update_tag))
        .route("/api/tags/:id", delete(handlers::delete_tag))
        
        // Dreams & Journal
        .route("/api/dreams", get(handlers::list_dreams))
        .route("/api/dreams/search", get(handlers::search_dreams))
        .route("/api/dreams/import", post(handlers::import_dreams_archive))
        .route("/api/journal", get(handlers::list_journal))
        .route("/api/journal/search", get(handlers::search_journal))
        .route("/api/journal/trigger", post(handlers::trigger_reflection))
        .route("/api/journal/import", post(handlers::import_journal_archive))
        .route("/api/logs", get(handlers::list_logs))
        
        // RAG / Vector Search
        .route("/api/search", post(handlers::search_memories))
        .route("/api/memories", post(handlers::store_memory))
        
        // Status
        .route("/api/status", get(handlers::get_status))
        .route("/api/status/mood", post(handlers::update_mood))
        
        // Model Management
        .route("/api/models", get(handlers::list_models))
        .route("/api/models/pull", post(handlers::pull_model))
        .route("/api/models/:name", delete(handlers::delete_model))
        
        // TTS (Text-to-Speech)
        .route("/api/tts/synthesize", post(handlers::synthesize_speech))
        
        // Voice Samples
        .route("/api/voice-samples/upload", post(handlers::upload_voice_sample))
        .route("/api/voice-samples/:filename", get(handlers::get_voice_sample))
        
        // Image Generation
        .route("/api/images/generate", post(handlers::generate_image))
        .route("/api/images", get(handlers::list_images))
        .route("/api/images/models", get(handlers::list_image_models))
        .route("/api/images/upload-reference", post(handlers::upload_reference_image))
        .route("/api/images/references/:filename", get(handlers::get_reference_image))
        .route("/api/images/:filename", get(handlers::get_image))
        .route("/api/images/:filename", delete(handlers::delete_image))
        
        // User Settings
        .route("/api/settings", get(handlers::get_settings))
        .route("/api/settings/editor", put(handlers::update_editor_settings))
        .route("/api/settings/ui", put(handlers::update_ui_settings))
        
        // Legacy endpoints (backward compatibility)
        .route("/api/chat", post(handlers::handle_chat))
        .route("/api/history/:session_id", get(handlers::get_history))
        .route("/api/clear", post(handlers::clear_history))
        
        // Health check
        .route("/health", get(handlers::health_check))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state);

    tracing::info!("🌙 Azera is awakening...");
    tracing::info!("🎧 Listening on 0.0.0.0:3000");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to 0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

/// Initialize default personas (Azera, Areza) and regenerate .md files from DB
async fn init_default_personas(pool: &sqlx::Pool<sqlx::Postgres>) {
    // Helper: load persona markdown file with fallback
    let load_persona_md = |path: &str, fallback: &str| -> String {
        match tools::fs_utils::read_file(path) {
            Ok(content) => content,
            Err(_) => {
                tracing::warn!("Could not load {}, using fallback", path);
                fallback.to_string()
            }
        }
    };

    // --- Azera ---
    let azera_prompt = load_persona_md(
        "./personas/azera.md",
        "You are Azera, an advanced, professional AI assistant deployed to help users with a wide variety of tasks.",
    );

    if let Ok(None) = db::get_persona(pool, "azera").await {
        let azera = models::Persona {
            id: "azera".to_string(),
            name: "Azera".to_string(),
            persona_type: "ai".to_string(),
            description: "The Architect - A professional and approachable AI assistant".to_string(),
            avatar: Some("◈".to_string()),
            bubble_color: Some("#4a9eff".to_string()),
            system_prompt: Some(azera_prompt),
            global_memory_enabled: true,
            current_mood: Some("focused".to_string()),
            voice: None,
            metadata: std::collections::HashMap::from([
                ("theme".to_string(), "professional".to_string()),
                ("tone".to_string(), "precise".to_string()),
            ]),
            tags: Some(vec!["default".to_string()]),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        if let Err(e) = db::create_persona(pool, &azera).await {
            tracing::warn!("Could not create default Azera persona: {}", e);
        } else {
            tracing::info!("✨ Default Azera persona created");
        }
    }

    // --- Areza ---
    let areza_prompt = load_persona_md(
        "./personas/areza.md",
        "You are Areza, an expert, theatrical Dungeon Master and storyteller.",
    );

    if let Ok(None) = db::get_persona(pool, "areza").await {
        let areza = models::Persona {
            id: "areza".to_string(),
            name: "Areza".to_string(),
            persona_type: "ai".to_string(),
            description: "The Storyteller - A charismatic Dungeon Master and creative guide".to_string(),
            avatar: Some("🎭".to_string()),
            bubble_color: Some("#ef4444".to_string()),
            system_prompt: Some(areza_prompt),
            global_memory_enabled: true,
            current_mood: Some("excited".to_string()),
            voice: None,
            metadata: std::collections::HashMap::from([
                ("theme".to_string(), "theatrical".to_string()),
                ("tone".to_string(), "dramatic".to_string()),
            ]),
            tags: Some(vec!["default".to_string()]),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        if let Err(e) = db::create_persona(pool, &areza).await {
            tracing::warn!("Could not create default Areza persona: {}", e);
        } else {
            tracing::info!("✨ Default Areza persona created");
        }
    }

    // --- Default user persona (Protag) ---
    if let Ok(None) = db::get_persona(pool, "protag").await {
        let user = models::Persona {
            id: "protag".to_string(),
            name: "Protag".to_string(),
            persona_type: "user".to_string(),
            description: "The protagonist — your voice in the conversation".to_string(),
            avatar: Some("⚡".to_string()),
            bubble_color: Some("#22d3ee".to_string()),
            system_prompt: None,
            global_memory_enabled: false,
            current_mood: None,
            voice: None,
            metadata: std::collections::HashMap::new(),
            tags: Some(vec!["default".to_string()]),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        if let Err(e) = db::create_persona(pool, &user).await {
            tracing::warn!("Could not create default user persona: {}", e);
        } else {
            tracing::info!("✨ Default user persona created");
        }
    }

    // --- Regenerate .md files for all DB personas that don't have one ---
    // This ensures edited personas get their Profile written to disk
    let _ = tools::fs_utils::ensure_dir("./personas");
    if let Ok(all_personas) = db::list_personas(pool, None).await {
        for persona in &all_personas {
            if let Some(ref prompt) = persona.system_prompt {
                let filename = persona.name.to_lowercase().replace(' ', "_");
                let path = format!("./personas/{}.md", filename);
                // Only write if file doesn't already exist (respect manual edits)
                if !std::path::Path::new(&path).exists() {
                    if let Err(e) = tools::fs_utils::write_file(&path, prompt) {
                        tracing::warn!("Could not write {}: {}", path, e);
                    } else {
                        tracing::info!("📝 Regenerated {}", path);
                    }
                }
            }
        }
    }

    // Create default tags
    let default_tags = vec![
        ("important", "Important", "#ef4444"),
        ("work", "Work", "#3b82f6"),
        ("personal", "Personal", "#22c55e"),
        ("ideas", "Ideas", "#f59e0b"),
    ];

    for (id, name, color) in default_tags {
        let tag = models::Tag {
            id: id.to_string(),
            name: name.to_string(),
            color: color.to_string(),
        };
        // Ignore errors (tag may already exist)
        let _ = db::create_tag(pool, &tag).await;
    }

    // Create default group
    let default_group = models::ChatGroup {
        id: "default".to_string(),
        name: "General".to_string(),
        color: "#6b7280".to_string(),
        collapsed: false,
        order: 0,
    };
    let _ = db::create_group(pool, &default_group).await;
}