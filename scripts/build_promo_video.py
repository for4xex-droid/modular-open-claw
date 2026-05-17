#!/usr/bin/env python3
import os
import subprocess
import argparse
from pathlib import Path

def run_ffmpeg(cmd):
    print(f"Running: {' '.join(cmd)}")
    subprocess.run(cmd, check=True)

def build_trailer(base_video: str, logo_image: str, jingle_audio: str, bgm_audio: str, output_file: str):
    print("🎬 Starting High-Quality Trailer Generation...")
    
    # Paths
    base_video_path = Path(base_video)
    logo_path = Path(logo_image)
    jingle_path = Path(jingle_audio) if jingle_audio else None
    bgm_path = Path(bgm_audio) if bgm_audio else None
    out_path = Path(output_file)
    
    if not base_video_path.exists():
        raise FileNotFoundError(f"Base video not found: {base_video_path}")
        
    temp_dir = Path("/tmp/aiome_trailer_build")
    temp_dir.mkdir(parents=True, exist_ok=True)
    
    # 1. Probe video duration
    duration_cmd = [
        "ffprobe", "-v", "error", "-show_entries",
        "format=duration", "-of", "default=noprint_wrappers=1:nokey=1",
        str(base_video_path)
    ]
    try:
        duration_str = subprocess.check_output(duration_cmd).decode("utf-8").strip()
        base_duration = float(duration_str)
    except Exception as e:
        print(f"Failed to probe duration: {e}. Defaulting to 90s.")
        base_duration = 90.0

    print(f"✅ Base video duration: {base_duration:.2f} seconds")

    # Define the complex filter graph for high-quality composition
    # We will:
    # 1. Scale video to standard 1080p (1920x1080)
    # 2. Add an opening sequence (3 seconds) with the logo fading in and out over black
    # 3. Add the main video fading in
    # 4. Add a closing sequence (3 seconds) with the logo fading in and out
    
    # For now, we will construct a robust basic overlay and fade script.
    # [0:v] is base_video, [1:v] is logo_image.
    
    # If users provide specific logo/audio, we prepare a high-end ffmpeg filter graph.
    # Since filter graphs get very complex, we use a bash script approach internally or structured array.
    
    filter_complex = []
    
    # --- Video Processing ---
    # Scale base video to 1920x1080 padding with black to maintain aspect ratio
    filter_complex.append("[0:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2,format=yuv420p[base];")
    
    # Apply fade in to the base video
    filter_complex.append(f"[base]fade=t=in:st=0:d=1.5,fade=t=out:st={base_duration-1.5}:d=1.5[main_v];")
    
    # Scale logo and position it at the center for opening/closing, or watermark
    # (Depending on user preference, we can just overlay it as a watermark for now, 
    # until we build the full sequence intro/outro)
    filter_complex.append("[1:v]scale=300:-1[logo];")
    filter_complex.append("[main_v][logo]overlay=main_w-overlay_w-40:40[out_v]")
    
    # --- Audio Processing ---
    audio_inputs = []
    audio_filter = ""
    
    ffmpeg_cmd = [
        "ffmpeg", "-y",
        "-i", str(base_video_path),
        "-i", str(logo_path),
    ]
    
    audio_idx = 2
    if jingle_path and jingle_path.exists():
        ffmpeg_cmd.extend(["-i", str(jingle_path)])
        audio_inputs.append(f"[{audio_idx}:a]")
        audio_idx += 1
        
    if bgm_path and bgm_path.exists():
        # Loop BGM if it's shorter than video
        ffmpeg_cmd.extend(["-stream_loop", "-1", "-i", str(bgm_path)])
        # Fade out BGM at the end
        audio_inputs.append(f"[{audio_idx}:a]afade=t=out:st={base_duration-3}:d=3[bgm_faded];")
        audio_idx += 1
        
    # Combine audio
    if len(audio_inputs) == 0:
        # Generate silent audio if none provided
        ffmpeg_cmd.extend(["-f", "lavfi", "-i", "anullsrc=channel_layout=stereo:sample_rate=44100"])
        audio_filter = f"[{audio_idx}:a]aformat=sample_fmts=fltp:sample_rates=44100:channel_layouts=stereo[out_a]"
    elif len(audio_inputs) == 1:
        # Just use the one audio track
        fade_start = max(0, base_duration - 3)
        if bgm_path:
            audio_filter = f"[{audio_idx-1}:a]afade=t=out:st={fade_start}:d=3[bgm_faded];[bgm_faded]aformat=sample_fmts=fltp:sample_rates=44100:channel_layouts=stereo[out_a]"
        else:
            audio_filter = f"{audio_inputs[0]}aformat=sample_fmts=fltp:sample_rates=44100:channel_layouts=stereo[out_a]"
    else:
        # Mix jingle and BGM
        fade_start = max(0, base_duration - 3)
        if bgm_path:
            # We need to prepend the bgm fade filter if it exists
            audio_filter = f"[{audio_idx-1}:a]afade=t=out:st={fade_start}:d=3[bgm_faded];" + "".join(audio_inputs[:1]) + "[bgm_faded]amix=inputs=2:duration=first:dropout_transition=2[out_a]"

    full_filter = "".join(filter_complex) + (";" + audio_filter if audio_filter else "")

    ffmpeg_cmd.extend([
        "-filter_complex", full_filter,
        "-map", "[out_v]",
        "-map", "[out_a]",
        "-c:v", "libx264",
        "-preset", "slow",
        "-crf", "18",      # High quality
        "-c:a", "aac",
        "-b:a", "192k",
        "-t", str(base_duration), # Cut exactly at base duration
        str(out_path)
    ])
    
    run_ffmpeg(ffmpeg_cmd)
    print(f"🎉 High-Quality Trailer successfully generated at: {out_path}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Aiome Trailer Generator")
    parser.add_argument("--video", required=True, help="Base screencast video (.webm/.mp4)")
    parser.add_argument("--logo", required=True, help="High-res transparent logo (.png)")
    parser.add_argument("--jingle", help="Audio jingle for the start (.mp3/.wav)")
    parser.add_argument("--bgm", help="Background music to loop (.mp3/.wav)")
    parser.add_argument("--out", default="promo_trailer.mp4", help="Output file path")
    
    args = parser.parse_args()
    build_trailer(args.video, args.logo, args.jingle, args.bgm, args.out)
