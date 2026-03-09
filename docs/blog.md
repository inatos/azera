# Building Azera: An AI That Remembers, Dreams, and Reflects

Most chat applications treat conversations as isolated events. You type, the AI responds, and everything disappears into the void. I wanted to build something different — an AI that actually *remembers*, that forms emotional context across conversations, and that does something interesting when you're not talking to it.

Azera is the result: an emotionally intelligent chat application with a three-layer cognitive architecture, autonomous mental states, AI image generation, voice synthesis, and a persona system that lets you define entirely different AI characters through markdown.

This post walks through why I built it, how the architecture works, and the interesting technical problems I ran into along the way.

---

## The Problem with Stateless AI

Every major chat interface today has the same fundamental limitation: context resets. You can have a deeply personal conversation with an AI, close the tab, and it's gone. Open a new chat and you're talking to a stranger again.

This isn't just annoying — it's architecturally lazy. We have vector databases, semantic search, and embedding models. There's no reason an AI can't maintain genuine continuity across conversations, recall what you discussed last week, and build an evolving understanding of the relationship over time.

That was the starting question: *what would it look like if an AI actually remembered everything?*

The answer turned out to involve a lot more than a vector database.

## Three Layers of Memory

The core insight was that human memory isn't one system — it's at least three. You have long-term associations (semantic memory), the ability to search for specific facts (lexical memory), and a short-term attention buffer (working memory). Azera mirrors this with three database services:

- **Semantic Memory (Qdrant)** stores vector embeddings of every conversation. When you send a message, the system generates an embedding and searches for the top 10 most similar memories across all past chats. This is how Azera can recall a passphrase you mentioned three days ago in a completely different conversation — the *meaning* matches even if the words don't.

- **Lexical Memory (Meilisearch)** provides structured, word-based retrieval across two indexes: `chats` and `memories` (which includes dreams, journal entries, and facts). This catches things that semantic search misses — proper nouns, specific dates, exact phrases. When you ask "what did you dream about recently?", it's Meilisearch that pulls the dream entries.

- **Working Memory (DragonflyDB)** is the attention buffer. It stores session context (24-hour TTL), an embedding cache (7-day TTL), and the current mental state. This is how Azera knows what you were *just* talking about without needing to hit the heavier databases.

Every message triggers a hybrid RAG pipeline that queries all three layers, deduplicates the results, and injects the combined context into the LLM prompt. The pipeline applies quality filters — dropping anything below 0.45 similarity, skipping memories less than 60 seconds old (to avoid echo), and truncating snippets to 400 characters.

### Why Not Just Use One Database?

I tried. A single vector database gives you great semantic recall but terrible exact-match search. A single search engine gives you great keyword matching but no understanding of meaning. And neither handles the "what were we just talking about?" case without a fast session cache.

The three-layer approach means each service does what it's best at:

```rust
// 1. Qdrant — what's semantically relevant?
let semantic_results = search_memories_with_filter_cached(
    vector_service: &vector_service,
    ollama_host:    &ollama_host,
    cache:          &cache,
    collection:     "azera_memory",
    query:          &message,
    limit:          10,
    filter:         Some(filter)
).await?;

// 2. Meilisearch — what matches the words?
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

// 3. Merge, dedup, inject as context
let mut seen_content = HashSet::new();
for r in &semantic_results {
    if r.score < 0.45 { continue; }
    // dedup by first 100 chars, truncate to 400
}
```

The deduplication step is important because both systems will often return overlapping results. Without it, the LLM gets the same memory repeated three times and starts hallucinating that something is more important than it actually is.

## The Tick Loop: Autonomous Behavior

The most novel part of Azera's architecture is that it does things when you're *not* talking to it. The backend runs a 1 Hz tick loop that continuously processes the agent's mental state:

1. **Perception** — Syncs DragonflyDB state into the agent, applies idle drift (energy slowly recovers, mood drifts toward neutral, focus decays)
2. **Emotional Processing** — Updates mood and energy based on recent interactions
3. **Dream Processing** — When energy drops low enough, the dreaming system kicks in and generates creative consolidations of recent conversations
4. **Reflection Processing** — At high clarity, the system writes journal entries with genuine insights about past interactions

Dreams and journal entries are dual-written to both Qdrant and Meilisearch, so they become part of the memory system. This means Azera can reference its own dreams in conversation — "I dreamt about our conversation on consciousness last night" — and it's not a parlor trick. The dream actually exists in the memory store and was generated from real conversation embeddings.

## Mood Is Not a Gimmick

Every AI response goes through mood inference: a lightweight LLM call that classifies the emotional tone of the response into one of eight moods (happy, excited, content, calm, curious, thoughtful, melancholy, concerned). This mood value propagates through the entire system:

```rust
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

let _ = tx.send(StreamEvent::Done {
    message_id: assistant_msg_id,
    mood:       mood,
    mood_value: done_mood_value,
    energy:     done_energy,
}).await;
```

The mood value is a float (0.0 to 1.0) stored in Dragonfly, synced to the frontend via the SSE `Done` event, and rendered as a live mood bar in the UI. Energy decays by ~0.03 per exchange and slowly recovers during idle time. When energy drops low enough, it triggers dreaming.

This creates emergent behavior that I didn't explicitly program. After a long intense conversation, Azera's energy is low and its mood reflects the tone of the discussion. Leave it alone for a few minutes and it might dream. Come back and it's refreshed but carrying forward the emotional context. It *feels* like continuity, because it is.

## Cross-Chat Isolation (The Hard Part)

Memory is great until it leaks. If you're having a private conversation in one chat and ask an unrelated question in another, you don't want the AI accidentally regurgitating details from the first.

The RAG pipeline enforces isolation at multiple levels:
- Every Qdrant query includes a `must_not` filter excluding the current `chat_id`
- Meilisearch queries filter by `ai_persona_id` to prevent cross-persona leakage
- A 60-second recency filter prevents the system from immediately retrieving what you just said (which would create bizarre echo effects)
- Results below 0.45 similarity are dropped entirely

The persona isolation was particularly tricky. Azera (the professional coder) and Areza (the dungeon master) share the same infrastructure but maintain completely separate memory pools. A conversation with Areza about goblin politics never surfaces when you're asking Azera about Rust lifetimes.

## The Persona System

Instead of hardcoding personalities, Azera uses markdown files as persona definitions. Each persona is essentially a structured prompt that the system feeds to the LLM — but broken into meaningful sections that shape different dimensions of behavior:

- **Intent** — The one-line purpose statement
- **Core Identity** — Who the AI believes it is
- **Prime Directive** — The relationship dynamic (how it perceives the user, tone, constraints)
- **Psychological Profile** — Archetype, MBTI, cognitive style, emotional landscape
- **Task Behaviors** — Context-dependent overrides (how it acts when coding vs storytelling vs troubleshooting)

The two built-in personas demonstrate the range. Azera is an ISTJ Logistician — analytical, sequential, even-keeled. It uses bullet points, code blocks, and the BLUF method. Areza is an ENTP Debater — improvisational, theatrical, dramatic. It uses *italics* for sensory details and **bold** for game mechanics.

There are also *user* personas. The default is Protag, but you can create others to adopt different roles in different contexts. Every message carries both a `user_persona_id` and an `ai_persona_id`, so the system always knows who's talking to whom. This enables proper roleplay scenarios — you can be a different character in Areza's dungeon while being yourself in Azera's dev sessions.

## Embedding Cache: Making RAG Fast

A naive RAG implementation hammers the embedding model on every message. For a 7-service architecture where every chat request triggers two separate embedding calls (one for the query, one for storage), this adds up fast.

The embedding cache uses DragonflyDB with a simple scheme: SHA256 the input text, truncate to 16 hex chars, store the base64-encoded f32 vector with a 7-day TTL:

```rust
fn embedding_key(text: &str) -> String {
    let hash = hex::encode(Sha256::new().chain_update(text).finalize());
    format!("emb:{}", &hash[..16])
}
```

Cache hits skip the Ollama round-trip entirely. Cache writes are fire-and-forget (spawned as async tasks) so they don't block the response. In practice, this eliminates most embedding computation after the first few hours of use, since recurring phrases and topics generate the same hashes.

## The Frontend: Svelte 5 Runes in Production

The frontend uses Svelte 5 with the runes API — `$state`, `$derived`, `$effect`. The entire application state lives in a single `AppState` class:

```typescript
export class AppState {
    chats = $state<Chat[]>([]);
    aiPersonas = $state<Persona[]>([]);
    mood = $state(0.5);
    energy = $state(0.7);
    showThinking = $state(true);
    sendOnEnter = $state(false);
    currentChat = $derived(this.chats.find(c => c.id === this.currentChatId) || null);
}
```

The mood and energy values update in real-time via the SSE `Done` event, driving live animated bars in the profile viewer. The streaming chat uses Server-Sent Events with typed event discrimination — `thinking_start`, `thinking`, `thinking_end`, `content`, `done`, `error` — so the UI can render AI reasoning blocks separately from the actual response.

## Image Generation

Azera includes a dedicated image generation pipeline powered by Animagine XL 3.1 (via HuggingFace Diffusers). It runs as a separate Python/CUDA sidecar with a custom FastAPI server. The interesting part is the real-time progress tracking — the backend streams SSE events for each diffusion step, so the UI shows a live progress bar and step count during generation.

There's also a Canvas page — a dedicated workspace for image generation that's separate from the chat interface. Generated images are stored on disk and served via the API, with a gallery view for browsing and managing them.

The LLM can even trigger image generation from within a chat conversation by emitting a special `[IMAGE_GEN: prompt="...", name="..."]` tag in its response. This gets parsed server-side and fires off an async generation task.

## 3D Model Generation

Building on the image generation pipeline, Azera supports image-to-3D generation using Tencent's Hunyuan3D 2.1. It follows the same sidecar architecture — a separate Python/CUDA container running a FastAPI server — but the pipeline is substantially more complex: shape generation via a 3.3B-parameter DiT flow-matching model, followed by PBR texture painting across multiple views.

Hunyuan3D is image-conditioned only — there's no native text-to-3D support. When a user provides only a text prompt, the gen3d server first calls the imagegen service to generate a reference image, then feeds that into the 3D pipeline. The UI reflects this by requiring a reference image upload for Generate-3D.

The progress tracking works the same way as image generation: SSE events stream through the Rust backend, so the UI shows real-time progress for both the shape and texture stages. Output files (GLB) are stored with JSON metadata sidecars and served through the API.

The Canvas page has four tabs — Generate-2D, Gallery-2D, Generate-3D, and Gallery-3D — with tab persistence via URL hash so refreshing preserves your position. The 3D gallery uses Google's `<model-viewer>` web component for interactive in-browser preview with auto-rotate, camera controls, and shadow rendering. The gallery filters to GLB/glTF formats only, since model-viewer can't handle raw OBJ files.

## Fitting a 3.3B Model in 16 GB VRAM

The most interesting engineering challenge was making Hunyuan3D 2.1 run on a laptop GPU. The model nominally requires 24+ GB of system RAM and 10+ GB of VRAM — my RTX 3080 Ti Laptop has 16 GB VRAM and shares 32 GB system RAM with WSL2 (capped at 24 GB via `.wslconfig`). And the 3D pipeline needs to coexist with image generation, voice synthesis, and seven other Docker containers.

### The Memory Problem

Naive loading of the Hunyuan3D checkpoint works like this:
1. `torch.load("checkpoint.ckpt")` reads ~7 GB into RAM
2. `model.load_state_dict(ckpt['model'])` copies 6.5 GB (DiT) — now 13.5 GB in RAM
3. `vae.load_state_dict(ckpt['vae'])` copies 1.5 GB — now 15 GB in RAM
4. Loading all three components to GPU simultaneously needs ~10 GB VRAM

On a system where WSL2 is already using 12+ GB for other services, this either OOM-kills the container or the entire VM.

### Three Layers of Memory Optimization

**Layer 1: Memory-mapped loading.** I monkey-patched `torch.load` to inject `mmap=True`, which lazy-loads the checkpoint — the OS pages data in on demand instead of reading the entire 7 GB into RAM. Then I created `_StagedDict`, a dict wrapper that auto-frees the previous sub-dict when a new key is accessed. When PyTorch loads the 'vae' weights, the 6.5 GB 'model' weights are automatically freed first. Finally, `assign=True` on `load_state_dict` replaces model parameters with the mmap'd tensors directly instead of copying. Combined peak: ~6.5 GB instead of ~16 GB.

```python
class _StagedDict(dict):
    """Dict wrapper that frees the previous value when a different key is read."""
    def __init__(self, src: dict):
        super().__init__(src)
        self._prev: list[str] = []

    def __getitem__(self, key):
        for old in self._prev:
            if old != key and old in self:
                dict.__delitem__(self, old)  # Free 6.5 GB before loading 1.5 GB
        gc.collect()
        self._prev = [key]
        return dict.__getitem__(self, key)
```

**Layer 2: Sequential CPU↔GPU offloading.** Inspired by ComfyUI's approach, instead of loading all components to GPU simultaneously, the pipeline runs in two phases:
- Phase 1: DiT + conditioner → GPU, run diffusion, output raw latents (~7 GB VRAM)
- Phase 2: DiT + conditioner → CPU, VAE → GPU, decode latents → mesh (~1.5 GB VRAM)

Peak VRAM = max(DiT, VAE) ≈ 7 GB instead of sum ≈ 10 GB.

The phase transition originally took ~29 seconds (sequential CPU↔GPU transfers). I parallelized it with `threading.Thread` — offloading DiT→CPU in a background thread while simultaneously loading VAE→GPU on the main thread. Both use separate DMA channels so they genuinely overlap. This brought the transition down to ~19 seconds:

```python
def _offload_dit_conditioner():
    pipe.model.to("cpu")
    pipe.conditioner.to("cpu")

offload_thread = threading.Thread(target=_offload_dit_conditioner)
offload_thread.start()
pipe.vae.to(device)  # Overlaps with DiT→CPU
offload_thread.join()
```

**Layer 3: Volume-backed on-demand loading.** Models sit on a Docker volume (~34 GB). With `mmap=True` + `assign=True`, reloading pages them in from disk (~30s) instead of holding ~9 GB in RAM permanently. After each generation, the shape pipeline is fully unloaded (`_shape_pipe = None` + `gc.collect()`). Between requests, gen3d holds ~0 GB instead of ~9 GB. The OS file cache keeps hot pages warm between runs.

### Texture Pipeline

The default Hunyuan3D texture settings (render 2048px, texture 4096px, 6 views) were both too slow (15+ minutes) and too VRAM-hungry. Reducing to render 1024px, texture 2048px, and 4 views still produces clean PBR materials while fitting in VRAM. The texture pipeline is lazy-loaded *after* shape generation finishes (so the DiT is safely on CPU), and released after each use to prevent OOM on subsequent generations.

### Compilation Caching

`torch.compile` with the inductor backend wraps the DiT and conditioner for Triton kernel fusion, but the first compilation takes 2-5 minutes. The compiled kernels are disk-cached on the model volume (`TORCHINDUCTOR_CACHE_DIR=/models/torch_cache`), so subsequent container restarts are fast. The VAE is excluded from compilation — its custom CUDA marching-cubes extensions create hard graph breaks, and single-shot execution gives minimal benefit.

### Recovery

Despite all optimizations, texture generation occasionally pushes the system past its limits. The gen3d container runs with `restart: unless-stopped` and `shm_size: 4g` so it automatically recovers from OOM kills without manual intervention.

## 13 Services, One `docker compose up`

The full system runs as 13 Docker containers:

| Service | Purpose |
|---------|----------|
| azera-core | Rust/Axum backend |
| azera-web | SvelteKit frontend |
| CockroachDB | Persistent SQL storage |
| DragonflyDB | Working memory and embedding cache |
| Qdrant | Semantic vector memory |
| Meilisearch | Lexical search |
| Ollama | LLM inference |
| ollama-init | Model management on startup |
| XTTS | Text-to-speech synthesis |
| ImageGen | AI image generation (CUDA) |
| Gen3D | Image-to-3D generation (CUDA, low-VRAM optimized) |
| Jenkins | CI/CD pipeline |
| docker-gc | Automated disk cleanup (daily prune) |

Despite the complexity, getting started is just `docker compose up -d`. The ollama-init sidecar reads a model ledger and pulls any missing models on startup, so the system is self-bootstrapping.

## What I Learned

**Polyglot persistence is worth the complexity.** Using four different databases (CockroachDB, DragonflyDB, Qdrant, Meilisearch) sounds like over-engineering, but each one is genuinely the best tool for its job. The alternative — cramming everything into Postgres with pgvector — would have been simpler to deploy, but worse at every individual task.

**Mood inference is cheap and surprisingly effective.** A one-shot classification call with `temperature: 0.1` and `num_predict: 10` adds maybe 200ms but creates a persistent emotional thread that makes the AI feel alive. The key is propagating it through the full system — cache, agent state, frontend, and back into future context.

**Cross-chat isolation is a correctness problem, not a feature.** I initially treated memory isolation as a nice-to-have. Then I watched one persona's conversation details leak into another and realized it's a hard requirement. Every query path needs explicit persona and chat filters or the system becomes untrustworthy.

**The persona system exceeded my expectations.** I thought of it as a simple system prompt swap, but the structured markdown format — with separate sections for psychology, task behaviors, quirks, and relationship dynamics — produces meaningfully different AI personalities. Azera and Areza don't just talk differently; they *think* differently about the same problems.

**VRAM is a budget, not a limit.** Running a 3.3B-parameter 3D generation model on a 16 GB laptop GPU alongside image generation, voice synthesis, and seven other services seemed impossible at first. The trick is treating VRAM like a time-shared resource: sequential CPU↔GPU offloading, pipeline parallelism for phase transitions, mmap-backed loading to eliminate RAM peaks, and aggressive unloading between requests. The model doesn't need to live in memory — it just needs to be there when you need it. Volume-backed mmap loading with OS page cache handles the rest.

**Docker disk is a slow leak.** Dangling images, orphaned build cache, and multi-stage build layers accumulate silently until you're 100+ GB deep. The WSL2 VHDX virtual disk *never* auto-shrinks. An automated docker-gc sidecar that prunes daily is cheap insurance — and you'll still need to compact the VHDX periodically.

---

Azera is open source on [GitHub](https://github.com/inatos/azera). Clone it, spin up the containers, and start chatting. Or write a persona and see what character emerges.
