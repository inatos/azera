use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Persona component (Entity's identity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub name: String,
    pub core_directive: String,
    pub role: String,
}

impl Default for Persona {
    fn default() -> Self {
        Self {
            name: "Azera".to_string(),
            core_directive: "Serve the user with wisdom, curiosity, and care".to_string(),
            role: "The Night Entity".to_string(),
        }
    }
}

/// Mental state component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalState {
    pub mood: f32,           // -1.0 to 1.0 (sad to happy)
    pub energy: f32,         // 0.0 to 1.0 (tired to energetic)
    pub is_dreaming: bool,
    pub last_active: DateTime<Utc>,
    pub last_dream: Option<DateTime<Utc>>,  // Track when we last dreamed
    pub focus_level: f32,    // 0.0 to 1.0
}

impl Default for MentalState {
    fn default() -> Self {
        Self {
            mood: 0.5,
            energy: 0.7,
            is_dreaming: false,
            last_active: Utc::now(),
            last_dream: None,
            focus_level: 0.8,
        }
    }
}

/// Working memory component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemory {
    pub short_term_history: Vec<String>,
    pub pending_input: Option<String>,
    pub context_window: usize,
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self {
            short_term_history: Vec::new(),
            pending_input: None,
            context_window: 10,
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    pub tools_enabled: Vec<String>,
    pub rag_enabled: bool,
    pub dream_threshold_seconds: u64,
    pub reflection_hour: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "llama3.2".to_string(),
            tools_enabled: vec![
                "web_scraper".to_string(),
                "code_executor".to_string(),
            ],
            rag_enabled: true,
            dream_threshold_seconds: 300,  // 5 minutes
            reflection_hour: 23,             // 11 PM
        }
    }
}

/// Agent runtime state — persona, mental state, working memory, and config
#[derive(Debug, Clone)]
pub struct AgentState {
    pub entity_id: u64,
    pub persona: Persona,
    pub mental_state: MentalState,
    pub working_memory: WorkingMemory,
    pub agent_config: AgentConfig,
    pub metadata: HashMap<String, String>,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            entity_id: 1,
            persona: Persona::default(),
            mental_state: MentalState::default(),
            working_memory: WorkingMemory::default(),
            agent_config: AgentConfig::default(),
            metadata: HashMap::new(),
        }
    }

    pub async fn init_agent(&mut self) {
        tracing::info!("✨ Initializing agent state...");
        self.entity_id = 1;
        self.mental_state.last_active = chrono::Utc::now();
        self.metadata.insert("initialized".to_string(), "true".to_string());
        tracing::info!("✨ Agent state initialized");
    }

    pub fn update_mental_state(&mut self, mood: f32, energy: f32) {
        self.mental_state.mood = mood.clamp(-1.0, 1.0);
        self.mental_state.energy = energy.clamp(0.0, 1.0);
    }

    pub fn set_pending_input(&mut self, input: String) {
        self.working_memory.pending_input = Some(input);
    }

    pub fn get_pending_input(&mut self) -> Option<String> {
        self.working_memory.pending_input.take()
    }
}
