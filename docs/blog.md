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
    &vector_service, &ollama_host, &cache, "azera_memory", &message, 10, Some(filter),
).await?;

// 2. Meilisearch — what matches the words?
let lexical_results = meili_search_memories(&meili_url, &meili_key, &message, None, ai_persona_id, 10).await;
let lexical_chats = meili_search_chats_for_rag(&meili_url, &meili_key, &message, ai_persona_id, 5).await;

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
let mood = llm.infer_mood(&model, &full_response).await?;
let _ = CacheService::update_mood(&cache, mood_value, &mood, -0.03).await;

let _ = tx.send(StreamEvent::Done {
    message_id, mood: Some(mood),
    mood_value: done_mood_value, energy: done_energy,
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

## 11 Services, One `docker compose up`

The full system runs as 11 Docker containers:

| Service | Purpose |
|---------|---------|
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
| Jenkins | CI/CD pipeline |

Despite the complexity, getting started is just `docker compose up -d`. The ollama-init sidecar reads a model ledger and pulls any missing models on startup, so the system is self-bootstrapping.

## What I Learned

**Polyglot persistence is worth the complexity.** Using four different databases (CockroachDB, DragonflyDB, Qdrant, Meilisearch) sounds like over-engineering, but each one is genuinely the best tool for its job. The alternative — cramming everything into Postgres with pgvector — would have been simpler to deploy, but worse at every individual task.

**Mood inference is cheap and surprisingly effective.** A one-shot classification call with `temperature: 0.1` and `num_predict: 10` adds maybe 200ms but creates a persistent emotional thread that makes the AI feel alive. The key is propagating it through the full system — cache, agent state, frontend, and back into future context.

**Cross-chat isolation is a correctness problem, not a feature.** I initially treated memory isolation as a nice-to-have. Then I watched one persona's conversation details leak into another and realized it's a hard requirement. Every query path needs explicit persona and chat filters or the system becomes untrustworthy.

**The persona system exceeded my expectations.** I thought of it as a simple system prompt swap, but the structured markdown format — with separate sections for psychology, task behaviors, quirks, and relationship dynamics — produces meaningfully different AI personalities. Azera and Areza don't just talk differently; they *think* differently about the same problems.

---

Azera is open source on [GitHub](https://github.com/inatos/azera). Clone it, spin up the containers, and start chatting. Or write a persona and see what character emerges.
