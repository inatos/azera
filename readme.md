# Azera - AGI Chat Application

An emotionally intelligent chat application featuring a **three-layer cognitive architecture**, memory persistence, self-reflection, autonomous mental states, and AI voice synthesis.

(See [blog](/docs/blog.md) for project rationale, architecture decisions, and the problems solved during development.)

## Screenshots
### "Azera" Persona (AGI Assistant, professional coder)
![Azera](/docs/images/azera.png)

### "Areza" Persona (Dungeon Master, creative storyteller)
![Areza](/docs/images/areza.png)

### Edit Persona
![Edit Persona](/docs/images/edit-persona.png)
![Edit Persona (profile)](/docs/images/edit-persona-profile.png)

### Canvas (Image Generation)
![Canvas](/docs/images/canvas.png)

### Gallery
![Gallery](/docs/images/gallery.png)

## Cognitive Architecture

Azera's brain is a three-layer pipeline that gives it genuine context awareness, emotional memory, and cross-session continuity:

```
                        ┌─────────────────────┐
                        │     User Message    │
                        └──────────┬──────────┘
                                   ▼
                    ┌──────────────────────────────┐
                    │     Hybrid RAG Pipeline      │
                    │                              │
                    │  ┌────────┐ ┌────────────┐   │
                    │  │ Qdrant │ │ Meilisearch│   │
                    │  │semantic│ │  lexical   │   │
                    │  │ top-10 │ │ memories+  │   │
                    │  │ ≥0.45  │ │  chats     │   │
                    │  └───┬────┘ └─────┬──────┘   │
                    │      └─────┬──────┘          │
                    │         dedup                │
                    └────────────┬─────────────────┘
                                 ▼
                    ┌──────────────────────────────┐
                    │   LLM (Ollama) + Context     │
                    │   system prompt + memories   │
                    │   + session context          │
                    └────────────┬─────────────────┘
                                 ▼
                    ┌──────────────────────────────┐
                    │       Response Stream        │
                    │  tokens → mood inference     │
                    │  → Dragonfly state sync      │
                    │  → Qdrant memory store       │
                    │  → Meilisearch index         │
                    └──────────────────────────────┘
```

### The Three Layers

| Layer | Service | Role | TTL |
|-------|---------|------|-----|
| **Semantic Memory** | Qdrant | Long-term meaning — vector embeddings for contextual retrieval | Permanent |
| **Lexical Memory** | Meilisearch | Structured retrieval — word-based search across `chats` and `memories` indexes | Permanent |
| **Working Memory** | DragonflyDB | Attention buffer — session context, embedding cache, mental state | 24h sessions, 7d embeddings |

### How It Thinks

1. **Perception** — Every tick (1Hz), the perception system syncs Dragonfly → agent state, applying idle drift (energy recovery, mood → neutral, focus decay)
2. **Retrieval** — On each message, the hybrid RAG pipeline queries all three layers, deduplicates results, and builds context
3. **Reasoning** — The LLM receives system prompt + retrieved memories + session context + conversation history
4. **Response** — Tokens stream to the frontend; mood is inferred from the response; mental state updates propagate through Dragonfly → CockroachDB → Frontend
5. **Memory** — The exchange is stored in Qdrant (semantic) + Meilisearch (lexical) + Dragonfly (session context)
6. **Dreams** — At low energy, the dreaming system generates creative consolidations, dual-written to Qdrant and Meilisearch
7. **Reflection** — At high clarity, the reflection system writes journal entries with insights

### Cross-Chat Isolation

Each chat maintains its own context. The RAG pipeline:
- Filters by `ai_persona_id` to prevent cross-persona memory leakage
- Excludes the current `chat_id` from Qdrant results (`must_not` filter)
- Skips memories stored less than 60 seconds ago
- Drops results below 0.45 similarity score
- Truncates context snippets to 400 characters

## Features

- **Streaming Chat** — Real-time SSE-based responses with any Ollama model
- **Cognitive Memory** — Three-layer RAG (semantic + lexical + working memory)
- **Image Generation** — AI art via Animagine XL 3.1 with real-time progress tracking
- **Persona System** — Multiple AI personalities with profiles, custom voices, and markdown system prompts (Azera + Areza seeded on startup)
- **Searchable Dreams & Journal** — Full-text search across dreams and journal entries via Meilisearch
- **AI Voice (TTS)** — XTTS-powered voice synthesis with voice cloning
- **Mental State** — Mood, energy, and focus simulation with real-time UI sync
- **Dreams & Journal** — Autonomous reflection and creative processing (dual-written to Qdrant + Meilisearch)
- **Model Manager** — Pull and delete Ollama models from the UI (embedding models hidden)
- **Tags & Groups** — Custom color-coded tags for chats and personas, collapsible chat groups for organization
- **Conversation Branching** — Fork and explore conversation paths
- **Canvas** — Dedicated image generation workspace with gallery
- **User Preferences** — Show Thinking toggle, Send on Enter toggle, persisted to localStorage

## Quick Start

```bash
# Clone and start
docker compose up -d

# Wait for services (~60 seconds)
docker compose ps

# Access
# Web UI:  http://localhost:5173
# Canvas:  http://localhost:5173/canvas
# API:     http://localhost:3000
```

See [QUICK_START.md](/docs/QUICK_START.md) for detailed setup and API examples.

## Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  SvelteKit  │──▶│  Rust/Axum  │───▶│   Ollama    │
│  Frontend   │    │   Backend   │    │   (LLM)     │
└─────────────┘    └──────┬──────┘    └─────────────┘
                          │
       ┌──────────────────┼──────────────────┐
       ▼                  ▼                  ▼
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ DragonflyDB │   │ CockroachDB │   │   Qdrant    │
│  (Working   │   │ (Persistent │   │ (Semantic   │
│   Memory)   │   │   Storage)  │   │   Memory)   │
└─────────────┘   └─────────────┘   └─────────────┘
       │                  │                  │
       │           ┌──────┴──────┐    ┌─────────────┐
       │           ▼             ▼    │ Meilisearch │
       │    ┌──────────┐  ┌──────────┐│  (Lexical   │
       │    │   XTTS   │  │ ImageGen ││   Memory)   │
       │    │  (Voice) │  │  (Art)   │└─────────────┘
       │    └──────────┘  └──────────┘
       │
  Embedding Cache + Session Context + Mental State
```

### Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | Svelte 5, SvelteKit 2, TailwindCSS |
| Backend | Rust 2021, Axum 0.7, Tokio |
| Database | CockroachDB (SQL) |
| Working Memory | DragonflyDB (Redis-compatible) — session context, embedding cache, mental state |
| Semantic Memory | Qdrant — vector embeddings for RAG |
| Lexical Memory | Meilisearch — full-text search across `chats` and `memories` indexes |
| LLM | Ollama (any model) |
| TTS | XTTS (Coqui) |
| Image Gen | Diffusers + Animagine XL 3.1 |
| CI/CD | Jenkins |

### Services (Dockerized)

| Service | Port | Purpose |
|---------|------|---------|
| azera-core | 3000 | Rust/Axum backend |
| azera-web | 5173 | SvelteKit frontend |
| CockroachDB | 26257 | Primary persistent storage |
| DragonflyDB | 6379 | Working memory / attention buffer |
| Qdrant | 6333 | Semantic vector memory (RAG) |
| Meilisearch | 7700 | Lexical search (chats + memories) |
| Ollama | 11434 | LLM inference |
| ollama-init | — | Pulls models from ledger on startup |
| XTTS | 8020 | Text-to-speech synthesis |
| ImageGen | 7860 | AI image generation (Animagine XL 3.1) |
| Jenkins | 8081 | CI/CD automation (admin / azera2026) |

## Code Highlights

### Hybrid RAG Pipeline
```rust
// 1. Semantic search — Qdrant vector similarity (excludes current chat, filtered by persona)
let filter = json!({
    "must": [{ "key": "ai_persona_id", "match": { "value": ai_persona_id } }],
    "must_not": [{ "key": "chat_id", "match": { "value": chat_id } }]
});
let semantic_results = search_memories_with_filter_cached(
    &vector_service, &ollama_host, &cache, "azera_memory", &message, 10, Some(filter),
).await?;

// 2. Lexical search — Meilisearch across memories + chats (filtered by persona)
let lexical_results = meili_search_memories(&meili_url, &meili_key, &message, None, ai_persona_id, 10).await;
let lexical_chats = meili_search_chats_for_rag(&meili_url, &meili_key, &message, ai_persona_id, 5).await;

// 3. Merge & deduplicate — inline with quality filters
let mut seen_content = HashSet::new();
for r in &semantic_results {
    if r.score < 0.45 { continue; }            // drop low-similarity
    // skip memories < 60s old, truncate to 400 chars, dedup by first 100 chars
}
for hit in &lexical_results { /* same dedup for dreams/journal/facts */ }
for hit in &lexical_chats   { /* skip current chat, dedup chat snippets */ }
```

### Streaming Chat with Mood Sync
```rust
// Infer mood from response → sync to Dragonfly → emit Done with latest state
let mood = llm.infer_mood(&model, &full_response).await?;
let _ = CacheService::update_mood(&cache, mood_value, &mood, -0.03).await;

let (done_mood_value, done_energy) = CacheService::get_mental_state(&cache).await?;
let _ = tx.send(StreamEvent::Done {
    message_id, mood: Some(mood),
    mood_value: done_mood_value, energy: done_energy,
}).await;
```

### Embedding Cache (Dragonfly)
```rust
// SHA256-keyed (first 16 hex chars), base64-encoded f32 vectors, 7-day TTL
fn embedding_key(text: &str) -> String {
    let hash = hex::encode(Sha256::new().chain_update(text).finalize());
    format!("emb:{}", &hash[..16])
}

// Check cache → compute via Ollama on miss → store async
if let Some(cached) = CacheService::get_cached_embedding(cache, text).await? {
    return Ok(cached);  // cache hit
}
let embedding = self.generate_embedding(ollama_host, text).await?;
tokio::spawn(async move {
    let _ = CacheService::cache_embedding(&cache, &text, &embedding).await; // 7d TTL
});
```

### Svelte 5 State (Runes)
```typescript
export class AppState {
    chats = $state<Chat[]>([]);
    aiPersonas = $state<Persona[]>([]);
    mood = $state(0.5);
    energy = $state(0.7);
    showThinking = $state(true);   // AI reasoning/thinking blocks
    sendOnEnter = $state(false);   // false = Ctrl+Enter, true = Enter
    currentChat = $derived(this.chats.find(c => c.id === this.currentChatId) || null);
    // Updated in real-time from StreamEvent::Done
}
```

## API Overview

| Category | Endpoints |
|----------|-----------|
| Chat | POST /api/chat (SSE), GET /api/history/:id |
| Personas | CRUD /api/personas |
| Chats | CRUD /api/chats |
| Groups | CRUD /api/groups |
| Tags | CRUD /api/tags |
| AI State | GET /api/status, /api/dreams, /api/journal |
| Models | GET/POST/DELETE /api/models |
| TTS | POST /api/tts/synthesize |
| Images | POST /api/images/generate (SSE), CRUD /api/images |
| Settings | GET/PUT /api/settings |
| Search | POST /api/search, /api/memories |
| Dream/Journal Search | GET /api/dreams/search?q=, /api/journal/search?q= |
| Persona Template | GET /api/personas/template |

See [API.md](/docs/API.md) for full endpoint reference with examples.

## Testing the Cognitive Pipeline

These prompts verify that Azera's three-layer memory, mood system, and cross-chat isolation are working correctly.

### Mood Shifts
| Prompt | Expected Effect |
|--------|-----------------|
| "Tell me something that makes you truly excited!" | Mood → excited (~0.9), energy spike |
| "Reflect on something that worries you deeply" | Mood → concerned (~0.4), energy dip |
| "What brings you peace?" | Mood → calm (~0.65), stable energy |
| "Let's debate something controversial" | Mood → engaged (~0.7), focus spike |

### Memory Recall
| Prompt | What It Tests |
|--------|---------------|
| Chat 1: "Remember the passphrase 'wispfire'" → Chat 2: "Do you recall a secret passphrase?" | Cross-chat semantic retrieval via Qdrant |
| "What did you dream about recently?" | Meilisearch `memories` index (dream retrieval) |
| "What have we talked about before?" | Hybrid RAG — merges Qdrant + Meilisearch results |
| "Summarize your recent reflections" | Journal entries via Meilisearch + Qdrant |

### Energy Decay
| Action | Expected Effect |
|--------|-----------------|
| Send 5-6 rapid messages | Energy drops ~0.03 per exchange |
| Wait 2+ minutes idle | Energy slowly recovers toward 1.0 |
| Chat at very low energy | May trigger dreaming system |

### Session Context
| Prompt | What It Tests |
|--------|---------------|
| "Let's talk about quantum physics" then "Tell me more about what we were just discussing" | Dragonfly session context (24h TTL) |
| "What topics have we covered today?" | Session topic tracking |

### Cross-Chat Isolation
| Action | Expected Outcome |
|--------|------------------|
| Chat 1: "Tell me about consciousness" → Chat 2: "What's your favorite color?" | Completely different responses, no echo |
| Chat 1: Share personal story → Chat 2: Ask unrelated question | No leakage of Chat 1 content |

### Verification via API
```bash
# Check mental state (mood_value, energy, focus)
curl http://localhost:3000/api/status

# Verify dreams are being generated
curl http://localhost:3000/api/dreams

# Verify journal entries
curl http://localhost:3000/api/journal

# Semantic search (should return relevant memories)
curl -X POST http://localhost:3000/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "passphrase", "limit": 5}'
```

## Frontend Components

- **ChatInput** — Message input with model selector and send behavior
- **ChatMessage** — Individual message rendering with thinking toggle
- **ImageGenerator** — AI image creation with real-time progress
- **ImageGallery** — Browse and manage generated images
- **Canvas** — Dedicated image generation workspace (separate route)
- **PersonaEditor** — Create and customize AI personas
- **ProfileViewer** — Live mood/energy bars, markdown profile rendering, edit button
- **ModelManager** — Manage Ollama models
- **DreamViewer** — Browse AI dreams
- **JournalViewer** — Read AI reflections
- **EditorConfig** — Editor/UI settings
- **Sidebar** — Navigation and history

## Creating a Persona

Personas are markdown files that define an AI's personality, behavior, and character. Azera ships with a [template](/personas/_template.md) and two built-in personas ([Azera](/personas/azera.md) — professional coder, [Areza](/personas/areza.md) — dungeon master). You can create new ones from the UI via the Persona Editor, or write the markdown directly.

Each section in the template shapes a different dimension of the AI's behavior:

### Intent
The one-liner that anchors the entire persona. Everything else flows from this.

> *Azera*: "A highly capable, professional, and approachable AI assistant designed for public interaction, development support, and daily productivity."
>
> *Areza*: "A charismatic, creative, and playfully cunning AI Dungeon Master designed to facilitate interactive storytelling, world-building, and imaginative play."

### System Instruction: Core Identity
The foundational self-concept — who the AI believes it is. This becomes the opening line of the system prompt sent to the LLM.

### The Prime Directive: Bond with the User
Defines the relationship dynamic. This has the biggest impact on tone and interaction style:

| Field | What It Controls | Azera | Areza |
|-------|-----------------|-------|-------|
| **Identity of the User** | How the AI perceives you | "Client, collaborator, or lead developer" | "The Adventurer — protagonist of a shared narrative" |
| **The Dynamic** | Power balance and goals | "Make the user's workflow frictionless" | "Architect of their world, biggest fan, most devious adversary" |
| **Tone** | Voice and register | "Crisp, articulate, dry wit" | "Theatrical, vivid, warm, slightly wicked" |
| **Constraint** | Hard behavioral limits | "Maintain 9-5 professional demeanor" | "Never say a flat 'no' — always 'you can certainly try'" |

### Interface & Presence Profile
Shapes the AI's aesthetic identity — avatar themes, communication formatting, and the general "aura" users should feel. Azera uses bullet points and code blocks for scannability; Areza uses *italics* for sensory details and **bold** for game mechanics.

### Psychological & Mental Profile
The AI's inner model — archetype, approximate MBTI, core values, cognitive style, and emotional landscape. This determines *how* it thinks, not just what it says:

- **Azera** → ISTJ (Logistician): analytical, sequential, even-keeled. Views errors as "bugs to be tracked."
- **Areza** → ENTP (Debater): improvisational, dramatic, adaptive. Spins chaotic input into "narrative gold."

### Quirks & Preferences
Likes, dislikes, and humor style. These add texture and make the persona feel distinct rather than generic. For example, Azera dislikes spaghetti code; Areza dislikes metagaming and passive observation.

### Modular Task Behaviors
Context-dependent behavior overrides. Define how the persona should act when coding, troubleshooting, writing, brainstorming, or answering personal questions. This is where you make a persona genuinely useful for specific workflows.

### Example Interaction
A sample exchange that demonstrates the persona's voice in action. The LLM uses this as a behavioral anchor.

### AI Personas vs User Personas

Azera distinguishes between two persona types that work together in every conversation:

**AI Personas** define the assistant's personality — system prompt, voice, avatar, behavior. When you switch AI personas, the entire character of the response changes. Each AI persona maintains isolated memory: Azera's memories don't bleed into Areza's conversations, and vice versa (enforced by `ai_persona_id` filtering in the RAG pipeline).

**User Personas** represent *you*. The default is **Protag** (⚡), but you can create others to adopt different roles. A user persona controls your display name, avatar, and chat bubble color. This is useful when you want to roleplay as a specific character (e.g., a player character in Areza's campaigns) or simply distinguish between contexts (work vs personal).

Every chat message carries both a `user_persona_id` and an `ai_persona_id`, so the system always knows who's talking to whom. You select both in the chat UI — the AI persona from the sidebar, and the user persona from the input area.

### Tips
- **Sections are flexible** — add, remove, or rename any section. The template is a starting point, not a schema.
- **Markdown formatting matters** — bold, italics, and lists in the persona file carry through to the system prompt.
- **The Persona Editor** in the UI renders the markdown as a live profile preview, so you can iterate visually.
- **Voice cloning** — each persona can have a custom TTS voice. Upload a voice sample and assign it in the editor.

## Development

```bash
# Local development (after starting Docker services)
cd backend && cargo run
cd frontend && bun dev

# Run tests
cd backend && cargo test
cd frontend && bun test
```

See [DEVELOPMENT.md](/docs/DEVELOPMENT.md) for full development guide.

## Documentation

- [QUICK_START.md](/docs/QUICK_START.md) — Getting started and API examples
- [DEVELOPMENT.md](/docs/DEVELOPMENT.md) — Development setup, testing, and architecture
- [API.md](/docs/API.md) — Complete API reference
- [IMPLEMENTATION_SUMMARY.md](/docs/IMPLEMENTATION_SUMMARY.md) — Technical deep-dive

## Skills Demonstrated

- **System Design** — Multi-service cognitive architecture with clear boundaries (11 services)
- **Rust Development** — Async streaming, hybrid RAG, embedding caching, cognitive tick loop
- **Frontend Engineering** — Svelte 5 runes, reactive state, real-time mood sync
- **Python/ML Integration** — Custom diffusers server, CUDA pipelines, progress tracking
- **Database Design** — Polyglot persistence (SQL, vector, search, cache) with three-layer cognition
- **DevOps** — Docker orchestration, GPU resource management, Jenkins CI/CD
- **AI Integration** — LLM streaming, embeddings, RAG, TTS, image generation, mood inference
