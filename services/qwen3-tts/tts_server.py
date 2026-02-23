"""
Qwen3-TTS FastAPI Server — OpenAI API互換ラッパー
================================================
POST /v1/audio/speech  →  44.1kHz WAV バイナリを返却

セキュリティ:
- voice パラメータの LFI 防止（英数字+ハイフン+アンダースコアのみ）
- PYTORCH_ENABLE_MPS_FALLBACK=1 によるMPS非互換op自動CPU fallback

パフォーマンス:
- asyncio.to_thread でPyTorch推論をスレッドプールへオフロード
- モデルは起動時に1回だけロード (グローバルシングルトン)
"""

import os
import io
import re
import asyncio
import logging

import torch
import torchaudio
import soundfile as sf
import numpy as np
from fastapi import FastAPI, HTTPException
from fastapi.responses import Response
from pydantic import BaseModel
from qwen_tts import Qwen3TTSModel

# --- Configuration ---
MODEL_ID = os.environ.get("QWEN_TTS_MODEL", "Qwen/Qwen3-TTS-12Hz-1.7B-Base")
VOICES_DIR = os.path.abspath(os.environ.get("VOICES_DIR", "../../resources/voices"))
HOST = os.environ.get("TTS_HOST", "0.0.0.0")
PORT = int(os.environ.get("TTS_PORT", "5001"))
OUTPUT_SAMPLE_RATE = 44100

# --- Logging ---
logging.basicConfig(level=logging.INFO, format="%(asctime)s | %(levelname)s | %(message)s")
logger = logging.getLogger("qwen3-tts-server")

# --- Safety: voice name validation ---
VOICE_NAME_RE = re.compile(r"^[a-zA-Z0-9_-]+$")

# --- App ---
app = FastAPI(title="Qwen3-TTS Server", version="1.0.0")

# --- Global model singleton ---
model = None


def load_model():
    """モデルを1回だけロードする (コールドスタート)"""
    global model
    if model is not None:
        return

    logger.info(f"Loading Qwen3-TTS model: {MODEL_ID}")

    # MPS (Apple Silicon) detection
    if torch.backends.mps.is_available():
        device = "mps"
        dtype = torch.float32  # MPS は bfloat16 未サポートの場合がある
        attn_impl = "eager"    # FlashAttention は MPS 非互換
        logger.info("Using MPS device with eager attention and float32")
    elif torch.cuda.is_available():
        device = "cuda:0"
        dtype = torch.bfloat16
        attn_impl = "flash_attention_2"
        logger.info("Using CUDA device with flash_attention_2 and bfloat16")
    else:
        device = "cpu"
        dtype = torch.float32
        attn_impl = "eager"
        logger.info("Using CPU device with eager attention and float32")

    model = Qwen3TTSModel.from_pretrained(
        MODEL_ID,
        device_map=device,
        dtype=dtype,
        attn_implementation=attn_impl,
    )
    logger.info("Model loaded successfully")


class SpeechRequest(BaseModel):
    """OpenAI API 互換リクエスト"""
    input: str
    voice: str = "aiome_narrator"
    response_format: str = "wav"
    # ボイスクローン用の参照テキスト（任意）
    ref_text: str | None = None
    # TTS品質パラメータ（任意: リクエスト側からオーバーライド可能）
    temperature: float | None = None
    repetition_penalty: float | None = None
    speed: float | None = None


def resolve_voice_path(voice_name: str) -> str:
    """
    voice パラメータからWAVファイルパスを安全に解決する。
    LFI (パストラバーサル) を物理的に防止。
    """
    if not VOICE_NAME_RE.match(voice_name):
        raise HTTPException(
            status_code=400,
            detail=f"Invalid voice name: '{voice_name}'. Only alphanumeric, hyphens, and underscores allowed."
        )

    wav_path = os.path.join(VOICES_DIR, f"{voice_name}.wav")

    # 二重チェック: resolved path が VOICES_DIR 内であることを確認
    real_path = os.path.realpath(wav_path)
    real_voices_dir = os.path.realpath(VOICES_DIR)
    if not real_path.startswith(real_voices_dir):
        raise HTTPException(status_code=400, detail="Path traversal detected.")

    if not os.path.isfile(wav_path):
        raise HTTPException(
            status_code=404,
            detail=f"Voice file not found: '{voice_name}'. Place a WAV file at: {wav_path}"
        )

    return wav_path


# --- デフォルト TTS 品質パラメータ ---
DEFAULT_SPEED = 1.0
DEFAULT_TEMPERATURE = 0.7
DEFAULT_REPETITION_PENALTY = 1.2

# --- リファレンステキスト (ICLモード用) ---
# aiome_narrator.wav の内容を書き起こしたテキスト。
# これを設定すると x_vector_only_mode=False (ICLモード) になり、
# 声質だけでなく話し方の「調子」まで模倣できるようになる。
# 空文字列なら x_vector_only_mode=True (声質のみ) のままになる。
DEFAULT_REF_TEXT = os.environ.get(
    "TTS_REF_TEXT",
    "そんなんだから友達できないんじゃない。でもまさか本当にお兄ちゃんが別の家の子供だったなんてねぇ。どうりでおかしいと思ったよ。誰にも似てないし、一人だけ勉強できるし、陰キャだし。"
)


def synthesize(text: str, voice_name: str, ref_text: str | None,
               temperature: float | None = None,
               repetition_penalty: float | None = None,
               speed: float | None = None) -> bytes:
    """
    同期的なTTS推論 (スレッドプールから呼ばれる)
    44.1kHz WAV バイナリを返す
    """
    ref_audio_path = resolve_voice_path(voice_name)

    # パラメータのフォールバック
    actual_speed = speed if speed is not None else DEFAULT_SPEED
    actual_temp = temperature if temperature is not None else DEFAULT_TEMPERATURE
    actual_rep_penalty = repetition_penalty if repetition_penalty is not None else DEFAULT_REPETITION_PENALTY

    # ref_text 解決: リクエスト > デフォルト > 空
    actual_ref_text = ref_text if ref_text is not None else DEFAULT_REF_TEXT
    use_xvector_only = actual_ref_text.strip() == ""

    logger.info(
        f"Synthesizing: text='{text[:50]}...', voice={voice_name}, "
        f"xvector_only={use_xvector_only}, speed={actual_speed}, "
        f"temp={actual_temp}, rep_penalty={actual_rep_penalty}"
    )

    gen_kwargs = {
        "temperature": actual_temp,
        "repetition_penalty": actual_rep_penalty,
    }

    if use_xvector_only:
        wavs, sr = model.generate_voice_clone(
            text=text,
            language="Auto",
            ref_audio=ref_audio_path,
            ref_text="",
            x_vector_only_mode=True,
            speed=actual_speed,
            **gen_kwargs,
        )
    else:
        wavs, sr = model.generate_voice_clone(
            text=text,
            language="Auto",
            ref_audio=ref_audio_path,
            ref_text=actual_ref_text,
            x_vector_only_mode=False,
            speed=actual_speed,
            **gen_kwargs,
        )

    # numpy array → torch tensor for resampling
    wav_tensor = torch.from_numpy(wavs[0]).unsqueeze(0).float()

    # Resample to 44.1kHz for FFmpeg pipeline compatibility
    if sr != OUTPUT_SAMPLE_RATE:
        wav_tensor = torchaudio.functional.resample(wav_tensor, sr, OUTPUT_SAMPLE_RATE)
        logger.info(f"Resampled: {sr}Hz → {OUTPUT_SAMPLE_RATE}Hz")

    # WAV binary encoding
    buf = io.BytesIO()
    sf.write(buf, wav_tensor.squeeze(0).numpy(), OUTPUT_SAMPLE_RATE, format="WAV", subtype="PCM_16")
    buf.seek(0)
    return buf.read()


@app.on_event("startup")
async def startup():
    """サーバー起動時にモデルをプリロード"""
    await asyncio.to_thread(load_model)
    logger.info(f"Server ready on port {PORT}")


@app.post("/v1/audio/speech")
async def create_speech(req: SpeechRequest):
    """OpenAI API 互換エンドポイント"""
    if not req.input or not req.input.strip():
        raise HTTPException(status_code=400, detail="Input text is empty.")

    if len(req.input) > 1500:
        raise HTTPException(status_code=400, detail="Input text too long (max 1500 chars). Split on Rust side.")

    try:
        wav_bytes = await asyncio.to_thread(
            synthesize, req.input, req.voice, req.ref_text,
            req.temperature, req.repetition_penalty, req.speed
        )
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Synthesis failed: {e}", exc_info=True)
        raise HTTPException(status_code=500, detail=f"Synthesis failed: {str(e)}")

    return Response(
        content=wav_bytes,
        media_type="audio/wav",
        headers={"Content-Disposition": "attachment; filename=speech.wav"},
    )


@app.get("/health")
async def health():
    """ヘルスチェック"""
    return {"status": "ok", "model": MODEL_ID, "device": str(next(model.parameters()).device) if model else "not_loaded"}


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(
        "tts_server:app",
        host=HOST,
        port=PORT,
        log_level="info",
        workers=1,  # モデルはプロセス内シングルトン
    )
