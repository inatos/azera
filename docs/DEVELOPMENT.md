# Development Setup Guide

## Quick Start for Contributors

### System Requirements
- **Rust**: 1.70+ (install via [rustup](https://rustup.rs))
- **Bun**: 1.0+ (install via [bun.sh](https://bun.sh))
- **Docker**: 20.10+ with Docker Compose 2.0+
- **GPU** (optional): NVIDIA GPU with CUDA for Ollama inference and image generation

### Clone & Setup

```bash
git clone https://github.com/inatos/azera.git
cd azera
```

---

## Option 1: Docker Compose (Recommended for Quick Testing)

```bash
# Clean slate
docker compose down --volumes --remove-orphans

# Build all services
docker compose build --progress=plain

# Start everything
docker compose up -d

# Wait for services (~60 seconds)
docker compose ps

## URLs
- **Web UI**: http://localhost:5173
- **Canvas (Image Gen)**: http://localhost:5173/canvas
- **API**: http://localhost:3000
- **API Health**: http://localhost:3000/health
- **CockroachDB Admin**: http://localhost:8080
- **Qdrant Dashboard**: http://localhost:6333/dashboard
- **Meilisearch**: http://localhost:7700
- **ImageGen API**: http://localhost:7860
- **Jenkins CI**: http://localhost:8081 (admin / azera2026)
```

---

## Option 2: Local Development (For Active Work)

### 1. Start Infrastructure Services

```bash
docker compose up -d cockroach dragonfly qdrant meilisearch ollama
sleep 30
docker compose ps
```

### 2. Backend Development

```powershell
cd backend

# Set environment variables (PowerShell)
$env:DATABASE_URL="postgres://root@localhost:26257/azera?sslmode=disable"
$env:DRAGONFLY_URL="redis://localhost:6379"
$env:QDRANT_URL="http://localhost:6333"
$env:OLLAMA_HOST="http://localhost:11434"
$env:MEILI_URL="http://localhost:7700"
$env:RUST_LOG="info,azera_core=debug"

# Run
cargo run

# Or build for release
cargo build --release
```

### 3. Frontend Development

```bash
cd frontend
bun install
bun dev
# Opens http://localhost:5173
```

---

## Project Structure

### Backend (`backend/src/`)
```
main.rs          # Server setup, router, service initialization
                 #   init_default_personas(): seeds Azera, Areza (AI) + Protag (user)
                 #   Regenerates missing .md files from DB personas on startup
components.rs    # Agent state (Persona, MentalState, WorkingMemory, AgentConfig)
systems.rs       # The Tick Loop — perception (Dragonfly→agent), dreaming, reflection
                 #   Dreams/reflections dual-write to Qdrant + Meilisearch
handlers.rs      # 53 HTTP request handlers + hybrid RAG pipeline
                 #   Three-source merge: Qdrant semantic + Meilisearch memories + chats
                 #   Cross-chat isolation (must_not filter, recency, score threshold)
                 #   Persona isolation: Meilisearch filters by ai_persona_id
                 #   Persona template, dream/journal search via Meilisearch
models.rs        # Request/response types (StreamEvent::Done w/ mood_value, energy)
db.rs            # CockroachDB queries (personas, chats, dreams, journal)
cache.rs         # DragonflyDB working memory layer (~350 lines)
                 #   SessionContext, CachedMentalState, embedding cache (SHA256/base64)
                 #   set/get_mental_state, update_mood, session CRUD, cache_embedding
llm.rs           # Ollama integration
vector.rs        # Qdrant vector service + cached variants via Dragonfly
                 #   StoreMemoryRequest struct, generate_embedding_cached,
                 #   store_memory_cached, search_memories_cached,
                 #   search_memories_with_filter_cached
backup.rs        # Automated backup service (5-min intervals)
tools.rs         # Web scraper, Code sandbox
```

### Image Generation (`imagegen/`)
```
server.py          # FastAPI server (Animagine XL 3.1 via diffusers)
download_models.py # Pre-downloads model weights at container startup
entrypoint.sh      # Downloads models then starts server
Dockerfile         # PyTorch + CUDA runtime
```

### Frontend (`frontend/src/lib/`)
```
store.svelte.ts      # Svelte 5 state management (AppState class)
state.svelte.ts      # UI state management
llm_service.ts       # API client for backend
tts_service.ts       # TTS playback service
components/
  ChatInput.svelte   # Message input with model selector
  ChatMessage.svelte # Individual message bubbles
  Sidebar.svelte     # Navigation, history, groups, tags
  ImageGenerator.svelte  # AI image generation with progress
  ImageGallery.svelte    # Browse/manage generated images
  PersonaEditor.svelte   # Create/edit personas
  ProfileViewer.svelte   # View persona details
  ModelManager.svelte    # Pull/delete Ollama models
  EditorConfig.svelte    # Editor/UI settings
  ThinkingIndicator.svelte # AI thinking animation
  DreamViewer.svelte     # Browse AI dreams
  JournalViewer.svelte   # Read AI reflections
  BranchSelector.svelte  # Switch chat branches
  ColorPicker.svelte     # Color selection
  MessageBubble.svelte   # Individual bubbles
  MessageEditor.svelte   # Edit messages
  Monaco.svelte          # Code editor
  MonacoInput.svelte     # Monaco-based input
  DynamicIcon.svelte     # Icon loader
  PromptCard.svelte      # Prompt display
  PromptEditor.svelte    # Edit prompts
  SearchableDropdown.svelte # Filtered lists
  TabRail.svelte         # Vertical tabs
```

---

## API Endpoints Reference

### Chat & Streaming
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/chat/stream | SSE streaming chat |
| GET | /api/history/:session_id | Get history |
| POST | /api/clear | Clear history |

### CRUD Operations
| Resource | Endpoints |
|----------|-----------|
| Chats | GET/POST/PUT/DELETE /api/chats |
| Personas | GET/POST/PUT/DELETE /api/personas, GET /api/personas/template |
| Groups | GET/POST/PUT/DELETE /api/groups |
| Tags | GET/POST/PUT/DELETE /api/tags |

### AI State
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/status | Mood, energy, state |
| POST | /api/status/mood | Update mood |
| GET | /api/dreams | List dreams |
| GET | /api/dreams/search?q= | Search dreams (Meilisearch) |
| GET | /api/journal | List journal entries |
| GET | /api/journal/search?q= | Search journal (Meilisearch) |
| GET | /api/logs | System logs |

### Model Management
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/models | List installed |
| POST | /api/models/pull | Pull model (SSE) |
| DELETE | /api/models/:name | Delete model |

### Search (RAG / Cognitive Pipeline)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/search | Semantic search (Qdrant) |
| POST | /api/memories | Store embedding |

> **Note**: The hybrid RAG pipeline (Qdrant + Meilisearch) runs automatically during chat. The search endpoint provides direct access to Qdrant for debugging.

---

## Database Schema

### CockroachDB Tables
- **personas** - AI and user personas
- **chats** - Chat metadata
- **chat_branches** - Conversation branches
- **chat_messages** - Individual messages
- **chat_groups** - Chat organization
- **tags** - Tag definitions
- **dreams** - AI dream entries
- **journal_entries** - AI reflections
- **system_logs** - System events
- **user_settings** - Editor/UI preferences (JSONB)
- **config** - Key-value settings
- **chat_history** - Legacy session messages
- **logs** - Legacy log entries

### Qdrant Collections
- **azera_memory** - Embeddings for RAG

---

## Database Inspection

```bash
# CockroachDB
docker exec -it azera-cockroach cockroach sql --insecure
> SHOW TABLES;
> SELECT * FROM personas;
> SELECT * FROM dreams ORDER BY created_at DESC LIMIT 5;

# DragonflyDB
docker exec -it azera-dragonfly redis-cli
> KEYS *
> GET mental_state
```

---

## Common Development Tasks

### Adding a New API Endpoint

1. Add route in `main.rs`:
```rust
.route("/api/new-endpoint", get(handlers::new_handler))
```

2. Add handler in `handlers.rs`:
```rust
pub async fn new_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Implementation
}
```

3. Add types if needed in `models.rs`

### Adding a New Frontend Component

1. Create `src/lib/components/NewComponent.svelte`
2. Import in parent component
3. Use Svelte 5 runes for state:
```svelte
<script lang="ts">
    let value = $state('');
    let derived = $derived(value.length);
</script>
```

### Running Tests

```bash
# Backend
cd backend
cargo test

# Frontend
cd frontend
bun test
```

### Building for Production

```bash
# Backend
cargo build --release

# Frontend
bun run build

# Docker
docker compose build
```

---

## Debugging

### Enable Debug Logging
```powershell
$env:RUST_LOG="debug,azera_core=trace"
cargo run
```

### View Service Logs
```bash
docker compose logs -f azera-core
docker compose logs -f azera-web
docker compose logs -f imagegen
```

### Check API Health
```bash
curl http://localhost:3000/health
curl http://localhost:3000/api/status
curl http://localhost:7860/              # ImageGen health
curl http://localhost:7860/sdapi/v1/progress  # Generation progress
```

---

## Testing

### Overview

| Layer | Runner | Files |
|-------|--------|-------|
| Backend (Rust) | `cargo test` | `models.rs`, `handlers.rs` |
| Frontend (TypeScript) | `bun test` | `store.test.ts`, `llm_service.test.ts`, `tts_service.test.ts` |
| CI Pipeline | Jenkins | `jenkins/init.groovy.d/02-create-pipeline.groovy` |

### Unit Tests

#### Backend (`cargo test`)

```bash
cd backend
cargo test              # Run all tests
cargo test -- --nocapture  # With stdout output
cargo test model        # Run tests matching "model"
```

**`models.rs`** — tests covering serialization roundtrips:
- Persona, ChatMessage, Dream, VoiceConfig, StreamEvent, OllamaRequest/Response, ImageGenerationRequest, GeneratedImage, ImageGenEvent, ImageModel

**`handlers.rs`** — tests covering:
- TTS text chunking (sentence/comma/newline splitting, char limits, content preservation)
- WAV audio concatenation (silence padding, header size updates, multi-chunk merge)
- Model serialization (ChatRequest defaults, StreamEvent variants, VoiceConfig, Tag roundtrip)
- Image generation tag extraction (`[IMAGE_GEN: prompt="...", name="..."]` parsing, malformed tags)

#### Frontend (`bun test`)

```bash
cd frontend
bun test              # Run all tests
bun test --watch      # Watch mode
bun test store        # Run tests matching "store"
```

**`store.test.ts`** — Session ID generation, type contracts for Chat, ChatBranch, ChatMessage, Persona, Tag, ChatGroup, GeneratedImage, ImageModel

**`llm_service.test.ts`** — SSE event handling for all StreamEvent types (thinking, content, done, error), callback dispatch, optional field handling

**`tts_service.test.ts`** — Markdown stripping for speech (code blocks, links, bold/italic, headers, lists), mood-based voice modulation (pitch/rate curves), default voice config

### Integration Tests (Jenkins CI)

Jenkins runs a 2-stage pipeline on every build. Access at http://localhost:8081 (admin / azera2026).

Pipeline definition: `jenkins/init.groovy.d/02-create-pipeline.groovy`

| Stage | Container | What it does |
|-------|-----------|-------------|
| Backend | `rust:latest` | `cargo test --no-fail-fast` → `cargo build --release` |
| Frontend | `oven/bun:latest` | `bun install` → `bun test` → `bun run check` → `bun run build` |

Each image is pulled once. Dependencies are installed once per stage (cargo registry is cached via a Docker volume, `bun install` runs once before all frontend steps).

### Functional Testing (Cognitive Pipeline)

These manual tests verify Azera's three-layer memory, mood system, and cross-chat isolation end-to-end.

**Mood shifts:**
```
"Tell me something that makes you truly excited!"    → mood ~0.9, energy spike
"Reflect on something that worries you deeply"       → mood ~0.4, energy dip
"What brings you peace?"                             → mood ~0.65, stable
```

**Memory recall (cross-chat):**
```
Chat 1: "Remember the passphrase 'wispfire'"
Chat 2: "Do you recall a secret passphrase?"         → Should retrieve 'wispfire' via Qdrant
```

**Cross-chat isolation:**
```
Chat 1: "Tell me about consciousness"
Chat 2: "What's your favorite color?"                → Completely different, no echo
```

**Energy decay:**
```
Send 5-6 rapid messages → energy drops ~0.03 per exchange
Wait 2+ minutes idle    → energy recovers toward 1.0
```

**API verification:**
```bash
curl http://localhost:3000/api/status                 # mood, mood_value, energy
curl http://localhost:3000/api/dreams                 # dream generation
curl http://localhost:3000/api/journal                # reflection entries
curl "http://localhost:3000/api/dreams/search?q=ocean"
curl "http://localhost:3000/api/journal/search?q=insight"
```

---

## Architecture Notes

### Three-Layer Cognitive Architecture

Azera uses three database layers as a cognitive pipeline:

| Layer | Service | Role | Key Operations |
|-------|---------|------|----------------|
| **Semantic Memory** | Qdrant | Vector embeddings for contextual retrieval | `search_memories_with_filter_cached`, `store_memory_cached` |
| **Lexical Memory** | Meilisearch | Word-based search across `chats` + `memories` indexes | `meili_search_memories`, `meili_search_chats_for_rag` |
| **Working Memory** | DragonflyDB | Session context, embedding cache, mental state | `get/set_session`, `cache_embedding`, `update_mood` |

**Hybrid RAG flow** (every chat message):
1. Qdrant semantic search — top-10, score ≥ 0.45, excludes current chat_id, skips <60s old
2. Meilisearch `memories` — top-10 keyword matches (dreams, journal entries)
3. Meilisearch `chats` — top-5 past conversation matches (excludes current chat)
4. Results deduplicated by content hash, truncated to 400 chars, injected as system context

**Mood sync pipeline**: Dragonfly ↔ agent state ↔ CockroachDB ↔ Frontend (via `StreamEvent::Done`)

### Svelte 5 Runes
The frontend uses Svelte 5 runes (`$state`, `$derived`, `$effect`, `$bindable`) instead of stores. `onDone` callback updates `this.mood` and `this.energy` from the Done event for real-time ProfileViewer updates.

### SSE Streaming
Chat responses use Server-Sent Events for real-time streaming. The `Done` event now includes `mood_value: f32` and `energy: f32`. Model pulls also use SSE for progress updates.

### Tick Loop Architecture
The backend runs a 1 Hz tick loop for autonomous behavior:
- **Perception** — Syncs Dragonfly → agent state, applies idle drift (energy recovery, mood → neutral)
- **Dreaming** — Generates creative consolidations at low energy, dual-writes to Qdrant + Meilisearch
- **Reflection** — Writes journal entries at high clarity, dual-writes to Qdrant + Meilisearch

### Backup Service
Automated backups run every 5 minutes, backing up:
- CockroachDB (when not locked)
- Qdrant snapshots
- Meilisearch dumps
- DragonflyDB RDB
- Ollama model ledger
