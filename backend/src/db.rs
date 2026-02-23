use sqlx::{Pool, Postgres, Row};
use anyhow::Result;
use crate::models::*;
use chrono::Utc;

/// Initialize database schema
pub async fn init_schema(pool: &Pool<Postgres>) -> Result<()> {
    // ============================================================
    // Personas table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS personas (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            persona_type TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            avatar TEXT,
            bubble_color TEXT,
            system_prompt TEXT,
            global_memory_enabled BOOLEAN DEFAULT TRUE,
            metadata JSONB DEFAULT '{}',
            tags JSONB DEFAULT '[]',
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;
    
    // Add global_memory_enabled column if it doesn't exist (for existing databases)
    let _ = sqlx::query(
        "ALTER TABLE personas ADD COLUMN IF NOT EXISTS global_memory_enabled BOOLEAN DEFAULT TRUE"
    )
    .execute(pool)
    .await;
    
    // Add current_mood column if it doesn't exist
    let _ = sqlx::query(
        "ALTER TABLE personas ADD COLUMN IF NOT EXISTS current_mood TEXT"
    )
    .execute(pool)
    .await;
    
    // Add voice column for TTS settings (JSONB)
    let _ = sqlx::query(
        "ALTER TABLE personas ADD COLUMN IF NOT EXISTS voice JSONB"
    )
    .execute(pool)
    .await;

    // ============================================================
    // Chat groups table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            color TEXT NOT NULL DEFAULT '#6b7280',
            collapsed BOOLEAN DEFAULT FALSE,
            sort_order INT DEFAULT 0,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ============================================================
    // Tags table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            color TEXT NOT NULL DEFAULT '#6b7280',
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ============================================================
    // Chats table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chats (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            current_branch_id TEXT NOT NULL,
            group_id TEXT REFERENCES chat_groups(id) ON DELETE SET NULL,
            tags JSONB DEFAULT '[]',
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_chats_group ON chats(group_id)")
        .execute(pool)
        .await?;

    // ============================================================
    // Chat branches table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_branches (
            id TEXT PRIMARY KEY,
            chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            parent_branch_id TEXT REFERENCES chat_branches(id) ON DELETE SET NULL,
            fork_point_message_id TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_branches_chat ON chat_branches(chat_id)")
        .execute(pool)
        .await?;

    // ============================================================
    // Chat messages table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY,
            branch_id TEXT NOT NULL REFERENCES chat_branches(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            user_persona_id TEXT,
            ai_persona_id TEXT,
            model TEXT,
            mood TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_branch ON chat_messages(branch_id, created_at)")
        .execute(pool)
        .await?;

    // ============================================================
    // Dreams table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS dreams (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            mood TEXT,
            persona_id TEXT,
            persona_name TEXT,
            tags TEXT[],
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Add persona columns if they don't exist (migration)
    let _ = sqlx::query("ALTER TABLE dreams ADD COLUMN IF NOT EXISTS persona_id TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE dreams ADD COLUMN IF NOT EXISTS persona_name TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE dreams ADD COLUMN IF NOT EXISTS tags TEXT[]")
        .execute(pool).await;

    // ============================================================
    // Journal entries table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS journal_entries (
            id TEXT PRIMARY KEY,
            date TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            mood TEXT,
            persona_id TEXT,
            persona_name TEXT,
            tags TEXT[],
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Add persona columns if they don't exist (migration)
    let _ = sqlx::query("ALTER TABLE journal_entries ADD COLUMN IF NOT EXISTS persona_id TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE journal_entries ADD COLUMN IF NOT EXISTS persona_name TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE journal_entries ADD COLUMN IF NOT EXISTS tags TEXT[]")
        .execute(pool).await;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_journal_date ON journal_entries(date)")
        .execute(pool)
        .await?;

    // ============================================================
    // System logs table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS system_logs (
            id TEXT PRIMARY KEY,
            level TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ============================================================
    // User settings table (for editor config, UI preferences, etc.)
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_settings (
            id TEXT PRIMARY KEY DEFAULT 'default',
            editor_settings JSONB DEFAULT '{}',
            ui_settings JSONB DEFAULT '{}',
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ============================================================
    // Configuration table (keep existing)
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ============================================================
    // Chat history table (legacy - for migration)
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_history (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_session ON chat_history(session_id, created_at)")
        .execute(pool)
        .await?;

    // ============================================================
    // Logs table (legacy - keep for now)
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS logs (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            date DATE NOT NULL,
            content TEXT,
            summary TEXT,
            embeddings_stored BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_date ON logs(date)")
        .execute(pool)
        .await?;

    // ============================================================
    // Embeddings metadata table
    // ============================================================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS embeddings (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            embedding_id TEXT NOT NULL UNIQUE,
            source TEXT,
            metadata JSONB,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Database schema initialized");
    Ok(())
}

// ============================================================
// Persona CRUD
// ============================================================

pub async fn create_persona(pool: &Pool<Postgres>, persona: &Persona) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO personas (id, name, persona_type, description, avatar, bubble_color, system_prompt, global_memory_enabled, metadata, tags, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(&persona.id)
    .bind(&persona.name)
    .bind(&persona.persona_type)
    .bind(&persona.description)
    .bind(&persona.avatar)
    .bind(&persona.bubble_color)
    .bind(&persona.system_prompt)
    .bind(persona.global_memory_enabled)
    .bind(serde_json::to_value(&persona.metadata)?)
    .bind(serde_json::to_value(&persona.tags)?)
    .bind(persona.created_at)
    .bind(persona.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_persona(pool: &Pool<Postgres>, id: &str) -> Result<Option<Persona>> {
    let row = sqlx::query(
        "SELECT id, name, persona_type, description, avatar, bubble_color, system_prompt, global_memory_enabled, voice, metadata, tags, created_at, updated_at FROM personas WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(Persona {
            id: r.get("id"),
            name: r.get("name"),
            persona_type: r.get("persona_type"),
            description: r.get("description"),
            avatar: r.get("avatar"),
            bubble_color: r.get("bubble_color"),
            system_prompt: r.get("system_prompt"),
            global_memory_enabled: r.try_get("global_memory_enabled").unwrap_or(true),
            current_mood: r.try_get("current_mood").ok().flatten(),
            voice: r.try_get::<Option<serde_json::Value>, _>("voice").ok().flatten().and_then(|v| serde_json::from_value(v).ok()),
            metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
            tags: serde_json::from_value(r.get("tags")).ok(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })),
        None => Ok(None),
    }
}

pub async fn list_personas(pool: &Pool<Postgres>, persona_type: Option<&str>) -> Result<Vec<Persona>> {
    let query = if let Some(pt) = persona_type {
        sqlx::query(
            "SELECT id, name, persona_type, description, avatar, bubble_color, system_prompt, global_memory_enabled, current_mood, voice, metadata, tags, created_at, updated_at FROM personas WHERE persona_type = $1 ORDER BY name"
        ).bind(pt)
    } else {
        sqlx::query(
            "SELECT id, name, persona_type, description, avatar, bubble_color, system_prompt, global_memory_enabled, current_mood, voice, metadata, tags, created_at, updated_at FROM personas ORDER BY name"
        )
    };

    let rows = query.fetch_all(pool).await?;
    
    Ok(rows.iter().map(|r| Persona {
        id: r.get("id"),
        name: r.get("name"),
        persona_type: r.get("persona_type"),
        description: r.get("description"),
        avatar: r.get("avatar"),
        bubble_color: r.get("bubble_color"),
        system_prompt: r.get("system_prompt"),
        global_memory_enabled: r.try_get("global_memory_enabled").unwrap_or(true),
        current_mood: r.try_get("current_mood").ok().flatten(),
        voice: r.try_get::<Option<serde_json::Value>, _>("voice").ok().flatten().and_then(|v| serde_json::from_value(v).ok()),
        metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
        tags: serde_json::from_value(r.get("tags")).ok(),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }).collect())
}

pub async fn update_persona(pool: &Pool<Postgres>, id: &str, req: &UpdatePersonaRequest) -> Result<()> {
    // Build dynamic update query
    let mut updates = vec!["updated_at = NOW()".to_string()];
    let mut param_count = 1;
    
    if req.name.is_some() { updates.push(format!("name = ${}", { param_count += 1; param_count })); }
    if req.description.is_some() { updates.push(format!("description = ${}", { param_count += 1; param_count })); }
    if req.avatar.is_some() { updates.push(format!("avatar = ${}", { param_count += 1; param_count })); }
    if req.bubble_color.is_some() { updates.push(format!("bubble_color = ${}", { param_count += 1; param_count })); }
    if req.system_prompt.is_some() { updates.push(format!("system_prompt = ${}", { param_count += 1; param_count })); }
    if req.global_memory_enabled.is_some() { updates.push(format!("global_memory_enabled = ${}", { param_count += 1; param_count })); }
    if req.current_mood.is_some() { updates.push(format!("current_mood = ${}", { param_count += 1; param_count })); }
    if req.voice.is_some() { updates.push(format!("voice = ${}", { param_count += 1; param_count })); }
    if req.metadata.is_some() { updates.push(format!("metadata = ${}", { param_count += 1; param_count })); }
    if req.tags.is_some() { updates.push(format!("tags = ${}", { param_count += 1; param_count })); }
    
    let query_str = format!("UPDATE personas SET {} WHERE id = $1", updates.join(", "));
    let mut query = sqlx::query(&query_str).bind(id);
    
    if let Some(ref name) = req.name { query = query.bind(name); }
    if let Some(ref description) = req.description { query = query.bind(description); }
    if let Some(ref avatar) = req.avatar { query = query.bind(avatar); }
    if let Some(ref bubble_color) = req.bubble_color { query = query.bind(bubble_color); }
    if let Some(ref system_prompt) = req.system_prompt { query = query.bind(system_prompt); }
    if let Some(global_memory_enabled) = req.global_memory_enabled { query = query.bind(global_memory_enabled); }
    if let Some(ref current_mood) = req.current_mood { query = query.bind(current_mood); }
    if let Some(ref voice) = req.voice { query = query.bind(serde_json::to_value(voice)?); }
    if let Some(ref metadata) = req.metadata { query = query.bind(serde_json::to_value(metadata)?); }
    if let Some(ref tags) = req.tags { query = query.bind(serde_json::to_value(tags)?); }
    
    query.execute(pool).await?;
    Ok(())
}

pub async fn delete_persona(pool: &Pool<Postgres>, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM personas WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ============================================================
// Chat CRUD
// ============================================================

pub async fn create_chat(pool: &Pool<Postgres>, chat: &Chat) -> Result<()> {
    // Start transaction
    let mut tx = pool.begin().await?;
    
    // Insert chat
    sqlx::query(
        r#"
        INSERT INTO chats (id, title, current_branch_id, group_id, tags, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&chat.id)
    .bind(&chat.title)
    .bind(&chat.current_branch_id)
    .bind(&chat.group_id)
    .bind(serde_json::to_value(&chat.tags)?)
    .bind(chat.created_at)
    .execute(&mut *tx)
    .await?;
    
    // Insert branches
    for branch in &chat.branches {
        sqlx::query(
            r#"
            INSERT INTO chat_branches (id, chat_id, name, parent_branch_id, fork_point_message_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&branch.id)
        .bind(&chat.id)
        .bind(&branch.name)
        .bind(&branch.parent_branch_id)
        .bind(&branch.fork_point_message_id)
        .bind(branch.created_at)
        .execute(&mut *tx)
        .await?;
        
        // Insert messages
        for msg in &branch.messages {
            sqlx::query(
                r#"
                INSERT INTO chat_messages (id, branch_id, role, content, user_persona_id, ai_persona_id, model, mood, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(&msg.id)
            .bind(&branch.id)
            .bind(&msg.role)
            .bind(&msg.content)
            .bind(&msg.user_persona)
            .bind(&msg.ai_persona)
            .bind(&msg.model)
            .bind(&msg.mood)
            .bind(msg.timestamp.unwrap_or_else(Utc::now))
            .execute(&mut *tx)
            .await?;
        }
    }
    
    tx.commit().await?;
    Ok(())
}

pub async fn get_chat(pool: &Pool<Postgres>, id: &str) -> Result<Option<Chat>> {
    let chat_row = sqlx::query(
        "SELECT id, title, current_branch_id, group_id, tags, created_at FROM chats WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    match chat_row {
        Some(c) => {
            let branches = get_chat_branches(pool, id).await?;
            Ok(Some(Chat {
                id: c.get("id"),
                title: c.get("title"),
                current_branch_id: c.get("current_branch_id"),
                group_id: c.get("group_id"),
                tags: serde_json::from_value(c.get("tags")).ok(),
                created_at: c.get("created_at"),
                branches,
            }))
        }
        None => Ok(None),
    }
}

async fn get_chat_branches(pool: &Pool<Postgres>, chat_id: &str) -> Result<Vec<ChatBranch>> {
    let branch_rows = sqlx::query(
        "SELECT id, name, parent_branch_id, fork_point_message_id, created_at FROM chat_branches WHERE chat_id = $1 ORDER BY created_at"
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    
    let mut branches = Vec::new();
    for b in branch_rows {
        let branch_id: String = b.get("id");
        let messages = get_branch_messages(pool, &branch_id).await?;
        
        branches.push(ChatBranch {
            id: branch_id,
            name: b.get("name"),
            parent_branch_id: b.get("parent_branch_id"),
            fork_point_message_id: b.get("fork_point_message_id"),
            messages,
            created_at: b.get("created_at"),
        });
    }
    
    Ok(branches)
}

async fn get_branch_messages(pool: &Pool<Postgres>, branch_id: &str) -> Result<Vec<ChatMessage>> {
    let rows = sqlx::query(
        "SELECT id, role, content, user_persona_id, ai_persona_id, model, mood, created_at FROM chat_messages WHERE branch_id = $1 ORDER BY created_at"
    )
    .bind(branch_id)
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| ChatMessage {
        id: r.get("id"),
        role: r.get("role"),
        content: r.get("content"),
        user_persona: r.get("user_persona_id"),
        ai_persona: r.get("ai_persona_id"),
        model: r.get("model"),
        mood: r.get("mood"),
        timestamp: Some(r.get("created_at")),
    }).collect())
}

pub async fn list_chats(pool: &Pool<Postgres>) -> Result<Vec<Chat>> {
    let chat_rows = sqlx::query(
        "SELECT id, title, current_branch_id, group_id, tags, created_at FROM chats ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    
    let mut chats = Vec::new();
    for c in chat_rows {
        let chat_id: String = c.get("id");
        let branches = get_chat_branches(pool, &chat_id).await?;
        
        chats.push(Chat {
            id: chat_id,
            title: c.get("title"),
            current_branch_id: c.get("current_branch_id"),
            group_id: c.get("group_id"),
            tags: serde_json::from_value(c.get("tags")).ok(),
            created_at: c.get("created_at"),
            branches,
        });
    }
    
    Ok(chats)
}

pub async fn update_chat(pool: &Pool<Postgres>, id: &str, req: &UpdateChatRequest) -> Result<()> {
    let mut updates = vec!["updated_at = NOW()".to_string()];
    let mut param_count = 1;
    
    if req.title.is_some() { updates.push(format!("title = ${}", { param_count += 1; param_count })); }
    if req.group_id.is_some() { updates.push(format!("group_id = ${}", { param_count += 1; param_count })); }
    if req.tags.is_some() { updates.push(format!("tags = ${}", { param_count += 1; param_count })); }
    if req.current_branch_id.is_some() { updates.push(format!("current_branch_id = ${}", { param_count += 1; param_count })); }
    
    let query_str = format!("UPDATE chats SET {} WHERE id = $1", updates.join(", "));
    let mut query = sqlx::query(&query_str).bind(id);
    
    if let Some(ref title) = req.title { query = query.bind(title); }
    if let Some(ref group_id) = req.group_id { query = query.bind(group_id); }
    if let Some(ref tags) = req.tags { query = query.bind(serde_json::to_value(tags)?); }
    if let Some(ref current_branch_id) = req.current_branch_id { query = query.bind(current_branch_id); }
    
    query.execute(pool).await?;
    Ok(())
}

pub async fn delete_chat(pool: &Pool<Postgres>, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM chats WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Ensure chat and branch exist in the database (creates if missing)
/// This handles the case where frontend creates chats locally
pub async fn ensure_chat_and_branch(
    pool: &Pool<Postgres>, 
    chat_id: &str, 
    branch_id: &str,
    chat_title: Option<&str>,
) -> Result<()> {
    // Check if chat exists
    let chat_exists = sqlx::query("SELECT 1 FROM chats WHERE id = $1")
        .bind(chat_id)
        .fetch_optional(pool)
        .await?
        .is_some();
    
    if !chat_exists {
        // Create the chat
        sqlx::query(
            r#"
            INSERT INTO chats (id, title, current_branch_id, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(chat_id)
        .bind(chat_title.unwrap_or("New Chat"))
        .bind(branch_id)
        .execute(pool)
        .await?;
        
        tracing::info!("Created chat {} in database", chat_id);
    }
    
    // Check if branch exists
    let branch_exists = sqlx::query("SELECT 1 FROM chat_branches WHERE id = $1")
        .bind(branch_id)
        .fetch_optional(pool)
        .await?
        .is_some();
    
    if !branch_exists {
        // Create the branch
        sqlx::query(
            r#"
            INSERT INTO chat_branches (id, chat_id, name, created_at)
            VALUES ($1, $2, 'Main', NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(branch_id)
        .bind(chat_id)
        .execute(pool)
        .await?;
        
        tracing::info!("Created branch {} in database", branch_id);
    }
    
    Ok(())
}

pub async fn add_message_to_branch(pool: &Pool<Postgres>, msg: &ChatMessage, branch_id: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO chat_messages (id, branch_id, role, content, user_persona_id, ai_persona_id, model, mood, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(&msg.id)
    .bind(branch_id)
    .bind(&msg.role)
    .bind(&msg.content)
    .bind(&msg.user_persona)
    .bind(&msg.ai_persona)
    .bind(&msg.model)
    .bind(&msg.mood)
    .bind(msg.timestamp.unwrap_or_else(Utc::now))
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// Groups CRUD
// ============================================================

pub async fn create_group(pool: &Pool<Postgres>, group: &ChatGroup) -> Result<()> {
    sqlx::query(
        "INSERT INTO chat_groups (id, name, color, collapsed, sort_order) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(&group.id)
    .bind(&group.name)
    .bind(&group.color)
    .bind(group.collapsed)
    .bind(group.order)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_groups(pool: &Pool<Postgres>) -> Result<Vec<ChatGroup>> {
    let rows = sqlx::query(
        "SELECT id, name, color, collapsed, sort_order FROM chat_groups ORDER BY sort_order"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| ChatGroup {
        id: r.get("id"),
        name: r.get("name"),
        color: r.get("color"),
        collapsed: r.get("collapsed"),
        order: r.get("sort_order"),
    }).collect())
}

pub async fn update_group(pool: &Pool<Postgres>, id: &str, req: &UpdateGroupRequest) -> Result<()> {
    let mut updates = Vec::new();
    let mut param_count = 1;
    
    if req.name.is_some() { updates.push(format!("name = ${}", { param_count += 1; param_count })); }
    if req.color.is_some() { updates.push(format!("color = ${}", { param_count += 1; param_count })); }
    if req.collapsed.is_some() { updates.push(format!("collapsed = ${}", { param_count += 1; param_count })); }
    if req.order.is_some() { updates.push(format!("sort_order = ${}", { param_count += 1; param_count })); }
    
    if updates.is_empty() { return Ok(()); }
    
    let query_str = format!("UPDATE chat_groups SET {} WHERE id = $1", updates.join(", "));
    let mut query = sqlx::query(&query_str).bind(id);
    
    if let Some(ref name) = req.name { query = query.bind(name); }
    if let Some(ref color) = req.color { query = query.bind(color); }
    if let Some(collapsed) = req.collapsed { query = query.bind(collapsed); }
    if let Some(order) = req.order { query = query.bind(order); }
    
    query.execute(pool).await?;
    Ok(())
}

pub async fn delete_group(pool: &Pool<Postgres>, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM chat_groups WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ============================================================
// Tags CRUD
// ============================================================

pub async fn create_tag(pool: &Pool<Postgres>, tag: &Tag) -> Result<()> {
    sqlx::query(
        "INSERT INTO tags (id, name, color) VALUES ($1, $2, $3)"
    )
    .bind(&tag.id)
    .bind(&tag.name)
    .bind(&tag.color)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_tags(pool: &Pool<Postgres>) -> Result<Vec<Tag>> {
    let rows = sqlx::query("SELECT id, name, color FROM tags ORDER BY name")
        .fetch_all(pool)
        .await?;
    
    Ok(rows.iter().map(|r| Tag {
        id: r.get("id"),
        name: r.get("name"),
        color: r.get("color"),
    }).collect())
}

pub async fn update_tag(pool: &Pool<Postgres>, id: &str, req: &UpdateTagRequest) -> Result<()> {
    let mut updates = Vec::new();
    let mut param_count = 1;
    
    if req.name.is_some() { updates.push(format!("name = ${}", { param_count += 1; param_count })); }
    if req.color.is_some() { updates.push(format!("color = ${}", { param_count += 1; param_count })); }
    
    if updates.is_empty() { return Ok(()); }
    
    let query_str = format!("UPDATE tags SET {} WHERE id = $1", updates.join(", "));
    let mut query = sqlx::query(&query_str).bind(id);
    
    if let Some(ref name) = req.name { query = query.bind(name); }
    if let Some(ref color) = req.color { query = query.bind(color); }
    
    query.execute(pool).await?;
    Ok(())
}

pub async fn delete_tag(pool: &Pool<Postgres>, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ============================================================
// Dreams CRUD
// ============================================================

pub async fn create_dream(pool: &Pool<Postgres>, dream: &Dream) -> Result<()> {
    sqlx::query(
        "INSERT INTO dreams (id, title, content, mood, persona_id, persona_name, tags, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(&dream.id)
    .bind(&dream.title)
    .bind(&dream.content)
    .bind(&dream.mood)
    .bind(&dream.persona_id)
    .bind(&dream.persona_name)
    .bind(&dream.tags)
    .bind(dream.timestamp)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_dreams(pool: &Pool<Postgres>, limit: i32) -> Result<Vec<Dream>> {
    let rows = sqlx::query(
        "SELECT id, title, content, mood, persona_id, persona_name, tags, created_at FROM dreams ORDER BY created_at DESC LIMIT $1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| Dream {
        id: r.get("id"),
        title: r.get("title"),
        content: r.get("content"),
        mood: r.get("mood"),
        persona_id: r.get("persona_id"),
        persona_name: r.get("persona_name"),
        tags: r.get("tags"),
        timestamp: r.get("created_at"),
    }).collect())
}

// ============================================================
// Journal CRUD
// ============================================================

pub async fn create_journal_entry(pool: &Pool<Postgres>, entry: &JournalEntry) -> Result<()> {
    sqlx::query(
        "INSERT INTO journal_entries (id, date, title, content, mood, persona_id, persona_name, tags, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(&entry.id)
    .bind(&entry.date)
    .bind(&entry.title)
    .bind(&entry.content)
    .bind(&entry.mood)
    .bind(&entry.persona_id)
    .bind(&entry.persona_name)
    .bind(&entry.tags)
    .bind(entry.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_journal_entries(pool: &Pool<Postgres>, limit: i32) -> Result<Vec<JournalEntry>> {
    let rows = sqlx::query(
        "SELECT id, date, title, content, mood, persona_id, persona_name, tags, created_at FROM journal_entries ORDER BY date DESC LIMIT $1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| JournalEntry {
        id: r.get("id"),
        date: r.get("date"),
        title: r.get("title"),
        content: r.get("content"),
        mood: r.get("mood"),
        persona_id: r.get("persona_id"),
        persona_name: r.get("persona_name"),
        tags: r.get("tags"),
        created_at: r.get("created_at"),
    }).collect())
}

// ============================================================
// System Logs
// ============================================================

pub async fn add_log(pool: &Pool<Postgres>, level: &str, message: &str) -> Result<()> {
    let id = format!("log_{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO system_logs (id, level, message) VALUES ($1, $2, $3)"
    )
    .bind(&id)
    .bind(level)
    .bind(message)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_logs(pool: &Pool<Postgres>, limit: i32) -> Result<Vec<LogEntry>> {
    let rows = sqlx::query(
        "SELECT id, level, message, created_at FROM system_logs ORDER BY created_at DESC LIMIT $1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    Ok(rows.iter().map(|r| LogEntry {
        id: r.get("id"),
        level: r.get("level"),
        message: r.get("message"),
        timestamp: r.get("created_at"),
    }).collect())
}

// ============================================================
// Legacy functions (keep for backward compatibility)
// ============================================================

/// Save a chat message (legacy)
pub async fn save_message(
    pool: &Pool<Postgres>,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO chat_history (session_id, role, content) VALUES ($1, $2, $3)",
    )
    .bind(session_id)
    .bind(role)
    .bind(content)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get chat history for a session (legacy)
pub async fn get_session_messages(
    pool: &Pool<Postgres>,
    session_id: &str,
    limit: i32,
) -> Result<Vec<(String, String)>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT role, content FROM chat_history WHERE session_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().rev().collect())
}

/// Save daily reflection/log (legacy)
pub async fn save_daily_log(
    pool: &Pool<Postgres>,
    content: &str,
    summary: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO logs (date, content, summary) VALUES ($1, $2, $3)",
    )
    .bind(chrono::Local::now().date_naive())
    .bind(content)
    .bind(summary)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get configuration value
/// TODO(future): Used by settings system for persistent key-value config
#[allow(dead_code)]
pub async fn get_config(pool: &Pool<Postgres>, key: &str) -> Result<Option<String>> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM config WHERE key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(v,)| v))
}

/// Set configuration value
/// TODO(future): Used by settings system for persistent key-value config
#[allow(dead_code)]
pub async fn set_config(pool: &Pool<Postgres>, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO config (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $2, updated_at = NOW()",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

// ============================================================
// User Settings
// ============================================================

/// Get user settings (editor_settings and ui_settings as JSON)
pub async fn get_user_settings(pool: &Pool<Postgres>) -> Result<Option<(serde_json::Value, serde_json::Value)>> {
    let row = sqlx::query_as::<_, (serde_json::Value, serde_json::Value)>(
        "SELECT editor_settings, ui_settings FROM user_settings WHERE id = 'default'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Save editor settings
pub async fn save_editor_settings(pool: &Pool<Postgres>, settings: &serde_json::Value) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_settings (id, editor_settings, updated_at) 
        VALUES ('default', $1, NOW()) 
        ON CONFLICT(id) DO UPDATE SET editor_settings = $1, updated_at = NOW()
        "#,
    )
    .bind(settings)
    .execute(pool)
    .await?;
    Ok(())
}

/// Save UI settings
pub async fn save_ui_settings(pool: &Pool<Postgres>, settings: &serde_json::Value) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_settings (id, ui_settings, updated_at) 
        VALUES ('default', $1, NOW()) 
        ON CONFLICT(id) DO UPDATE SET ui_settings = $1, updated_at = NOW()
        "#,
    )
    .bind(settings)
    .execute(pool)
    .await?;
    Ok(())
}
