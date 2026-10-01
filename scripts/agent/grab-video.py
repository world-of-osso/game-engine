#!/usr/bin/env python3
"""Mux a fixture's wall-clock recording (POLY_GRAB dir: grab-*.jpg, frames.txt,
times.txt, audio.wav, audio.txt) into a 30 fps H.264 + AAC MP4 and check it.

Usage: grab-video.py <grab_dir> <fixture_log> <out_dir> [name]

Writes to <out_dir>:
  <name>.mp4        frames at their wall-clock durations, resampled to constant 30 fps
                    (a frame that lasted longer is repeated), audio from the first frame on
  timing.txt        ffprobe stream durations, achieved grab rate, frame-interval stats
  audio-check.txt   RMS (dBFS) of the MP4's audio around every `FIXTURE ... t=` event
  contact-sheet.png one MP4 frame per fixture MARK/event, labelled with its time
"""
import json
import re
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

EVENT = re.compile(r"^FIXTURE (\w+) t=([0-9.]+)\s*(.*)$")
RATE = 48000


def run(*args: str) -> str:
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def read_audio_meta(grab: Path) -> dict:
    meta = {}
    for line in (grab / "audio.txt").read_text().splitlines():
        key, value = line.split()
        meta[key] = float(value)
    return meta


def encode(grab: Path, mp4: Path, meta: dict) -> None:
    run(
        "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
        "-f", "concat", "-safe", "0", "-i", str(grab / "frames.txt"),
        "-itsoffset", f"{meta['audio_start_s']:.6f}", "-i", str(grab / "audio.wav"),
        "-map", "0:v", "-map", "1:a",
        "-fps_mode", "cfr", "-r", "30",
        "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p",
        "-c:a", "aac", "-b:a", "192k", "-ar", str(RATE),
        "-t", f"{meta['end_s']:.6f}", "-movflags", "+faststart", str(mp4),
    )


def stream_durations(mp4: Path) -> list[dict]:
    probe = json.loads(run(
        "ffprobe", "-v", "error", "-show_entries",
        "stream=codec_type,codec_name,duration,nb_frames,r_frame_rate,avg_frame_rate,sample_rate",
        "-of", "json", str(mp4),
    ))
    return probe["streams"]


def interval_stats(times: np.ndarray, end: float) -> list[str]:
    intervals = np.diff(np.append(times, end)) * 1000
    lines = [
        f"frames {len(times)} over {end:.3f} s: achieved {len(times) / end:.2f} fps (target 30)",
        "interval ms: " + " ".join(
            f"{label} {value:.1f}" for label, value in [
                ("min", intervals.min()), ("p10", np.percentile(intervals, 10)),
                ("median", np.median(intervals)), ("mean", intervals.mean()),
                ("p90", np.percentile(intervals, 90)), ("p99", np.percentile(intervals, 99)),
                ("max", intervals.max()),
            ]
        ),
    ]
    edges = [0, 30, 36, 50, 67, 100, 200, 500, 1e9]
    counts, _ = np.histogram(intervals, bins=edges)
    for low, high, count in zip(edges, edges[1:], counts):
        top = "inf" if high > 1e8 else f"{high:.0f}"
        lines.append(f"  [{low:.0f}, {top}) ms: {count} ({100 * count / len(intervals):.1f}%)")
    worst = np.argsort(intervals)[-5:][::-1]
    lines.append("longest gaps: " + ", ".join(f"{intervals[i]:.0f} ms at t={times[i]:.3f}" for i in worst))
    return lines


def decode_audio(mp4: Path) -> np.ndarray:
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(mp4), "-map", "0:a", "-ac", "1", "-ar", str(RATE),
         "-f", "f32le", "-"],
        check=True, capture_output=True,
    ).stdout
    return np.frombuffer(raw, dtype=np.float32)


def rms_db(samples: np.ndarray) -> float:
    if samples.size == 0:
        return float("-inf")
    rms = float(np.sqrt(np.mean(np.square(samples, dtype=np.float64))))
    return 20 * np.log10(rms) if rms > 0 else float("-inf")


def read_events(log: Path) -> list[tuple[float, str, str]]:
    events = []
    for line in log.read_text(errors="replace").splitlines():
        match = EVENT.match(line)
        if match:
            events.append((float(match[2]), match[1], match[3][:110]))
    return events


def audio_check(audio: np.ndarray, events: list[tuple[float, str, str]]) -> list[str]:
    window = RATE // 2
    blocks = audio[: len(audio) // window * window].reshape(-1, window)
    block_db = [rms_db(block) for block in blocks]
    lines = [
        f"audio {len(audio) / RATE:.3f} s, overall RMS {rms_db(audio):.1f} dBFS, "
        f"peak {20 * np.log10(np.abs(audio).max() + 1e-12):.1f} dBFS",
        f"0.5 s blocks: quietest {min(block_db):.1f} dBFS, loudest {max(block_db):.1f} dBFS, "
        f"silent (< -60 dBFS) {sum(db < -60 for db in block_db)} of {len(block_db)}",
        "event: RMS 0.5 s before -> 0.5 s from the event (dBFS)",
    ]
    for at, kind, detail in events:
        start = int(at * RATE)
        before = rms_db(audio[max(0, start - window):start])
        after = rms_db(audio[start:start + window])
        lines.append(f"  t={at:8.3f} {kind:<9} {before:6.1f} -> {after:6.1f}  {detail}")
    return lines


def contact_sheet(mp4: Path, events: list[tuple[float, str, str]], out: Path, work: Path) -> None:
    picks = [e for e in events if e[1] in ("MARK", "PRESS", "ACCEPTED", "SHEEP", "BROKEN")]
    tiles = []
    for index, (at, kind, detail) in enumerate(picks):
        still = work / f"sheet-{index:02d}.png"
        run("ffmpeg", "-y", "-v", "error", "-ss", f"{at:.3f}", "-i", str(mp4),
            "-frames:v", "1", "-vf", "scale=480:-2", str(still))
        tile = Image.open(still).convert("RGB")
        draw = ImageDraw.Draw(tile)
        label = f"t={at:.2f} {kind} {detail.split(' frame=')[0][:40]}"
        draw.rectangle([0, 0, tile.width, 18], fill=(0, 0, 0))
        draw.text((4, 3), label, fill=(255, 255, 255))
        tiles.append(tile)
        still.unlink()
    columns = 4
    width, height = tiles[0].size
    rows = (len(tiles) + columns - 1) // columns
    sheet = Image.new("RGB", (columns * width, rows * height), (24, 24, 24))
    for index, tile in enumerate(tiles):
        sheet.paste(tile, ((index % columns) * width, (index // columns) * height))
    sheet.save(out)


def main() -> None:
    grab, log, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    name = sys.argv[4] if len(sys.argv) > 4 else "polymorph-30fps"
    out.mkdir(parents=True, exist_ok=True)
    meta = read_audio_meta(grab)
    mp4 = out / f"{name}.mp4"
    encode(grab, mp4, meta)
    times = np.loadtxt(grab / "times.txt", ndmin=1)
    timing = [f"{s['codec_type']} {s['codec_name']}: duration {s.get('duration')} s, "
              f"frames {s.get('nb_frames')}, rate {s.get('avg_frame_rate') or s.get('sample_rate')}"
              for s in stream_durations(mp4)]
    timing += [f"recording: audio starts {meta['audio_start_s'] * 1000:.2f} ms after the first frame; "
               f"audio.wav {meta['audio_s']:.3f} s vs wall clock {meta['end_s']:.3f} s"]
    timing += interval_stats(times, meta["end_s"])
    (out / "timing.txt").write_text("\n".join(timing) + "\n")
    events = read_events(log)
    (out / "audio-check.txt").write_text("\n".join(audio_check(decode_audio(mp4), events)) + "\n")
    contact_sheet(mp4, events, out / "contact-sheet.png", out)
    print((out / "timing.txt").read_text())
    print((out / "audio-check.txt").read_text())


if __name__ == "__main__":
    main()
