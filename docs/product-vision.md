# Kiri: The Open AI Layer

**Every device will have an AI layer. Yours should be open source, private, and locally-powered.**

Windows has Copilot. macOS has Apple Intelligence. Both are cloud-dependent, closed, and extract your data. Linux has nothing. Android's AI is Google's AI. Kiri fills the gap — a single open AI layer that runs on your laptop, your phone, your PC. Same personality, scaled to your hardware.

---

## The Superpowers

### 1. Voice as a First-Class Input — Everywhere

Not just dictation. Full system control.

- "Open Firefox and go to GitHub"
- "Move this window to workspace 2"
- "Turn brightness down"
- "Kill whatever is using 90% CPU"
- D-Bus integration means Kiri can talk to any Linux app natively

### 2. Kiri's Own Model — Purpose-Built, Not Borrowed

Kiri doesn't just run someone else's model. It trains and fine-tunes its own.

- **Purpose-built** — a model specifically trained for voice-driven system control, personal assistance, and on-device reasoning — not a general chatbot shoehorned into a desktop
- **Trained with Claude Code + Hugging Face** — Claude Code generates high-quality synthetic training data (system commands, voice interactions, multi-turn conversations). Hugging Face provides the training infrastructure, dataset hosting, and model hub
- **Distilled from the best** — start with a capable open base model (Qwen, Llama, Phi, Mistral), fine-tune with SFT/DPO on Kiri-specific tasks, then quantize aggressively (GGUF Q4/Q5) for consumer hardware
- **Runs on a consumer laptop** — 2-4B parameter sweet spot, 2-4GB RAM, runs on integrated GPU, NPU, or CPU-only. No discrete GPU required
- **Runs on a phone** — same model architecture scales down to mobile via llama.cpp on ARM. 1-2B variant for phones (transcription + simple Q&A), 3-4B for laptops (conversation + commands), 7B+ for desktops with GPU (full capability)
- **Continuous improvement** — Kiri's model gets better with each release. Community contributions, automated evals, and optional user-submitted feedback (explicit opt-in, never silent) drive the training loop
- **Task-specific LoRA adapters** — one base model, multiple lightweight adapters for different tasks: command parsing, conversational response, summarization, translation. Shared base weights mean they fit in memory together
- All training data, model weights, and training scripts are open source. You can retrain it yourself.

> **Ground truth:** Training a good model is expensive. Synthetic data generation via Claude API costs real money ($500-2000+ per training run). The core team maintains the training pipeline and publishes datasets. Community contributes eval data and fine-tunes. This is not a "just fork and retrain" situation — it's a funded, deliberate process.

### 3. Local LLM Brain — Private by Design

Run Kiri's own fine-tuned model or any llama.cpp / Ollama-compatible model locally:

- "Summarize this PDF"
- "Write a reply to this email — make it polite but firm"
- "Explain this error message"
- "Translate this to Khmer"
- All on-device. Your data never leaves your machine unless you explicitly choose to share it.

### 4. Second Brain — ~/kiri/ Becomes Your Knowledge Base

The private notes you're already building become a RAG-powered personal knowledge base:

- Semantic search over all your notes, documents, files
- "What did I think about that project last week?"
- "Find the article I saved about quantum computing"
- Auto-links related thoughts across days
- Local embeddings index, no cloud

### 5. Context Awareness — Kiri Knows What You're Doing

- In a terminal? Kiri helps with commands
- In a code editor? Kiri assists with code
- On a video call? Kiri transcribes and summarizes
- Reading a web page? "Summarize this" just works
- Wayland protocols + D-Bus give Kiri deep desktop awareness

### 6. Workflow Automation — Learns Your Patterns

- "Every morning, show me my calendar and unread messages"
- "When I plug in my external monitor, arrange my workspaces"
- Trigger chains: wake word -> voice command -> system action -> response
- Describe workflows in plain language, Kiri proposes the action chain, you confirm, it becomes a saved automation
- Not magic — Kiri suggests, you approve. Trust is earned through reliability, not assumed

### 7. Agents — Multi-Step Autonomous Tasks

- "Research flights to Phnom Penh next month and put the best options in my notes"
- "Monitor this log file and tell me if anything breaks"
- "Draft a blog post from my notes this week"
- Local agents that can use tools, browse, read files, think

### 8. Accessibility Superpower

- Full desktop control by voice for users who can't use keyboard/mouse
- Screen reading + understanding (vision model on screenshots)
- Real-time captioning of any audio
- This fills the biggest gap in Linux accessibility — and makes it competitive for the first time

### 9. Soul — Kiri Feels Alive

Not a transcription tool. A presence.

- **Streaming word-by-word** — text appears as you speak, not after you stop (Moonshine / streaming ASR)
- **Two-model pipeline** — tiny model for instant display, larger model for final accuracy
- **Neural voice activity detection** — knows speech from breathing, typing, background noise (Silero VAD)
- **Smart endpointing** — understands "I want to go to..." is incomplete even during a pause
- **Always-hot daemon** — model stays in memory via `kiri daemon`, popup connects via Unix socket, zero load time
- **Personality** — Kiri has a voice (Piper TTS), a tone, a way of responding that feels like a companion, not a command line
- The difference between a tool and an assistant is soul. Kiri has soul.

### 10. Memory — Kiri Remembers You

Not just notes. Kiri builds a relationship with you over time.

- **Conversational memory** — remembers what you said last session, last week, last month
- **User preferences** — learns how you like emails written, what tone you prefer, your workflow habits
- **Contextual recall** — "remember that idea I had about the garden?" just works
- **Temporal awareness** — "what was I working on yesterday?" draws from activity, notes, and context
- **Private journal** — ~/kiri/ becomes a living, searchable memory, not just flat markdown files
- **Forgetting** — "forget what I said about X" is respected immediately. You control your memory.
- Local embeddings, local vector store. Your memory never leaves your machine.

### 11. Cross-Platform — One Personality, Every Device

Not just a Linux desktop app. Kiri goes where you go.

- **Linux desktop** — the flagship. Full experience: GTK4, Wayland-native, D-Bus, system control, always-on daemon, wake word
- **Android phone** — the companion. Voice notes, transcription, quick questions, knowledge base access. On-demand activation (no always-on daemon — battery matters)
- **Android tablet** — expanded companion with touch-optimized UI
- **Linux phone** (PinePhone, Librem 5) — native citizen, same binary as desktop, battery-aware power modes
- **Shared memory** — your ~/kiri/ knowledge base syncs across devices (Syncthing, local network, or manual export). No cloud required
- **Adaptive UI** — same Rust core, platform-native shell. GTK4 on Linux desktop, Jetpack Compose on Android
- **Adaptive model** — automatically selects the right model size for your hardware. 1-2B on phone, 3-4B on laptop, 7B+ on desktop with GPU
- **Offline everywhere** — works on a plane, on a bus, in the forest

> **Ground truth:** Capabilities are not equal across devices. Android's sandbox prevents the deep system control that Linux allows. A 1-2B phone model can't match a 7B desktop model's reasoning. Kiri on desktop is the full experience. Kiri on mobile is a capable companion. Both share the same personality and your knowledge — not the same power.

#### Platform Capability Tiers

| Capability | Desktop (Linux) | Laptop (Linux) | Phone (Android) |
|---|---|---|---|
| Voice transcription | Full | Full | Full |
| Wake word / always-on | Yes | Yes (on power) | No (on-demand) |
| System control (D-Bus) | Full | Full | Not available |
| LLM conversation | 7B+ (full reasoning) | 3-4B (good) | 1-2B (basic Q&A) |
| Knowledge base (RAG) | Full | Full | Read + search |
| TTS responses | Yes | Yes | Yes |
| Vision model | Yes | Limited by RAM | Future |
| Agents (multi-step) | Yes | Yes | Simple only |
| Workflow automation | Full | Full | Trigger only |

### 12. Multi-Language, Multi-Modal

- Seamless switching between languages mid-sentence
- Khmer, English, any language whisper supports
- Vision: understand screenshots, photos, documents
- Future: gesture, gaze tracking

---

## Architecture

```
+------------------------------------------------------------------+
|                        Kiri Shell (per-platform)                  |
|  Linux: GTK4 + Wayland    |  Android: Jetpack Compose            |
|  Voice | Text | Vision | Gesture | Hotkey                        |
+-------------------------------+---------------------------------+
                                |
+-------------------------------v---------------------------------+
|                  Kiri Daemon (always running)                     |
|  Unix Socket API (Linux) | Bound Service (Android)               |
|  Hot models in memory | Adaptive model selection                 |
+-------------------------------+---------------------------------+
                                |
+-------------------------------v---------------------------------+
|                  Kiri Core (Rust — shared across all platforms)   |
|  +----------+ +----------+ +----------------+                    |
|  | Stream   | | Kiri     | |  Knowledge     |                    |
|  | ASR      | | Model    | |  Base (RAG)    |                    |
|  +----------+ +----------+ +----------------+                    |
|  +----------+ +----------+ +----------------+                    |
|  | TTS      | | Vision   | |  Agents        |                    |
|  | (Piper)  | | Model    | |  Framework     |                    |
|  +----------+ +----------+ +----------------+                    |
|  +----------+ +----------+ +----------------+                    |
|  | Neural   | | Memory   | |  User          |                    |
|  | VAD      | | Store    | |  Preferences   |                    |
|  +----------+ +----------+ +----------------+                    |
+-------------------------------+---------------------------------+
                                |
+-------------------------------v---------------------------------+
|                  Platform Integration Layer                       |
|  Linux: D-Bus | Wayland | PipeWire | Systemd | XDG              |
|  Android: Intents | Accessibility | MediaRecorder | NDK          |
+------------------------------------------------------------------+

=== Model Training Pipeline (separate codebase, Python, runs in CI/CD — not on user machines) ===

+------------------------------------------------------------------+
|                  Kiri Model Factory                               |
|  +------------------+  +------------------+  +--------------+    |
|  | Claude Code      |  | Hugging Face     |  | Eval Suite   |    |
|  | Synthetic Data   |  | Training Infra   |  | Benchmarks   |    |
|  | Generation       |  | (SFT, DPO, GRPO) |  | & Metrics    |    |
|  +--------+---------+  +--------+---------+  +------+-------+    |
|           |                      |                    |           |
|           v                      v                    v           |
|  +------------------+  +------------------+  +--------------+    |
|  | Training         |  | Quantization     |  | Model        |    |
|  | Datasets (HF)    |  | GGUF Q4/Q5/Q8   |  | Registry     |    |
|  |                  |  | Phone | Laptop   |  | (HF Hub)     |    |
|  +------------------+  +------------------+  +--------------+    |
+------------------------------------------------------------------+
```

---

## Roadmap

| Phase | Name | What |
|-------|------|------|
| **v0.1** | **Voice** (current) | STT, popup, wake word, private notes, Vulkan GPU |
| **v0.2** | **Soul** | Streaming ASR (Moonshine), neural VAD, two-model pipeline, daemon |
| **v0.3** | **Brain** | Local LLM integration (llama.cpp), conversational mode, eval framework for comparing models |
| **v0.4** | **Memory** | RAG over ~/kiri/, semantic search, local embeddings, conversational memory |
| **v0.5** | **Control** | D-Bus system commands, app control by voice, safety confirmations for destructive actions |
| **v0.6** | **Own Model v1** | Synthetic data pipeline (Claude Code), first SFT fine-tune on HF, GGUF quantization, benchmarks vs generic models |
| **v0.7** | **Speak** | TTS responses (Piper), conversational voice loop, personality |
| **v0.8** | **See** | Vision model, screenshot understanding |
| **v0.9** | **Agents** | Multi-step task execution, tool use |
| **v0.10** | **Own Model v2** | DPO/GRPO refinement, LoRA task adapters, phone-size (1-2B) variant, community eval contributions |
| **v0.11** | **Mobile** | Android port — Rust core via NDK, Jetpack Compose shell, battery-aware power modes, companion feature set |
| **v1.0** | **Kiri** | The complete AI layer — Linux desktop full experience, Android companion, model maturity |

> **Ground truth on ordering:** Brain (v0.3) and Memory (v0.4) come before Own Model (v0.6) deliberately. A good generic model with good RAG beats a mediocre fine-tune without RAG. Get the infrastructure right first, then improve the model. Own Model v1 is about proving the pipeline works. Own Model v2 is about making the model genuinely better than off-the-shelf. Mobile comes last because Android's NDK + Rust toolchain needs the core to be stable first.

---

## The Killer Differentiators

### Privacy is not a limitation — it's the product.

Every competitor sends your voice, your thoughts, your screen to a cloud. Kiri runs 100% locally on your hardware. Your NPU, your GPU, your CPU. Your data stays yours. Period.

This isn't just ideology — it's a technical advantage:

- **No latency** to a server
- **Works offline** — on a plane, in the field, anywhere
- **Works everywhere** — even in countries with censored internet
- **Works for everyone** — journalists, activists, businesses with sensitive data

### A model built for this, not borrowed for this.

Every other local AI assistant grabs a generic model off the shelf and hopes for the best. Kiri trains its own model — purpose-built for voice interaction, system control, and personal assistance on consumer hardware.

- **Better at its job** — a 3B model fine-tuned for Kiri's tasks outperforms a generic 7B model trying to do everything
- **Smaller and faster** — purpose-built means less waste. Runs on a phone where generic models can't
- **Open training pipeline** — every dataset, every training run, every eval is public. Fork it, retrain it, improve it
- **Cloud for training, local for inference** — Hugging Face GPU clusters train the model. Your device runs it. Best of both worlds

### One AI, every device.

Copilot is Windows-only. Apple Intelligence is Apple-only. Kiri runs on any Linux desktop, any Android phone, any ARM or x86 device. Same personality, same knowledge — scaled to wherever you are.

### A public good, not a product.

Kiri is not a startup. It's not a platform play. It's infrastructure — like Linux itself.

Created by KOOMPI. Owned by no one. Used by everyone. The code, the models, the training data, the pipeline — all open. No company controls it. No company can kill it. No company can enshittify it.

If KOOMPI disappears tomorrow, Kiri lives on. That's the point.

---

## What Kiri Is Not

Honesty about scope is a feature, not a weakness.

- **Not a cloud AI wrapper** — no OpenAI API key, no fallback to a server. If it can't run locally, it doesn't ship
- **Not equal on every device** — a phone gets a companion, a desktop gets the full experience. See Platform Capability Tiers
- **Not a single binary in the literal sense** — the Rust binary is the engine, but it needs model files (1-4GB) to function. "Single install" is the real promise, not "single file"
- **Not free to develop** — model training costs real money. Open source doesn't mean zero cost. The training pipeline is funded and deliberate
- **Not magic** — destructive system commands require confirmation. Workflow automation requires approval. Natural language is the interface, not the safety mechanism
- **Not a replacement for accessibility frameworks** — Kiri adds voice control to Linux, filling a major gap. It doesn't replace decades of accessibility infrastructure in other OSes overnight

---

## Principles

1. **Open, always** — created by KOOMPI, owned by no one, used by everyone. Code, models, datasets, training scripts — all open. Like Linux itself
2. **Local first** — everything runs on the user's machine. Your data never leaves unless you explicitly choose to share it
3. **Rust native** — single binary, no runtimes, no Python deps at runtime. Training pipeline is separate (Python, CI/CD). Cross-compiles to ARM and x86
4. **Cross-platform** — Linux desktop (flagship, full experience) and Android (companion) from the same Rust core
5. **Consumer hardware** — runs on a laptop with integrated graphics. Runs on a phone. No discrete GPU required
6. **Purpose-built model** — Kiri trains its own model for its own job. A focused 3B beats a generic 7B at Kiri's tasks
7. **Honest about limits** — capabilities scale with hardware. Not every feature works everywhere. The docs say so
8. **Composable** — Unix philosophy, each piece works standalone
9. **Accessible** — voice-first means everyone can use it
10. **Multilingual** — not English-first, truly global
11. **Respectful** — no telemetry, no tracking, no dark patterns. No corporate owner to change this later

---

## Dependency Risks & Mitigations

The sky is the limit, but we live on the ground. These are the things that could go wrong.

| Dependency | Risk | Mitigation |
|---|---|---|
| **llama.cpp** | Rapid API changes, single maintainer ecosystem | Pin versions, maintain fork if needed, abstract model loading |
| **whisper.cpp** | Same single-maintainer risk | whisper-rs wraps it; can swap to alternative ASR if needed |
| **Hugging Face** | Training infra pricing/access could change | Export scripts to run on any GPU cluster; pre-generate and version datasets |
| **Claude API** | Synthetic data gen depends on Anthropic pricing | Pre-generate datasets per release; don't require live API for builds |
| **GTK4-layer-shell** | Niche crate, already pins gtk4 to 0.10 | Monitor upstream; evaluate alternative overlay approaches |
| **GGUF format** | Could be superseded | Abstract model loading; GGUF is the dominant format for now |
| **Android NDK + Rust** | Toolchain maturity for complex apps | Start Android experiments early (v0.8+); don't assume late-stage port works |
| **KOOMPI funding** | Core team needs resources for training pipeline | Design for community handoff from day one; no single point of failure |
