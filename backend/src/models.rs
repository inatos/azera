use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ============================================================
// Core Domain Models (matching frontend TypeScript interfaces)
// ============================================================

/// Tag for organizing chats and personas
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,  // Hex color
}

/// Chat group for organizing conversations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatGroup {
    pub id: String,
    pub name: String,
    pub color: String,  // Hex color
    pub collapsed: bool,
    pub order: i64,
}

/// Persona (AI or User)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Persona {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub persona_type: String,  // "ai" or "user"
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bubble_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default = "default_true")]
    pub global_memory_enabled: bool,  // Enable cross-chat memory (default true for AI growth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_mood: Option<String>,  // Dynamic mood based on last response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<VoiceConfig>,    // Voice/TTS settings
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool { true }

/// Voice configuration for TTS
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,       // Creative description of the voice
    #[serde(default = "default_pitch")]
    pub pitch: f32,                         // 0.0 to 2.0 (1.0 = normal) - browser TTS
    #[serde(default = "default_rate")]
    pub rate: f32,                          // 0.1 to 10.0 (1.0 = normal) - browser TTS
    #[serde(default = "default_volume")]
    pub volume: f32,                        // 0.0 to 1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,         // Browser voice name
    
    // AI TTS settings
    #[serde(default)]
    pub use_ai_tts: bool,                   // Toggle between browser/AI TTS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_model: Option<String>,           // AI model name, e.g., "qwen3-tts"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_sample_url: Option<String>,   // URL to uploaded voice sample
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_description: Option<String>,  // Voice characteristics description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_text: Option<String>,        // Text to read when testing
}

fn default_pitch() -> f32 { 1.0 }
fn default_rate() -> f32 { 1.0 }
fn default_volume() -> f32 { 1.0 }

/// Chat message
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,  // "user", "assistant", "system"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_persona: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_persona: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
}

/// Chat branch for conversation forking
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatBranch {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fork_point_message_id: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
}

/// Full chat with branches
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub branches: Vec<ChatBranch>,
    pub current_branch_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Dream entry (AI hallucinations during idle)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dream {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Journal entry (AI reflections)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalEntry {
    pub id: String,
    pub date: String,  // YYYY-MM-DD
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
}

/// System log entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: String,  // "info", "debug", "warn", "error"
    pub message: String,
}

// ============================================================
// API Request/Response Types
// ============================================================

/// Request structure for streaming chat endpoint
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatRequest {
    pub chat_id: String,
    pub branch_id: String,
    pub message: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_persona_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_persona_id: Option<String>,
}

fn default_model() -> String {
    "llama3.2".to_string()
}

/// SSE event types for streaming response
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "thinking_start")]
    ThinkingStart,
    #[serde(rename = "thinking")]
    Thinking { content: String },
    #[serde(rename = "thinking_end")]
    ThinkingEnd,
    #[serde(rename = "content")]
    Content { content: String },
    #[serde(rename = "done")]
    Done { 
        message_id: String,
        mood: Option<String>,
        mood_value: Option<f32>,
        energy: Option<f32>,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Create chat request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateChatRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
}

/// Update chat request
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateChatRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch_id: Option<String>,
}

/// Create persona request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePersonaRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub persona_type: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bubble_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default = "default_true")]
    pub global_memory_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<VoiceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Update persona request
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePersonaRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bubble_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_memory_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<VoiceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Create group request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
}

/// Update group request
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateGroupRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
}

/// Create tag request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
}

/// Update tag request
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

fn default_color() -> String {
    "#6b7280".to_string()
}

/// Update mood request
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMoodRequest {
    pub mood: String,  // "idle", "thinking", "surprised", "happy"
}

/// TTS synthesis request  
#[derive(Debug, Serialize, Deserialize)]
pub struct TtsSynthesisRequest {
    pub text: String,
    #[serde(default = "default_tts_model")]
    pub model: String,  // "qwen3-tts", "coqui-tts", "bark"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_sample_url: Option<String>,  // URL to voice sample for cloning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_description: Option<String>, // Voice characteristics description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,        // Persona ID to use saved voice config
}

fn default_tts_model() -> String {
    "qwen3-tts".to_string()
}

/// TTS synthesis response
#[derive(Debug, Serialize, Deserialize)]
pub struct TtsSynthesisResponse {
    pub audio_base64: String,  // Base64 encoded audio data
    pub format: String,        // "wav", "mp3", etc.
    pub duration_ms: u64,      // Duration in milliseconds
}

/// Status response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusResponse {
    pub status: String,  // "awake", "dreaming", "thinking"
    pub mood: String,    // "idle", "thinking", "surprised", "happy"
    pub mood_value: f32, // -1.0 to 1.0
    pub energy: f32,     // 0.0 to 1.0
    pub is_dreaming: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_active: Option<DateTime<Utc>>,
}

/// History response (legacy)
#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
}

/// List response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
}

// ============================================================
// Ollama API Types
// ============================================================

/// Ollama chat message (simplified for API)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
}

/// Ollama API request format
/// TODO(future): Used when non-streaming Ollama requests are needed
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub messages: Vec<OllamaMessage>,
    pub stream: bool,
}

/// Ollama streaming response chunk
#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaStreamChunk {
    pub model: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<OllamaMessage>,
    pub done: bool,
}

/// Ollama non-streaming response
/// TODO(future): Used when non-streaming Ollama responses are needed
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaResponse {
    pub model: String,
    pub message: OllamaMessage,
    pub done: bool,
}

// ============================================================
// Model Management Types
// ============================================================

/// Installed model info from Ollama
/// TODO(future): Used by model management UI
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledModel {
    pub name: String,
    pub modified_at: Option<String>,
    pub size: Option<u64>,
    pub digest: Option<String>,
}

/// Request to pull a new model
#[derive(Debug, Serialize, Deserialize)]
pub struct PullModelRequest {
    pub model: String,  // format: "model:version" e.g. "llama3.2:latest"
}

/// Response for model operations
/// TODO(future): Used by model management handlers
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub status: String,
    pub model: Option<String>,
    pub message: Option<String>,
}

/// Ollama pull progress event
#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaPullProgress {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
}

// ============================================================
// Image Generation Types
// ============================================================

/// Request for image generation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<String>,  // base64 encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_strength: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,       // For persona-triggered generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_filename: Option<String>,  // Custom name for the image
}

/// Generated image metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneratedImage {
    pub id: String,
    pub filename: String,
    pub url: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Image generation progress SSE event
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ImageGenEvent {
    #[serde(rename = "progress")]
    Progress { step: u32, total_steps: u32, percentage: f32 },
    #[serde(rename = "preview")]
    Preview { image_data: String },  // base64 preview
    #[serde(rename = "complete")]
    Complete { image: Box<GeneratedImage> },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Image upload request for reference images
/// TODO(future): Used when image upload endpoint is implemented
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUploadRequest {
    pub image_data: String,  // base64 encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Response for image upload
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUploadResponse {
    pub id: String,
    pub url: String,
}

/// Available image generation models
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageModel {
    pub name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub installed: bool,
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod persona_tests {
        use super::*;

        #[test]
        fn persona_serialization_roundtrip() {
            let persona = Persona {
                id: "test-id".to_string(),
                name: "Test Persona".to_string(),
                persona_type: "ai".to_string(),
                description: "A test persona".to_string(),
                avatar: Some("🤖".to_string()),
                bubble_color: Some("#ff0000".to_string()),
                system_prompt: Some("You are helpful.".to_string()),
                global_memory_enabled: true,
                current_mood: Some("happy".to_string()),
                voice: None,
                metadata: HashMap::new(),
                tags: Some(vec!["test".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let json = serde_json::to_string(&persona).unwrap();
            let deserialized: Persona = serde_json::from_str(&json).unwrap();
            
            assert_eq!(persona.id, deserialized.id);
            assert_eq!(persona.name, deserialized.name);
            assert_eq!(persona.persona_type, deserialized.persona_type);
        }

        #[test]
        fn persona_type_renamed_in_json() {
            let persona = Persona {
                id: "test".to_string(),
                name: "Test".to_string(),
                persona_type: "ai".to_string(),
                description: "".to_string(),
                avatar: None,
                bubble_color: None,
                system_prompt: None,
                global_memory_enabled: true,
                current_mood: None,
                voice: None,
                metadata: HashMap::new(),
                tags: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let json = serde_json::to_string(&persona).unwrap();
            // Should serialize as "type" not "persona_type"
            assert!(json.contains("\"type\":\"ai\""));
            assert!(!json.contains("persona_type"));
        }
    }

    mod chat_message_tests {
        use super::*;

        #[test]
        fn chat_message_with_all_fields() {
            let message = ChatMessage {
                id: "msg-1".to_string(),
                role: "assistant".to_string(),
                content: "Hello!".to_string(),
                timestamp: Some(Utc::now()),
                user_persona: Some("user-1".to_string()),
                ai_persona: Some("ai-1".to_string()),
                model: Some("llama3.2".to_string()),
                mood: Some("friendly".to_string()),
            };

            let json = serde_json::to_string(&message).unwrap();
            assert!(json.contains("\"role\":\"assistant\""));
            assert!(json.contains("\"content\":\"Hello!\""));
        }

        #[test]
        fn chat_message_skips_none_fields() {
            let message = ChatMessage {
                id: "msg-1".to_string(),
                role: "user".to_string(),
                content: "Hi".to_string(),
                timestamp: None,
                user_persona: None,
                ai_persona: None,
                model: None,
                mood: None,
            };

            let json = serde_json::to_string(&message).unwrap();
            // Optional None fields should not appear in JSON
            assert!(!json.contains("timestamp"));
            assert!(!json.contains("model"));
            assert!(!json.contains("mood"));
        }
    }

    mod dream_tests {
        use super::*;

        #[test]
        fn dream_serialization() {
            let dream = Dream {
                id: "dream-1".to_string(),
                timestamp: Utc::now(),
                title: "A Curious Dream".to_string(),
                content: "I dreamt of electric sheep...".to_string(),
                mood: Some("contemplative".to_string()),
                persona_id: Some("azera".to_string()),
                persona_name: Some("Azera".to_string()),
                tags: Some(vec!["surreal".to_string(), "peaceful".to_string()]),
            };

            let json = serde_json::to_string(&dream).unwrap();
            let parsed: Dream = serde_json::from_str(&json).unwrap();
            
            assert_eq!(dream.title, parsed.title);
            assert_eq!(dream.content, parsed.content);
        }
    }

    mod voice_config_tests {
        use super::*;

        #[test]
        fn voice_config_default_values() {
            let json = "{}";
            let config: VoiceConfig = serde_json::from_str(json).unwrap();
            
            assert_eq!(config.pitch, 1.0);
            assert_eq!(config.rate, 1.0);
            assert_eq!(config.volume, 1.0);
            assert!(!config.use_ai_tts);
        }

        #[test]
        fn voice_config_custom_values() {
            let config = VoiceConfig {
                description: Some("A warm voice".to_string()),
                pitch: 1.2,
                rate: 0.9,
                volume: 0.8,
                voice_name: Some("Alex".to_string()),
                use_ai_tts: true,
                ai_model: Some("xtts".to_string()),
                voice_sample_url: Some("/samples/custom.wav".to_string()),
                voice_description: Some("Deep and calm".to_string()),
                sample_text: Some("Testing one two three".to_string()),
            };

            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"pitch\":1.2"));
            assert!(json.contains("\"use_ai_tts\":true"));
        }
    }

    mod stream_event_tests {
        use super::*;

        #[test]
        fn stream_event_variants() {
            // Test each variant serializes with correct type tag
            let events = vec![
                (StreamEvent::ThinkingStart, "thinking_start"),
                (StreamEvent::Thinking { content: "...".to_string() }, "thinking"),
                (StreamEvent::ThinkingEnd, "thinking_end"),
                (StreamEvent::Content { content: "Hi".to_string() }, "content"),
                (StreamEvent::Done { message_id: "1".to_string(), mood: None, mood_value: None, energy: None }, "done"),
                (StreamEvent::Error { message: "oops".to_string() }, "error"),
            ];

            for (event, expected_type) in events {
                let json = serde_json::to_string(&event).unwrap();
                assert!(json.contains(&format!("\"type\":\"{}\"", expected_type)), 
                    "Expected type {} in {}", expected_type, json);
            }
        }
    }

    mod ollama_types_tests {
        use super::*;

        #[test]
        fn ollama_message_roles() {
            let messages = vec![
                OllamaMessage { role: "system".to_string(), content: "You are helpful.".to_string() },
                OllamaMessage { role: "user".to_string(), content: "Hello".to_string() },
                OllamaMessage { role: "assistant".to_string(), content: "Hi there!".to_string() },
            ];

            let request = OllamaRequest {
                model: "llama3.2".to_string(),
                messages,
                stream: true,
            };

            let json = serde_json::to_string(&request).unwrap();
            assert!(json.contains("\"stream\":true"));
            assert!(json.contains("\"model\":\"llama3.2\""));
        }

        #[test]
        fn ollama_pull_progress_partial() {
            // Progress events may not have all fields
            let json = r#"{"status": "downloading"}"#;
            let progress: OllamaPullProgress = serde_json::from_str(json).unwrap();
            assert_eq!(progress.status, "downloading");
            assert!(progress.total.is_none());
            assert!(progress.completed.is_none());
        }

        #[test]
        fn ollama_pull_progress_complete() {
            let json = r#"{
                "status": "downloading",
                "digest": "sha256:abc123",
                "total": 1000000,
                "completed": 500000
            }"#;
            let progress: OllamaPullProgress = serde_json::from_str(json).unwrap();
            assert_eq!(progress.total, Some(1000000));
            assert_eq!(progress.completed, Some(500000));
        }
    }
    
    mod image_generation_tests {
        use super::*;
        
        #[test]
        fn image_generation_request_serialization() {
            let request = ImageGenerationRequest {
                prompt: "A beautiful sunset".to_string(),
                negative_prompt: Some("blurry, low quality".to_string()),
                model: Some("stable-diffusion".to_string()),
                width: Some(512),
                height: Some(512),
                steps: Some(20),
                cfg_scale: Some(7.0),
                seed: Some(42),
                reference_image: None,
                reference_strength: None,
                persona_id: Some("azera".to_string()),
                custom_filename: Some("my_sunset".to_string()),
            };
            
            let json = serde_json::to_string(&request).unwrap();
            assert!(json.contains("\"prompt\":\"A beautiful sunset\""));
            assert!(json.contains("\"negative_prompt\":\"blurry, low quality\""));
            assert!(json.contains("\"seed\":42"));
        }
        
        #[test]
        fn image_generation_request_deserialize_minimal() {
            let json = r#"{ "prompt": "A cat sitting on a rainbow" }"#;
            let request: ImageGenerationRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.prompt, "A cat sitting on a rainbow");
            assert!(request.negative_prompt.is_none());
            assert!(request.model.is_none());
            assert!(request.width.is_none());
        }
        
        #[test]
        fn generated_image_serialization() {
            let image = GeneratedImage {
                id: "img-123".to_string(),
                filename: "sunset_20260101.png".to_string(),
                url: "/api/images/sunset_20260101.png".to_string(),
                prompt: "A beautiful sunset".to_string(),
                negative_prompt: None,
                model: Some("stable-diffusion".to_string()),
                width: 512,
                height: 512,
                steps: Some(20),
                cfg_scale: Some(7.0),
                seed: Some(42),
                persona_id: None,
                persona_name: Some("Azera".to_string()),
                created_at: Utc::now(),
            };
            
            let json = serde_json::to_string(&image).unwrap();
            assert!(json.contains("\"filename\":\"sunset_20260101.png\""));
            assert!(json.contains("\"width\":512"));
            assert!(json.contains("\"persona_name\":\"Azera\""));
        }
        
        #[test]
        fn image_gen_event_variants() {
            let progress = ImageGenEvent::Progress { 
                step: 5, 
                total_steps: 20, 
                percentage: 25.0 
            };
            let json = serde_json::to_string(&progress).unwrap();
            assert!(json.contains("\"type\":\"progress\""));
            assert!(json.contains("\"step\":5"));
            
            let error = ImageGenEvent::Error { 
                message: "Generation failed".to_string() 
            };
            let json = serde_json::to_string(&error).unwrap();
            assert!(json.contains("\"type\":\"error\""));
            assert!(json.contains("\"message\":\"Generation failed\""));
        }
        
        #[test]
        fn image_model_serialization() {
            let model = ImageModel {
                name: "stable-diffusion-xl".to_string(),
                display_name: "Stable Diffusion XL".to_string(),
                description: Some("High quality image generation".to_string()),
                installed: true,
            };
            
            let json = serde_json::to_string(&model).unwrap();
            assert!(json.contains("\"name\":\"stable-diffusion-xl\""));
            assert!(json.contains("\"installed\":true"));
        }
    }
}