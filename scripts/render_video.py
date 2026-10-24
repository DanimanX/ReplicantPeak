#!/usr/bin/env python3
"""
Full video renderer for ReplicantPeak Cyberpunk / Blade Runner demonstration.
Generates 1920x1080 Full HD 30fps video synchronized with the synthesized 48kHz audio track.
"""

import math
import subprocess
import sys
import time
import wave
import numpy as np
from PIL import Image, ImageDraw, ImageFont

def render_video():
    wav_path = "assets/cyberpunk_blade_runner.wav"
    out_mp4 = "assets/cyberpunk_blade_runner_demo.mp4"
    panel_path = "assets/panel.png"

    print(f"Loading audio from {wav_path}...")
    with wave.open(wav_path, "rb") as w:
        rate = w.getframerate()
        nframes = w.getnframes()
        dur = nframes / rate
        audio = np.frombuffer(w.readframes(nframes), dtype=np.int16).astype(np.float32) / 32768.0
        audio = audio.reshape(-1, 2)
        audio_l = audio[:, 0]
        audio_r = audio[:, 1]

    fps = 30
    total_video_frames = int(dur * fps)
    print(f"Audio duration: {dur:.2f}s | Target video frames: {total_video_frames} @ {fps} fps")

    print(f"Loading chassis panel from {panel_path}...")
    panel_img = Image.open(panel_path).convert("RGB")

    w, h = 1920, 1080
    panel_x = (w - panel_img.width) // 2
    panel_y = 95

    # Fonts
    try:
        font_title = ImageFont.truetype("/usr/share/fonts/TTF/RobotoCondensed-Bold.ttf", 26)
        font_sub = ImageFont.truetype("/usr/share/fonts/TTF/RobotoCondensed-Light.ttf", 15)
        font_mono = ImageFont.truetype("/usr/share/fonts/TTF/Roboto-Regular.ttf", 14)
        font_hud = ImageFont.truetype("/usr/share/fonts/TTF/RobotoCondensed-Bold.ttf", 17)
    except Exception:
        font_title = font_sub = font_mono = font_hud = ImageFont.load_default()

    # Launch ffmpeg process
    ffmpeg_cmd = [
        "ffmpeg", "-y",
        "-f", "rawvideo",
        "-vcodec", "rawvideo",
        "-s", f"{w}x{h}",
        "-pix_fmt", "rgb24",
        "-r", str(fps),
        "-i", "-",
        "-i", wav_path,
        "-c:v", "libx264",
        "-preset", "faster",
        "-crf", "19",
        "-pix_fmt", "yuv420p",
        "-c:a", "aac",
        "-b:a", "320k",
        "-shortest",
        out_mp4
    ]

    print(f"Spawning ffmpeg encoder...")
    proc = subprocess.Popen(ffmpeg_cmd, stdin=subprocess.PIPE)

    oled_screen_x0 = panel_x + 755
    oled_screen_y0 = panel_y + 105
    oled_w = 205
    oled_h = 80
    oled_mid_y = oled_screen_y0 + oled_h // 2

    leds_y = panel_y + 218
    led_cell_w = 385.0 / 8.0

    spec_x0 = 160
    spec_w = w - 320
    spec_h = 95
    spec_y0 = 960
    num_bars = 64
    bar_pitch = spec_w / float(num_bars)
    bar_w = max(2, int(bar_pitch * 0.72))

    t_start = time.time()

    # Pre-render base background once to accelerate rendering
    base_bg = Image.new("RGB", (w, h), (4, 7, 14))
    draw_bg = ImageDraw.Draw(base_bg)

    # Perspective Grid Horizon
    horizon_y = 760
    for y in range(horizon_y, h, 24):
        dist = (y - horizon_y) / (h - horizon_y)
        alpha = int(40 + 120 * dist)
        draw_bg.line([(0, y), (w, y)], fill=(0, int(alpha * 0.8), alpha))

    vp_x = w // 2
    for x in range(-400, w + 400, 75):
        draw_bg.line([(vp_x + (x - vp_x) * 0.05, horizon_y), (x, h)], fill=(0, 60, 90))

    # Panel border & paste
    draw_bg.rectangle([panel_x - 3, panel_y - 3, panel_x + panel_img.width + 3, panel_y + panel_img.height + 3], outline=(0, 180, 220), width=1)
    base_bg.paste(panel_img, (panel_x, panel_y))

    # Top Static Header
    draw_bg.rectangle([0, 0, w, 80], fill=(2, 4, 8))
    draw_bg.line([(0, 80), (w, 80)], fill=(0, 180, 220), width=2)
    draw_bg.text((45, 16), "// REPLICANT PEAK  •  NEW OXFORD NCO SYNTHESIZER", fill=(0, 245, 255), font=font_title)
    draw_bg.text((45, 48), "HARDWARE EMULATION // 8-VOICE ANALOG HYBRID // CHRIS HUGGETT 24dB OTA FILTER", fill=(120, 170, 190), font=font_sub)

    # Precompute spectrum visualizer history for smooth ballistics
    prev_mags = np.zeros(num_bars, dtype=np.float32)

    for frame_idx in range(total_video_frames):
        im = base_bg.copy()
        draw = ImageDraw.Draw(im)

        t_sec = frame_idx / float(fps)
        sample_pos = int((frame_idx / float(fps)) * rate)
        slice_len = 1024
        if sample_pos + slice_len < len(audio_l):
            wave_slice = audio_l[sample_pos:sample_pos + slice_len]
        else:
            wave_slice = np.zeros(slice_len, dtype=np.float32)

        rms_val = float(np.sqrt(np.mean(wave_slice**2)))

        # 1. Header Dynamic Telemetry
        secs = int(t_sec)
        frames_rem = int((t_sec - secs) * fps)
        tc_str = f"TC: 00:00:{secs:02d}:{frames_rem:02d}  |  48 kHz / 32-bit float  |  M4: 168 MHz"
        draw.text((w - 480, 22), tc_str, fill=(0, 220, 240), font=font_mono)
        draw.text((w - 480, 48), "PRESET: BLADE RUNNER CS-80 + TNT SADTOUCH", fill=(255, 170, 0), font=font_mono)

        # 2. Dynamic OLED Scope
        draw.rectangle([oled_screen_x0, oled_screen_y0, oled_screen_x0 + oled_w, oled_screen_y0 + oled_h], fill=(2, 8, 10))
        draw.line([(oled_screen_x0, oled_mid_y), (oled_screen_x0 + oled_w, oled_mid_y)], fill=(5, 25, 28))
        draw.line([(oled_screen_x0 + oled_w//2, oled_screen_y0), (oled_screen_x0 + oled_w//2, oled_screen_y0 + oled_h)], fill=(5, 25, 28))

        step = len(wave_slice) / float(oled_w)
        scope_pts = []
        for px in range(oled_w):
            idx = int(px * step)
            val = wave_slice[idx]
            py = int(oled_mid_y - val * (oled_h * 0.44))
            py = max(oled_screen_y0 + 2, min(oled_screen_y0 + oled_h - 2, py))
            scope_pts.append((oled_screen_x0 + px, py))

        for i in range(len(scope_pts) - 1):
            p1, p2 = scope_pts[i], scope_pts[i+1]
            draw.line([p1, p2], fill=(0, 220, 240), width=2)
            draw.line([p1, p2], fill=(220, 255, 255), width=1)

        # 3. Dynamic Voice Activity LEDs
        num_active_voices = min(8, int(1 + rms_val * 35.0))
        if t_sec > 18.5 and t_sec < 37.0:
            num_active_voices = min(8, max(num_active_voices, 4 + (frame_idx % 4)))

        for v in range(8):
            cx = int(panel_x + 758 + (v + 0.5) * led_cell_w)
            cy = leds_y
            is_lit = (v < num_active_voices) and (rms_val > 0.005)
            if is_lit:
                draw.ellipse([cx - 7, cy - 7, cx + 7, cy + 7], fill=(0, 80, 110))
                draw.ellipse([cx - 4, cy - 4, cx + 4, cy + 4], fill=(0, 220, 255))
                draw.ellipse([cx - 2, cy - 2, cx + 2, cy + 2], fill=(255, 255, 255))
            else:
                draw.ellipse([cx - 3, cy - 3, cx + 3, cy + 3], fill=(10, 25, 30))

        # 4. Section Banner
        if t_sec < 9.5:
            section_txt = ">> SECTION 1 // \"NEON HORIZON\" — TNT SADTOUCH PAD + 24dB ANALOG SUB DRONE"
            sec_color = (0, 240, 255)
        elif t_sec < 18.5:
            section_txt = ">> SECTION 2 // \"REPLICANT THEME\" — CS-80 EXPRESSIVE LEAD (OXFORD TRIPLE-SAW + LADDER SWEEP)"
            sec_color = (255, 180, 0)
        elif t_sec < 27.5:
            section_txt = ">> SECTION 3 // \"NEON PURSUIT\" — 16TH-NOTE CYBERPUNK ARPEGGIO + DUAL-DELAY ECHO"
            sec_color = (255, 0, 120)
        elif t_sec < 36.5:
            section_txt = ">> SECTION 4 // \"OFF-WORLD CLIMAX\" — HIGH OCTAVE VANGELIS LEAD + LUSH ANALOG CHORUS"
            sec_color = (0, 255, 200)
        else:
            section_txt = ">> SECTION 5 // \"TEARS IN REPLICANT\" — ANALOG REVERB DECAY & TAPE DELAY RESOLUTION"
            sec_color = (160, 190, 255)

        draw.rectangle([w//2 - 480, 920, w//2 + 480, 948], fill=(6, 12, 22))
        draw.rectangle([w//2 - 480, 920, w//2 + 480, 948], outline=sec_color, width=1)
        draw.text((w//2 - 460, 924), section_txt, fill=sec_color, font=font_hud)

        # 5. Spectrum Visualizer
        fft_slice = np.abs(np.fft.rfft(wave_slice * np.hanning(len(wave_slice))))[:len(wave_slice)//2]
        fft_bands = np.array_split(fft_slice, num_bars)
        raw_mags = np.array([float(np.mean(b)) for b in fft_bands], dtype=np.float32)
        # Peak decay smoothing
        prev_mags = np.maximum(raw_mags, prev_mags * 0.82)
        max_m = max(1.0, float(np.max(prev_mags)))

        for b in range(num_bars):
            bx = int(spec_x0 + b * bar_pitch)
            norm_mag = min(1.0, (prev_mags[b] / max_m) ** 0.65)
            bh = int(norm_mag * spec_h)
            by = spec_y0 + spec_h - bh

            frac = b / float(num_bars)
            r_c = int(0 + 255 * frac)
            g_c = int(240 * (1.0 - frac * 0.6))
            b_c = int(255 * (0.8 + 0.2 * frac))

            draw.rectangle([bx, by, bx + bar_w, spec_y0 + spec_h], fill=(r_c, g_c, b_c))
            draw.line([(bx, by), (bx + bar_w, by)], fill=(255, 255, 255), width=1)

        # Pipe frame bytes into ffmpeg
        proc.stdin.write(im.tobytes())

        if frame_idx % 150 == 0 or frame_idx == total_video_frames - 1:
            pct = (frame_idx + 1) / total_video_frames * 100.0
            elapsed = time.time() - t_start
            fps_act = (frame_idx + 1) / elapsed if elapsed > 0 else 0
            print(f"Progress: {pct:5.1f}% ({frame_idx+1}/{total_video_frames}) | {fps_act:.1f} fps | Elapsed: {elapsed:.1f}s")

    proc.stdin.close()
    proc.wait()
    total_time = time.time() - t_start
    print(f"Video encoding complete in {total_time:.1f}s! Saved to {out_mp4}")

if __name__ == "__main__":
    render_video()
