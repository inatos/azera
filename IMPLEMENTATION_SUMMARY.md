# Azera Implementation Summary

## Overview
Azera is an AGI chat application featuring emotional intelligence, memory persistence, and self-reflection. Built with Rust (Axum) backend and Svelte 5 (SvelteKit) frontend.

---

## Core Features

### 1. Streaming Chat
- SSE-based real-time token streaming
- Multi-model support (any Ollama model)
- Contextual memory with RAG
- Conversation branching

### 2. Persona System
- Multiple AI personas with distinct personalities
- Custom system prompts (markdown-rendered "Profiles") and voices
- Default AI personas seeded on startup: **Azera**, **Areza**
- Default user persona: **Protag** (id: `protag`)
- Persona `.md` files regenerated from DB on startup if missing
- Template endpoint for new persona creation (`GET /api/personas/template`)
- Frontend CRUD synced to backend API (fire-and-forget)

### 3. Mental State Management
- Mood tracking (valence + arousal)
- Energy simulation with decay
- Focus state management
- Emotional responses to conversations

### 4. Dream System
- Automatic dream generation
- Processing of unresolved emotions
- Creative consolidation
- Viewable dream history

### 5. Journal System
- Self-reflection entries
- Daily summaries
- Insight generation
- Browsable journal viewer

### 6. Cognitive Memory Pipeline (Three-Layer Architecture)
- **Qdrant (Semantic)** — Vector embeddings for contextual memory retrieval
- **Meilisearch (Lexical)** — Two indexes (`chats` + `memories`) for word-based search with filters
- **DragonflyDB (Working Memory)** — Session context (24h TTL), embedding cache (7d TTL), mental state sync
- Hybrid RAG: 3-source merge (Qdrant top-10 + Meili memories top-10 + Meili chats top-5)
- Persona isolation: Meilisearch searches filter by `ai_persona_id` to prevent cross-persona leakage
- Cross-chat isolation: `must_not` chat_id filter, 60s recency filter, 0.45 score threshold
- Embedding cache: SHA256-keyed, base64-encoded f32 vectors in Dragonfly
- Dreams/reflections dual-written to both Qdrant and Meilisearch

### 7. Model Management
- List installed Ollama models (embedding-only models filtered from API)
- Pull new models with SSE progress
- Delete unused models
- Dynamic model selection in chat

### 8. Image Generation (Canvas)
- Animagine XL 3.1 powered by HuggingFace Diffusers
- Real-time step-by-step progress via SSE
- Dedicated Canvas page (`/canvas`)
- Image gallery with download/delete
- Reference image upload for img2img variations
- Custom FastAPI server with CUDA acceleration

### 9. Settings & Customization
- Dynamic model selector (from installed Ollama models)
- Show Thinking toggle (display AI reasoning blocks)
- Send on Enter toggle (Enter vs Ctrl+Enter to send)
- Editor theme and font settings
- User preferences persisted to localStorage (`azera_preferences`)
- Per-component configuration

---

## Technical Architecture

### Backend Services
| Service | Port | Purpose |
|---------|------|---------|
| azera-core | 3000 | Main API server |
| azera-web | 5173 | SvelteKit frontend |
| CockroachDB | 26257 | Persistent storage |
| DragonflyDB | 6379 | Working memory (session context, embedding cache, mental state) |
| Qdrant | 6333 | Semantic memory (vector embeddings, RAG) |
| Meilisearch | 7700 | Lexical memory (chats + memories indexes) |
| Ollama | 11434 | LLM inference |
| XTTS | 8020 | Voice synthesis |
| ImageGen | 7860 | AI image generation |
| Jenkins | 8081 | CI/CD pipeline (admin / azera2026) |

### API Surface (53 Endpoints)

**Chat Operations**
- `POST /api/chat` - Streaming chat (SSE)
- `GET /api/history/:session_id` - Get conversation
- `POST /api/clear` - Clear session

**Chats CRUD**
- `GET /api/chats` - List all
- `POST /api/chats` - Create new
- `GET /api/chats/:id` - Get one
- `PUT /api/chats/:id` - Update
- `DELETE /api/chats/:id` - Delete

**Personas CRUD**
- `GET /api/personas` - List all
- `POST /api/personas` - Create new
- `GET /api/personas/template` - Get template for new persona
- `GET /api/personas/:id` - Get one
- `PUT /api/personas/:id` - Update
- `DELETE /api/personas/:id` - Delete

**Groups CRUD**
- `GET /api/groups` - List all
- `POST /api/groups` - Create new
- `GET /api/groups/:id` - Get one
- `PUT /api/groups/:id` - Update
- `DELETE /api/groups/:id` - Delete

**Tags CRUD**
- `GET /api/tags` - List all
- `POST /api/tags` - Create new
- `GET /api/tags/:id` - Get one
- `PUT /api/tags/:id` - Update
- `DELETE /api/tags/:id` - Delete

**AI State**
- `GET /api/status` - Mental state
- `POST /api/status/mood` - Update mood
- `GET /api/dreams` - List dreams
- `GET /api/dreams/search?q=` - Search dreams (Meilisearch)
- `POST /api/dreams/import` - Import dream archives
- `GET /api/journal` - List entries
- `GET /api/journal/search?q=` - Search journal (Meilisearch)
- `POST /api/journal/trigger` - Trigger reflection
- `POST /api/journal/import` - Import journal archives
- `GET /api/logs` - System logs

**Model Management**
- `GET /api/models` - List models
- `POST /api/models/pull` - Pull (SSE)
- `DELETE /api/models/:name` - Delete

**Image Generation**
- `POST /api/images/generate` - Generate image (SSE with progress)
- `GET /api/images` - List generated images
- `GET /api/images/models` - List image gen models
- `POST /api/images/upload-reference` - Upload reference image
- `GET /api/images/references/:filename` - Get reference image
- `GET /api/images/:filename` - Get image
- `DELETE /api/images/:filename` - Delete image

**Voice**
- `POST /api/tts/synthesize` - Synthesize speech
- `POST /api/voice-samples/upload` - Upload voice sample
- `GET /api/voice-samples/:filename` - Get voice sample

**Settings**
- `GET /api/settings` - Get all settings
- `PUT /api/settings/editor` - Update editor settings
- `PUT /api/settings/ui` - Update UI settings

**Search & Memory**
- `POST /api/search` - Semantic search
- `POST /api/memories` - Store embedding

**Health**
- `GET /health` - Service health

---

## Frontend Components (23)

### Core UI
- **ChatInput.svelte** - Input with model selector
- **ChatMessage.svelte** - Message rendering
- **MessageBubble.svelte** - Individual bubbles
- **MessageEditor.svelte** - Edit messages
- **ThinkingIndicator.svelte** - AI thinking state

### Image Generation
- **ImageGenerator.svelte** - AI image creation with progress tracking
- **ImageGallery.svelte** - Browse and manage generated images

### Navigation
- **Sidebar.svelte** - Main navigation
- **TabRail.svelte** - Vertical tabs
- **SearchableDropdown.svelte** - Filtered lists

### Persona Management
- **PersonaEditor.svelte** - Create/edit personas
- **ProfileViewer.svelte** - View profiles

### AI State Viewers
- **DreamViewer.svelte** - Browse dreams
- **JournalViewer.svelte** - Read reflections

### Model Management
- **ModelManager.svelte** - Ollama model UI

### Settings
- **EditorConfig.svelte** - Editor/UI settings
- **ColorPicker.svelte** - Color selection

### Branch Management
- **BranchSelector.svelte** - Switch branches

### Utilities
- **Monaco.svelte** - Code editor
- **MonacoInput.svelte** - Monaco-based input
- **DynamicIcon.svelte** - Icon loader
- **PromptCard.svelte** - Prompt display
- **PromptEditor.svelte** - Edit prompts

---

## State Management

### Svelte 5 Runes
```typescript
class AppState {
    // State
    personas = $state<Persona[]>([]);
    selectedPersona = $state<Persona | null>(null);
    messages = $state<Message[]>([]);
    models = $state<Model[]>([]);
    isLoadingModels = $state(false);
    showThinking = $state(true);      // AI reasoning blocks
    sendOnEnter = $state(false);      // Enter vs Ctrl+Enter
    
    // Derived
    aiPersonas = $derived(this.personas.filter(p => !p.is_user));
    userPersonas = $derived(this.personas.filter(p => p.is_user));
}
```

### Key Methods
- `fetchModels()` - Load Ollama models
- `selectPersona()` - Switch active persona
- `sendMessage()` - Stream chat response
- `loadHistory()` - Get conversation
- `savePreferences()` - Persist user settings to localStorage
- `loadPreferences()` - Restore user settings on init

---

## Database Schema

### Personas Table
```sql
CREATE TABLE personas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50),
    avatar_url TEXT,
    model VARCHAR(255),
    system_prompt TEXT,
    voice TEXT,
    is_user BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);
```

### Chats Table
```sql
CREATE TABLE chats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255),
    ai_persona_id UUID REFERENCES personas(id),
    user_persona_id UUID REFERENCES personas(id),
    group_id UUID,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);
```

### Messages Table
```sql
CREATE TABLE chat_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID REFERENCES chats(id),
    branch_id UUID,
    role VARCHAR(50),
    content TEXT,
    parent_id UUID,
    model VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT now()
);
```

### Dreams Table
```sql
CREATE TABLE dreams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content TEXT,
    emotion VARCHAR(100),
    themes TEXT[],
    created_at TIMESTAMPTZ DEFAULT now()
);
```

### Journal Entries
```sql
CREATE TABLE journal_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content TEXT,
    mood VARCHAR(100),
    insights TEXT[],
    created_at TIMESTAMPTZ DEFAULT now()
);
```

---

## Tick Loop

The backend runs a 1 Hz tick loop for autonomous AI behavior:

```
1. Perception Processing  - Sync Dragonfly → agent state, apply idle drift
                           (energy recovery, mood → neutral, focus decay)
2. Cognitive Processing   - Think and decide
3. Emotional Processing   - Update mood/energy in Dragonfly + agent state
4. Dream Processing       - Generate dreams at low energy
                           (dual-write: Qdrant + Meilisearch)
5. Reflection Processing  - Write journal at high clarity
                           (dual-write: Qdrant + Meilisearch)
6. Output Generation      - Produce responses
```

### Mood Sync Pipeline
```
LLM Response → mood inference → Dragonfly update_mood()
    → perception_system syncs Dragonfly → agent state
    → StreamEvent::Done { mood_value, energy }
    → Frontend onDone → this.mood, this.energy
    → ProfileViewer bar updates in real-time
```

---

## Backup System

Automated backups every 5 minutes:
- CockroachDB SQL dumps
- Qdrant snapshots
- Meilisearch data
- DragonflyDB RDB
- Ollama model ledger

Location: `logs/backups/`