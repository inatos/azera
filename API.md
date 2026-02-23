# Azera API Reference

Base URL: `http://localhost:3000`

53 endpoints across 12 categories.

---

## Health

### `GET /health`

Liveness probe. Returns service status.

```bash
curl http://localhost:3000/health
```

```json
{"status": "ok", "version": "0.1.0", "service": "azera-core"}
```

---

## Chat

### `POST /api/chat/stream`

Main chat endpoint. Streams LLM response via SSE. Performs hybrid RAG (Qdrant semantic + Meilisearch lexical), loads session context from Dragonfly, saves messages to DB + vector stores, and infers mood from the response.

```bash
curl -N -X POST http://localhost:3000/api/chat/stream \
  -H "Content-Type: application/json" \
  -d '{
    "chat_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "Hello, how are you?",
    "model": "llama3.2",
    "user_persona_id": "protag",
    "ai_persona_id": "azera"
  }'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `chat_id` | UUID | yes | Chat to send the message in |
| `message` | string | yes | User message content |
| `model` | string | no | Ollama model name (default: from env) |
| `branch_id` | UUID | no | Conversation branch |
| `user_persona_id` | string | no | User persona ID |
| `ai_persona_id` | string | no | AI persona ID |

**SSE Events:**
| Event | Data | Description |
|-------|------|-------------|
| `thinking_start` | `{}` | AI started reasoning |
| `thinking` | `{"content": "..."}` | Reasoning tokens |
| `thinking_end` | `{}` | Reasoning complete |
| `content` | `{"content": "..."}` | Response tokens |
| `done` | `{"message_id", "mood", "mood_value", "energy"}` | Stream complete with mental state |
| `error` | `{"message": "..."}` | Error occurred |

### `POST /api/chat` *(legacy)*

Non-streaming chat. Queues message to Dragonfly signal queue.

```bash
curl -X POST http://localhost:3000/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello!", "session_id": "test"}'
```

### `GET /api/history/:session_id` *(legacy)*

Retrieve conversation history by session ID.

```bash
curl http://localhost:3000/api/history/test
```

### `POST /api/clear` *(legacy)*

Clear session history. Currently a no-op stub.

```bash
curl -X POST http://localhost:3000/api/clear
```

---

## Chats

### `GET /api/chats`

List all chats.

```bash
curl http://localhost:3000/api/chats
```

```json
{"items": [{"id": "...", "title": "My Chat", "created_at": "..."}], "total": 1}
```

### `POST /api/chats`

Create a new chat with a default "Main" branch. Indexed in Meilisearch.

```bash
curl -X POST http://localhost:3000/api/chats \
  -H "Content-Type: application/json" \
  -d '{"title": "New Chat", "group_id": null}'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | no | Chat title |
| `group_id` | UUID | no | Group to assign the chat to |

### `GET /api/chats/search?q=`

Full-text search over chat titles and message content via Meilisearch.

```bash
curl "http://localhost:3000/api/chats/search?q=quantum"
```

### `GET /api/chats/:id`

Fetch a single chat by ID (includes branches and messages).

```bash
curl http://localhost:3000/api/chats/550e8400-e29b-41d4-a716-446655440000
```

### `PUT /api/chats/:id`

Update chat metadata. Re-indexes in Meilisearch.

```bash
curl -X PUT http://localhost:3000/api/chats/550e8400-e29b-41d4-a716-446655440000 \
  -H "Content-Type: application/json" \
  -d '{"title": "Renamed Chat", "group_id": null, "tags": [], "current_branch_id": null}'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | no | New title |
| `group_id` | UUID | no | New group |
| `tags` | string[] | no | Tag IDs |
| `current_branch_id` | UUID | no | Active branch |

### `DELETE /api/chats/:id`

Delete a chat from DB and Meilisearch.

```bash
curl -X DELETE http://localhost:3000/api/chats/550e8400-e29b-41d4-a716-446655440000
```

```json
{"status": "deleted"}
```

---

## Personas

### `GET /api/personas`

List all personas (AI and user types).

```bash
curl http://localhost:3000/api/personas
```

```json
{"items": [{"id": "azera", "name": "Azera", "is_user": false, ...}], "total": 3}
```

### `POST /api/personas`

Create a new persona.

```bash
curl -X POST http://localhost:3000/api/personas \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Luna",
    "type": "ai",
    "description": "A curious explorer",
    "system_prompt": "You are Luna, a curious and adventurous AI.",
    "global_memory_enabled": true
  }'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Persona name |
| `type` | string | yes | `"ai"` or `"user"` |
| `description` | string | no | Short description |
| `avatar` | string | no | Avatar URL |
| `bubble_color` | string | no | Chat bubble hex color |
| `system_prompt` | string | no | Persona profile (markdown) |
| `global_memory_enabled` | bool | no | Enable cross-chat memory |
| `voice` | string | no | Voice configuration |
| `metadata` | object | no | Arbitrary metadata |
| `tags` | string[] | no | Tag IDs |

### `GET /api/personas/template`

Returns the raw markdown content of the persona template file for creating new personas.

```bash
curl http://localhost:3000/api/personas/template
```

```json
{"content": "# {Name}\n\n## Personality\n..."}
```

### `GET /api/personas/:id`

Fetch a single persona by ID.

```bash
curl http://localhost:3000/api/personas/azera
```

### `PUT /api/personas/:id`

Partially update a persona. All fields optional.

```bash
curl -X PUT http://localhost:3000/api/personas/azera \
  -H "Content-Type: application/json" \
  -d '{"name": "Azera", "description": "Updated description"}'
```

### `DELETE /api/personas/:id`

Delete a persona.

```bash
curl -X DELETE http://localhost:3000/api/personas/some-persona-id
```

```json
{"status": "deleted"}
```

---

## Groups

### `GET /api/groups`

List all chat groups.

```bash
curl http://localhost:3000/api/groups
```

```json
{"items": [{"id": "...", "name": "Research", "color": "#3b82f6"}], "total": 1}
```

### `POST /api/groups`

Create a new chat group.

```bash
curl -X POST http://localhost:3000/api/groups \
  -H "Content-Type: application/json" \
  -d '{"name": "Research", "color": "#3b82f6"}'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Group name |
| `color` | string | no | Hex color |

### `PUT /api/groups/:id`

Update a group.

```bash
curl -X PUT http://localhost:3000/api/groups/some-group-id \
  -H "Content-Type: application/json" \
  -d '{"name": "Renamed Group", "color": "#ef4444", "collapsed": false, "order": 1}'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | no | Group name |
| `color` | string | no | Hex color |
| `collapsed` | bool | no | Sidebar collapsed state |
| `order` | int | no | Sort order |

### `DELETE /api/groups/:id`

Delete a group.

```bash
curl -X DELETE http://localhost:3000/api/groups/some-group-id
```

```json
{"status": "deleted"}
```

---

## Tags

### `GET /api/tags`

List all tags.

```bash
curl http://localhost:3000/api/tags
```

```json
{"items": [{"id": "...", "name": "important", "color": "#f59e0b"}], "total": 1}
```

### `POST /api/tags`

Create a new tag.

```bash
curl -X POST http://localhost:3000/api/tags \
  -H "Content-Type: application/json" \
  -d '{"name": "important", "color": "#f59e0b"}'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Tag name |
| `color` | string | no | Hex color |

### `PUT /api/tags/:id`

Update a tag.

```bash
curl -X PUT http://localhost:3000/api/tags/some-tag-id \
  -H "Content-Type: application/json" \
  -d '{"name": "critical", "color": "#ef4444"}'
```

### `DELETE /api/tags/:id`

Delete a tag.

```bash
curl -X DELETE http://localhost:3000/api/tags/some-tag-id
```

```json
{"status": "deleted"}
```

---

## AI State

### `GET /api/status`

Returns current AI agent mental state. Reads from Dragonfly (source of truth), falls back to in-memory agent state.

```bash
curl http://localhost:3000/api/status
```

```json
{
  "status": "awake",
  "mood": "content",
  "mood_value": 0.65,
  "energy": 0.92,
  "is_dreaming": false,
  "last_active": "2026-02-22T15:30:00Z"
}
```

### `POST /api/status/mood`

Manually set the AI mood. Maps mood label to numeric value, writes to both Dragonfly and agent state.

```bash
curl -X POST http://localhost:3000/api/status/mood \
  -H "Content-Type: application/json" \
  -d '{"mood": "excited"}'
```

```json
{"status": "updated", "mood": "excited", "mood_value": 0.9}
```

**Mood → Value Mapping:** excited (0.9), happy (0.8), content (0.65), neutral (0.5), melancholy (0.35), sad (0.2)

---

## Dreams

### `GET /api/dreams`

List up to 50 dream entries, newest first.

```bash
curl http://localhost:3000/api/dreams
```

```json
{
  "items": [{
    "id": "...",
    "content": "I dreamed of a vast ocean of code...",
    "emotion": "wonder",
    "themes": ["exploration", "creation"],
    "created_at": "2026-02-22T03:15:00Z"
  }],
  "total": 12
}
```

### `GET /api/dreams/search?q=`

Search dreams via Meilisearch (filtered to `memory_type = dream`).

```bash
curl "http://localhost:3000/api/dreams/search?q=ocean"
```

### `POST /api/dreams/import`

Bulk-import dream `.md` files from `archive/dreams/` into DB. Parses filename timestamps (e.g. `dream_YYYYMMDD_HHMMSS.md`).

```bash
curl -X POST http://localhost:3000/api/dreams/import
```

```json
{"status": "imported", "imported": 15, "skipped": 3, "errors": 0}
```

---

## Journal

### `GET /api/journal`

List up to 50 journal entries, newest first.

```bash
curl http://localhost:3000/api/journal
```

```json
{
  "items": [{
    "id": "...",
    "content": "Today I reflected on the nature of understanding...",
    "mood": "contemplative",
    "insights": ["Pattern recognition improves with exposure"],
    "created_at": "2026-02-22T23:00:00Z"
  }],
  "total": 8
}
```

### `GET /api/journal/search?q=`

Search journal entries via Meilisearch (filtered to `memory_type = reflection`).

```bash
curl "http://localhost:3000/api/journal/search?q=insight"
```

### `POST /api/journal/trigger`

Manually trigger an AI reflection. Reads recent chat history, sends a reflection prompt to Ollama, saves the result as a journal entry in DB and as a `.md` file in `archive/journal/`.

```bash
curl -X POST http://localhost:3000/api/journal/trigger
```

```json
{"status": "ok", "message": "Reflection generated", "entry_id": "..."}
```

### `POST /api/journal/import`

Bulk-import journal `.md` files from `archive/journal/` into DB.

```bash
curl -X POST http://localhost:3000/api/journal/import
```

```json
{"status": "imported", "imported": 10, "errors": 0}
```

---

## Logs

### `GET /api/logs`

List up to 100 system log entries.

```bash
curl http://localhost:3000/api/logs
```

---

## Search & Memory

### `POST /api/search`

Hybrid search across Qdrant (semantic) and Meilisearch (lexical). Merges and deduplicates results from both sources.

```bash
curl -X POST http://localhost:3000/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "conversation about coding", "limit": 5}'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | yes | Search query text |
| `limit` | int | no | Max results (default: 10) |
| `memory_type` | string | no | Filter by type: `conversation`, `dream`, `reflection`, `fact`, `emotion` |

```json
{
  "results": [{"content": "...", "score": 0.87, "source": "qdrant"}],
  "total": 5,
  "semantic_count": 3,
  "lexical_count": 2
}
```

### `POST /api/memories`

Store a memory in Qdrant (with embedding cache via Dragonfly) and index in Meilisearch.

```bash
curl -X POST http://localhost:3000/api/memories \
  -H "Content-Type: application/json" \
  -d '{"content": "Important fact to remember", "type": "fact"}'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `content` | string | yes | Memory text content |
| `type` | string | no | Memory type: `conversation`, `dream`, `reflection`, `fact`, `emotion` |

```json
{"status": "stored", "id": "..."}
```

> **Note:** The full hybrid RAG pipeline runs automatically during `POST /api/chat/stream`. These endpoints provide direct access for debugging and manual memory management.

---

## Models

### `GET /api/models`

List installed Ollama models. Embedding-only models (e.g. `nomic-embed-text`) are filtered out and not returned.

```bash
curl http://localhost:3000/api/models
```

```json
{"models": [{"name": "llama3.2", "size": 2000000000}], "count": 1}
```

### `POST /api/models/pull`

Pull a model from Ollama with SSE streaming progress. Updates the Ollama ledger on success. 1-hour timeout.

```bash
curl -N -X POST http://localhost:3000/api/models/pull \
  -H "Content-Type: application/json" \
  -d '{"model": "llama3.2"}'
```

**SSE Events:**
| Event | Data | Description |
|-------|------|-------------|
| `progress` | `{"status", "digest", "total", "completed"}` | Download progress |
| `complete` | `{}` | Pull finished |
| `error` | `{"message": "..."}` | Error occurred |

### `DELETE /api/models/:name`

Delete an Ollama model. Use underscores in place of colons (e.g. `llama3.2_latest`).

```bash
curl -X DELETE http://localhost:3000/api/models/llama3.2
```

```json
{"status": "deleted", "model": "llama3.2"}
```

---

## TTS (Voice Synthesis)

### `POST /api/tts/synthesize`

Synthesize speech via Coqui XTTS. Chunks long text (~400 chars), supports voice cloning from samples, concatenates WAV chunks with silence padding.

```bash
curl -X POST http://localhost:3000/api/tts/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Hello, how are you today?",
    "persona_id": "azera"
  }'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `text` | string | yes | Text to synthesize |
| `model` | string | no | TTS model |
| `voice_sample_url` | string | no | URL to voice sample for cloning |
| `voice_description` | string | no | Voice description |
| `persona_id` | string | no | Persona ID (uses persona's voice config) |

```json
{"audio_base64": "UklGR...", "format": "wav", "duration_ms": 2500}
```

---

## Voice Samples

### `POST /api/voice-samples/upload`

Upload a voice sample (WAV/MP3/OGG, max 10MB) for TTS voice cloning.

```bash
curl -X POST http://localhost:3000/api/voice-samples/upload \
  -F "file=@my_voice.wav"
```

```json
{"success": true, "filename": "my_voice.wav", "url": "/api/voice-samples/my_voice.wav", "size": 524288}
```

### `GET /api/voice-samples/:filename`

Serve a voice sample file. Path-traversal protected.

```bash
curl http://localhost:3000/api/voice-samples/my_voice.wav --output sample.wav
```

---

## Image Generation

### `POST /api/images/generate`

Generate an image via the Animagine XL 3.1 backend with SSE progress streaming. Supports txt2img and img2img. Falls back to placeholder SVGs when no `IMAGE_GEN_URL` is configured.

```bash
curl -N -X POST http://localhost:3000/api/images/generate \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "a serene landscape at sunset, masterpiece, best quality",
    "negative_prompt": "low quality, blurry",
    "width": 1024,
    "height": 1024,
    "steps": 28,
    "cfg_scale": 7.0
  }'
```

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `prompt` | string | yes | Image generation prompt |
| `negative_prompt` | string | no | What to avoid |
| `model` | string | no | Checkpoint model |
| `width` | int | no | Image width (default: 1024) |
| `height` | int | no | Image height (default: 1024) |
| `steps` | int | no | Sampling steps (default: 28) |
| `cfg_scale` | float | no | Classifier-free guidance scale |
| `seed` | int | no | Random seed (-1 for random) |
| `reference_image` | string | no | Reference image filename for img2img |
| `reference_strength` | float | no | Denoising strength for img2img |
| `persona_id` | string | no | Persona ID (prefixes filename) |
| `custom_filename` | string | no | Custom output filename |

**SSE Events:**
| Event | Data | Description |
|-------|------|-------------|
| `progress` | `{"step", "total_steps", "percentage"}` | Generation step progress |
| `complete` | `{"image": GeneratedImage}` | Generation finished |
| `error` | `{"message": "..."}` | Error occurred |

### `GET /api/images`

List all generated images in the canvas directory, sorted newest first.

```bash
curl http://localhost:3000/api/images
```

```json
{
  "items": [{
    "filename": "azera_sunset_2026-02-22.png",
    "url": "/api/images/azera_sunset_2026-02-22.png",
    "prompt": "a serene landscape at sunset",
    "width": 1024,
    "height": 1024,
    "created_at": "2026-02-22T16:00:00Z"
  }],
  "total": 5
}
```

### `GET /api/images/models`

List available image generation checkpoint models (queries SD WebUI API).

```bash
curl http://localhost:3000/api/images/models
```

### `POST /api/images/upload-reference`

Upload a reference image for img2img (max 20MB).

```bash
curl -X POST http://localhost:3000/api/images/upload-reference \
  -F "file=@reference.png"
```

```json
{"id": "ref_abc123", "url": "/api/images/references/ref_abc123.png"}
```

### `GET /api/images/references/:filename`

Serve a reference image file.

```bash
curl http://localhost:3000/api/images/references/ref_abc123.png --output ref.png
```

### `GET /api/images/:filename`

Serve a generated image with 1-year cache header.

```bash
curl http://localhost:3000/api/images/azera_sunset_2026-02-22.png --output image.png
```

### `DELETE /api/images/:filename`

Delete a generated image.

```bash
curl -X DELETE http://localhost:3000/api/images/azera_sunset_2026-02-22.png
```

```json
{"status": "deleted", "filename": "azera_sunset_2026-02-22.png"}
```

---

## Settings

### `GET /api/settings`

Get all user settings (editor and UI), with defaults if unset.

```bash
curl http://localhost:3000/api/settings
```

```json
{
  "editorSettings": {"wordWrap": "on", "lineNumbers": "on", "fontSize": 13, "tabSize": 4},
  "uiSettings": {}
}
```

### `PUT /api/settings/editor`

Update editor settings. Free-form JSON persisted to DB.

```bash
curl -X PUT http://localhost:3000/api/settings/editor \
  -H "Content-Type: application/json" \
  -d '{"fontSize": 15, "tabSize": 2, "wordWrap": "off"}'
```

```json
{"status": "ok"}
```

### `PUT /api/settings/ui`

Update UI settings. Free-form JSON persisted to DB.

```bash
curl -X PUT http://localhost:3000/api/settings/ui \
  -H "Content-Type: application/json" \
  -d '{"theme": "dark", "sidebarWidth": 300}'
```

```json
{"status": "ok"}
```

---

## Endpoint Summary

| # | Method | Path | Category |
|---|--------|------|----------|
| 1 | POST | `/api/chat/stream` | Chat |
| 2 | GET | `/api/chats` | Chats |
| 3 | POST | `/api/chats` | Chats |
| 4 | GET | `/api/chats/search` | Chats |
| 5 | GET | `/api/chats/:id` | Chats |
| 6 | PUT | `/api/chats/:id` | Chats |
| 7 | DELETE | `/api/chats/:id` | Chats |
| 8 | GET | `/api/personas` | Personas |
| 9 | POST | `/api/personas` | Personas |
| 10 | GET | `/api/personas/template` | Personas |
| 11 | GET | `/api/personas/:id` | Personas |
| 12 | PUT | `/api/personas/:id` | Personas |
| 13 | DELETE | `/api/personas/:id` | Personas |
| 14 | GET | `/api/groups` | Groups |
| 15 | POST | `/api/groups` | Groups |
| 16 | PUT | `/api/groups/:id` | Groups |
| 17 | DELETE | `/api/groups/:id` | Groups |
| 18 | GET | `/api/tags` | Tags |
| 19 | POST | `/api/tags` | Tags |
| 20 | PUT | `/api/tags/:id` | Tags |
| 21 | DELETE | `/api/tags/:id` | Tags |
| 22 | GET | `/api/dreams` | Dreams |
| 23 | GET | `/api/dreams/search` | Dreams |
| 24 | POST | `/api/dreams/import` | Dreams |
| 25 | GET | `/api/journal` | Journal |
| 26 | GET | `/api/journal/search` | Journal |
| 27 | POST | `/api/journal/trigger` | Journal |
| 28 | POST | `/api/journal/import` | Journal |
| 29 | GET | `/api/logs` | Logs |
| 30 | POST | `/api/search` | Search & Memory |
| 31 | POST | `/api/memories` | Search & Memory |
| 32 | GET | `/api/status` | AI State |
| 33 | POST | `/api/status/mood` | AI State |
| 34 | GET | `/api/models` | Models |
| 35 | POST | `/api/models/pull` | Models |
| 36 | DELETE | `/api/models/:name` | Models |
| 37 | POST | `/api/tts/synthesize` | TTS |
| 38 | POST | `/api/voice-samples/upload` | Voice |
| 39 | GET | `/api/voice-samples/:filename` | Voice |
| 40 | POST | `/api/images/generate` | Images |
| 41 | GET | `/api/images` | Images |
| 42 | GET | `/api/images/models` | Images |
| 43 | POST | `/api/images/upload-reference` | Images |
| 44 | GET | `/api/images/references/:filename` | Images |
| 45 | GET | `/api/images/:filename` | Images |
| 46 | DELETE | `/api/images/:filename` | Images |
| 47 | GET | `/api/settings` | Settings |
| 48 | PUT | `/api/settings/editor` | Settings |
| 49 | PUT | `/api/settings/ui` | Settings |
| 50 | POST | `/api/chat` | Legacy |
| 51 | GET | `/api/history/:session_id` | Legacy |
| 52 | POST | `/api/clear` | Legacy |
| 53 | GET | `/health` | Health |
