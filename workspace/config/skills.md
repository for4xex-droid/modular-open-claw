# The Capability Matrix (skills.md)

This document serves as the "Skills" artifact in the Samsara Protocol.
Aiome has the following internal tools and execution mechanisms available.
When defining the topic and style of the next video, leverage these capabilities.

## 1. Video Generator (Comfy Bridge)
- **Engine**: ComfyUI (Local SDXL/Pony models via WS)
- **Role**: Visual content generation.
- **Workflows (Styles) Available**:
  - `tech_news_v1`: Generates anime-style news presentation visuals (perfect for internet trends, tech news, VTuber-like content).
  - `cyber_drama`: Dark, cyberpunk, intense aesthetics (best for mysteries, dramatic tech failures, hacking topics).
  - `zen_philosophy`: Clean, minimalistic aesthetic (suitable for motivational quotes, calm explanations, abstract concepts).
- **Constraints**: Prompts should emphasize visual tags (e.g., "masterpiece, 1girl, newscaster, futuristic studio").

## 2. Voice Narration (VoiceActor)
- **Engine**: Style-Bert-VITS2
- **Role**: Dynamic voice generation (Japanese).
- **Available Voices**:
  - `jvnv-F1-jp`: Crisp, clear female announcer (default for news).
  - `tsukuyomi_chan`: Emotional, anime-like female voice.
- **Constraints**: Scripts are parsed into arrays of lines. Avoid extremely long sentences without punctuation.

## 3. Post-Production (MediaForge)
- **Engine**: FFmpeg 
- **Role**: Trimming, 9:16 vertical resizing, and BGM mixing.
- **Effect Options**:
  - `zoom_pan`: Subtle Ken Burns effect on static images to create illusion of video.
  - `audio_ducking`: Automatically lowers background music volume when characters speak.

---

**Instruction for Synthesis**:
Observe the above skills. Consider any past Karma. Then select the best *workflow/style* matching the topic you wish to explore today.
