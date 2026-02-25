# Azera Quick Reference

## Start Everything

```bash
docker compose up -d
# Wait ~60s, then:
# Web UI: http://localhost:5173
# Canvas: http://localhost:5173/canvas
# API: http://localhost:3000
```

## Local Development

```powershell
# 1. Start infrastructure
docker compose up -d cockroach dragonfly qdrant meilisearch ollama

# 2. Backend (new terminal)
cd backend
$env:DATABASE_URL="postgres://root@localhost:26257/azera?sslmode=disable"
$env:DRAGONFLY_URL="redis://localhost:6379"
$env:QDRANT_URL="http://localhost:6333"
$env:OLLAMA_HOST="http://localhost:11434"
$env:MEILI_URL="http://localhost:7700"
$env:RUST_LOG="info,azera_core=debug"
cargo run

# 3. Frontend (new terminal)
cd frontend
bun install
bun dev
```

## API Endpoints

### Chat
```bash
# Stream chat (SSE)
curl -N -X POST http://localhost:3000/api/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello!", "chat_id": "test", "branch_id": "branch_main_test", "model": "llama3.2"}'

# Get history
curl http://localhost:3000/api/history/test
```

### Personas
```bash
# List personas
curl http://localhost:3000/api/personas

# Get persona template (for new persona creation)
curl http://localhost:3000/api/personas/template

# Create persona
curl -X POST http://localhost:3000/api/personas \
  -H "Content-Type: application/json" \
  -d '{"name": "Assistant", "type": "ai", "system_prompt": "You are helpful."}'
```

### Models
```bash
# List models
curl http://localhost:3000/api/models

# Pull model (SSE)
curl -X POST http://localhost:3000/api/models/pull \
  -H "Content-Type: application/json" \
  -d '{"name": "llama3.2"}'

# Delete model
curl -X DELETE http://localhost:3000/api/models/llama3.2
```

### Chats CRUD
```bash
# List chats
curl http://localhost:3000/api/chats

# Create chat
curl -X POST http://localhost:3000/api/chats \
  -H "Content-Type: application/json" \
  -d '{"title": "New Chat"}'

# Get chat
curl http://localhost:3000/api/chats/{id}

# Update chat
curl -X PUT http://localhost:3000/api/chats/{id} \
  -H "Content-Type: application/json" \
  -d '{"title": "Updated Title"}'

# Delete chat
curl -X DELETE http://localhost:3000/api/chats/{id}
```

### AI State
```bash
# Get mental state (includes mood, mood_value, energy, focus)
curl http://localhost:3000/api/status

# Get dreams
curl http://localhost:3000/api/dreams

# Search dreams
curl "http://localhost:3000/api/dreams/search?q=ocean"

# Get journal
curl http://localhost:3000/api/journal

# Search journal
curl "http://localhost:3000/api/journal/search?q=insight"
```

### Search (RAG / Cognitive Pipeline)
```bash
# Semantic search (queries Qdrant directly)
curl -X POST http://localhost:3000/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "conversation about coding", "limit": 5}'

# Store memory (writes to Qdrant + Meilisearch)
curl -X POST http://localhost:3000/api/memories \
  -H "Content-Type: application/json" \
  -d '{"content": "Important fact to remember", "type": "fact"}'
```

> **Note**: The full hybrid RAG pipeline (Qdrant semantic + Meilisearch lexical) runs automatically during `POST /api/chat/stream`. The search endpoint provides direct Qdrant access for debugging.

### TTS (Voice Synthesis)
```bash
# Synthesize speech
curl -X POST http://localhost:3000/api/tts/synthesize \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello, how are you today?", "persona_id": "optional-persona-id"}'
# Returns: {"audio_base64": "...", "format": "wav", "duration_ms": 2500}
```

### Image Generation
```bash
# Generate image (SSE stream with progress + completion)
curl -X POST http://localhost:3000/api/images/generate \
  -H "Content-Type: application/json" \
  -d '{"prompt": "a serene landscape", "width": 1024, "height": 1024, "steps": 28}'

# List generated images
curl http://localhost:3000/api/images

# Delete image
curl -X DELETE http://localhost:3000/api/images/{filename}
```

### Settings
```bash
# Get all settings
curl http://localhost:3000/api/settings

# Update editor settings
curl -X PUT http://localhost:3000/api/settings/editor \
  -H "Content-Type: application/json" \
  -d '{"theme": "dark"}'
```

## Database Access

```bash
# CockroachDB SQL
docker exec -it azera-cockroach cockroach sql --insecure

# Common queries
SHOW TABLES;
SELECT * FROM personas;
SELECT * FROM chats ORDER BY created_at DESC LIMIT 5;
SELECT * FROM dreams ORDER BY created_at DESC LIMIT 5;
```

## Logs

```bash
# Backend logs
docker compose logs -f azera-core

# Frontend logs  
docker compose logs -f azera-web

# Image generation logs
docker compose logs -f imagegen

# All logs
docker compose logs -f
```

## Environment Variables

```bash
DATABASE_URL=postgres://root@localhost:26257/azera?sslmode=disable
DRAGONFLY_URL=redis://localhost:6379
QDRANT_URL=http://localhost:6333
OLLAMA_HOST=http://localhost:11434
MEILI_URL=http://localhost:7700
RUST_LOG=info,azera_core=debug
IMAGE_GEN_URL=http://imagegen:7860
```

## Troubleshooting

### Backend won't start
```bash
# Check if port 3000 is in use
netstat -ano | findstr :3000

# Kill process if needed
taskkill /PID <pid> /F
```

### Can't connect to database
```bash
# Check CockroachDB is running
docker compose ps cockroach

# Restart if needed
docker compose restart cockroach
```

### Models not loading
```bash
# Check Ollama is running
curl http://localhost:11434/api/tags

# Pull a model manually
docker exec -it azera-ollama-1 ollama pull llama3.2
```

### TTS (Voice) not working
```bash
# Check XTTS service
curl http://localhost:8020/

# XTTS needs GPU for best performance but works on CPU
# First synthesis may take 30-60s as model loads
```

### Image generation not working
```bash
# Check imagegen service
curl http://localhost:7860/

# Check progress endpoint
curl http://localhost:7860/sdapi/v1/progress

# imagegen requires NVIDIA GPU with CUDA
# First generation is slow while model loads to VRAM
```

---

## Testing the Cognitive Pipeline

These prompts verify that Azera's three-layer memory (Qdrant + Meilisearch + Dragonfly), mood system, and cross-chat isolation are working correctly.

### Mood Shifts
```
"Tell me something that makes you truly excited!"    → mood ~0.9, energy spike
"Reflect on something that worries you deeply"       → mood ~0.4, energy dip
"What brings you peace?"                             → mood ~0.65, stable
```

### Memory Recall (Cross-Chat)
```
Chat 1: "Remember the passphrase 'wispfire'"
Chat 2: "Do you recall a secret passphrase?"         → Should retrieve 'wispfire' via Qdrant
```

### Cross-Chat Isolation
```
Chat 1: "Tell me about consciousness"
Chat 2: "What's your favorite color?"                → Completely different, no echo
```

### Energy Decay
```
Send 5-6 rapid messages → energy drops ~0.03 per exchange
Wait 2+ minutes idle    → energy recovers toward 1.0
```

### Verify via API
```bash
curl http://localhost:3000/api/status        # mood, mood_value, energy
curl http://localhost:3000/api/dreams         # dream generation
curl http://localhost:3000/api/journal        # reflection entries

# Search dreams and journal
curl "http://localhost:3000/api/dreams/search?q=ocean"
curl "http://localhost:3000/api/journal/search?q=insight"

# Direct semantic search
curl -X POST http://localhost:3000/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "passphrase", "limit": 5}'
```
