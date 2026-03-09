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

### Canvas (Image & 3D Generation)
![2D Canvas](/docs/images/canvas.png)
![3D Canvas](/docs/images/image-to-3d.png)

### Gallery
![2D Gallery](/docs/images/gallery.png)
![3D Gallery](/docs/images/3d-vaal-orb.png)

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
- **3D Generation** — Image-to-3D via Hunyuan3D 2.1 with low-VRAM pipeline parallelism and SSE progress tracking
- **Persona System** — Multiple AI personalities with profiles, custom voices, and markdown system prompts (Azera + Areza seeded on startup)
- **Searchable Dreams & Journal** — Full-text search across dreams and journal entries via Meilisearch
- **AI Voice (TTS)** — XTTS-powered voice synthesis with voice cloning
- **Mental State** — Mood, energy, and focus simulation with real-time UI sync
- **Dreams & Journal** — Autonomous reflection and creative processing (dual-written to Qdrant + Meilisearch)
- **Model Manager** — Pull and delete Ollama models from the UI (embedding models hidden)
- **Tags & Groups** — Custom color-coded tags for chats and personas, collapsible chat groups for organization
- **Conversation Branching** — Fork and explore conversation paths
- **Canvas** — Dedicated image & 3D generation workspace with 2D and 3D galleries
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
       │                  ┌──────────┐
       │                  │  Gen3D   │
       │                  │  (3D)    │
       │                  └──────────┘
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
| 3D Gen | Hunyuan3D 2.1 (Tencent) |
| CI/CD | Jenkins |
| Disk Maintenance | docker-gc (daily auto-prune) |

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
| Gen3D | 7861 | Image-to-3D generation (Hunyuan3D 2.1) |
| Jenkins | 8081 | CI/CD automation (admin / azera2026) |
| docker-gc | — | Automated Docker disk cleanup (daily prune) |

## GPU Memory Optimization (Low-VRAM Strategy)

Hunyuan3D 2.1 is a 3.3B-parameter DiT model that nominally requires 24+ GB of system RAM and 10+ GB of VRAM. I dev'd Azera on a RTX 3080 Ti Laptop (16 GB VRAM) alongside image generation and voice synthesis by applying several layers of memory optimization:

### Sequential CPU↔GPU Offloading
Instead of loading all components to GPU simultaneously (DiT 6.5 GB + VAE 1.5 GB + conditioner 1 GB = ~10 GB), the pipeline runs in two phases:

- **Phase 1** — DiT + conditioner → GPU, run diffusion, output raw latents
- **Phase 2** — DiT + conditioner → CPU, VAE → GPU, decode latents → mesh

Peak VRAM = `max(DiT, VAE)` ≈ 7 GB instead of `sum` ≈ 10 GB.

### Pipeline Parallelism
The phase transition (DiT→CPU offload + VAE→GPU load) is parallelized using threading — both transfers happen simultaneously over separate DMA channels. This reduced the transition from ~29s to ~18.9s.

### Memory-Mapped Model Loading
Three techniques eliminate the 2× RAM peak from checkpoint loading:

1. **`mmap=True`** — Lazy page loading; the OS pages data in on demand instead of reading the entire 7 GB checkpoint into RAM
2. **`_StagedDict`** — A custom dict wrapper that auto-frees the previous sub-dict when a new key is accessed (e.g., the 6.5 GB 'model' weights are freed before 'vae' weights are touched)
3. **`assign=True`** — `load_state_dict` replaces parameters with mmap'd tensors directly instead of copying, eliminating the simultaneous checkpoint + model parameter RAM peak

Combined peak: ~6.5 GB (single largest model) instead of ~16 GB naive loading.

### Volume-Backed On-Demand Loading
Models are stored on a Docker volume (`gen3d-models`, ~34 GB) and loaded on-demand per request via `mmap`. After each generation completes, the shape pipeline is fully unloaded from RAM (`_shape_pipe = None` + `gc.collect()`). Between requests, the gen3d service holds ~0 GB instead of ~9 GB. The OS file cache keeps hot pages warm, so reloads take ~30s.

### Texture Pipeline Optimization
Default Hunyuan3D texture settings (render 2048px, texture 4096px, 6 views) were too slow and VRAM-hungry. Reduced to render 1024px, texture 2048px, 4 views — still produces clean PBR materials while fitting in VRAM alongside shape generation. The texture pipeline is lazy-loaded after shape generation finishes and released after each use.

### AOT Graph Compilation
`torch.compile` with the inductor backend wraps the DiT and conditioner for Triton kernel fusion. Compiled kernels are disk-cached (`TORCHINDUCTOR_CACHE_DIR`) on the model volume so subsequent container restarts skip the 2-5 minute compilation. The VAE is excluded from compilation because its custom CUDA marching-cubes extensions create hard graph breaks.

### Automated Recovery
The gen3d container runs with `restart: unless-stopped` and `shm_size: 4g` to survive OOM events during texture generation. A `docker-gc` sidecar prunes dangling images and build cache daily to prevent disk bloat.

## Code Highlights

### Hybrid RAG Pipeline
```rust
// 1. Semantic search — Qdrant vector similarity (excludes current chat, filtered by persona)
let filter = json!({
    "must": [{ "key": "ai_persona_id", "match": { "value": ai_persona_id } }],
    "must_not": [{ "key": "chat_id", "match": { "value": chat_id } }]
});

let semantic_results = search_memories_with_filter_cached(
    vector_service: &vector_service,
    ollama_host:    &ollama_host,
    cache:          &cache,
    collection:     "azera_memory",
    query:          &message,
    limit:          10,
    filter:         Some(filter)
).await?;

// 2. Lexical search — Meilisearch across memories + chats (filtered by persona)
let lexical_results = meili_search_memories(
    meili_url:   &meili_url,
    meili_key:   &meili_key,
    query:       &message,
    memory_type: None,
    persona_id:  ai_persona_id.as_deref(),
    limit:       10
).await;

let lexical_chats = meili_search_chats_for_rag(
    meili_url:     &meili_url,
    meili_key:     &meili_key,
    query:         &message,
    ai_persona_id: ai_persona_id.as_deref(),
    limit:         5
).await;

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
let mood = llm.infer_mood(
    model:         &model,
    response_text: &full_response
).await?;

let _ = CacheService::update_mood(
    cache:        &cache,
    mood_value:   mood_value,
    mood_label:   &mood,
    energy_delta: -0.03
).await;

// Read latest mood/energy from Dragonfly for the Done event
let (done_mood_value, done_energy) = match CacheService::get_mental_state(
    cache: &cache
).await {
    Ok(Some(ms)) => (Some(ms.mood), Some(ms.energy)),
    _            => (None, None),
};

let _ = tx.send(StreamEvent::Done {
    message_id: assistant_msg_id,
    mood:       mood,
    mood_value: done_mood_value,
    energy:     done_energy
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
if let Some(cached) = CacheService::get_cached_embedding(
    cache: cache,
    text:  text
).await? {
    return Ok(cached);  // cache hit
}
let embedding = self.generate_embedding(
    ollama_host: ollama_host,
    text:        text
).await?;
tokio::spawn(async move {
    let _ = CacheService::cache_embedding(
        cache:     &cache,
        text:      &text,
        embedding: &embedding  // 7d TTL
    ).await;
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
| Chat | POST /api/chat/stream (SSE), GET /api/history/:id |
| Personas | CRUD /api/personas |
| Chats | CRUD /api/chats, GET /api/chats/search |
| Groups | CRUD /api/groups |
| Tags | CRUD /api/tags |
| AI State | GET /api/status, /api/dreams, /api/journal |
| Models | GET/POST/DELETE /api/models |
| TTS | POST /api/tts/synthesize |
| Images | POST /api/images/generate (SSE), CRUD /api/images |
| 3D Models | POST /api/models3d/generate (SSE), CRUD /api/models3d |
| Feature Flags | GET /api/features |
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
- **Model3DGenerator** — Image-to-3D generation with parameter controls
- **Model3DGallery** — Browse and manage generated 3D models
- **Canvas** — Dedicated image & 3D generation workspace (separate route)
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

- **System Design** — Multi-service cognitive architecture with clear boundaries (13 services)
- **Rust Development** — Async streaming, hybrid RAG, embedding caching, cognitive tick loop
- **Frontend Engineering** — Svelte 5 runes, reactive state, real-time mood sync
- **Python/ML Integration** — Custom diffusers server, CUDA pipelines, low-VRAM optimization, pipeline parallelism
- **Database Design** — Polyglot persistence (SQL, vector, search, cache) with three-layer cognition
- **DevOps** — Docker orchestration, GPU resource management, automated disk cleanup, Jenkins CI/CD
- **AI Integration** — LLM streaming, embeddings, RAG, TTS, image generation, 3D generation, mood inference
- **GPU Memory Engineering** — Sequential CPU↔GPU offloading, mmap-backed model loading, volume-backed on-demand pipelines
