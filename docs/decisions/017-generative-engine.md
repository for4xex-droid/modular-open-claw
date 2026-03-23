# ADR-017: Generative Engine (Non-Text Engine)

## Context
Current AI agents primarily generate text (`LlmProvider`). Expanding to image, audio, and video generation requires an engine that can handle non-text generative assets, prompt-to-media conversion, and media post-processing in a scalable way.

## Decision
1.  **Orchestrated Multimedia Generation**: Rather than monolithic media generation, we will build a generative orchestrator in `libs/infrastructure/src/generative/`.
2.  **Provider Trait System**: Define a `GenerativeProvider` trait with specific capabilities (Image, Audio, Video, 3D).
3.  **Default Stacks**:
    - **Image**: Re-generate via Stable Diffusion (Self-hosted via Docker or API) or Fal.ai for maximum quality.
    - **Audio**: ElevenLabs or OpenAI TTS for voice synthesis.
    - **Video**: Kling, Runway, or Luma-AI for narrative-to-video generation.
4.  **Batch/Async Execution**: All media generation tasks MUST be queued via the `JobQueue` to avoid blocking main execution and to handle long-running generative processes (e.g., video rendering).
5.  **Artifact Workflow**: Generated media will be instantly stored in the `ArtifactStore`, categorized by the agent's UUID, and subjected to the `QuarantineStore` (ADR-015 compliant checks).

## Status
Proposed (Phase 27 Target)

## Consequences
- **Capability Expansion**: Enables multimodal AI agents (VTubers, voice assistants, creative creators).
- **Cost Management**: Media generation is expensive; require a priority/karma-based throttling mechanism inside the engine.
- **Resource Intensity**: High VRAM demand if self-hosted; requires robust resource allocation or fallback to external APIs.
- **Latency**: Media generation takes time; frontend must support asynchronous status updates and real-time generation previews.
