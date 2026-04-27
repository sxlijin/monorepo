#!/usr/bin/env python
"""Download a URL with yt-dlp as a WAV file and split it into stems with demucs."""
import argparse
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "src" / "analysis"))

from stem_separation import separate_stems


def download_wav(url: str, music_dir: Path) -> Path:
    output_template = str(music_dir / "%(title)s.%(ext)s")
    proc = subprocess.run(
        [
            "uv", "run", "yt-dlp",
            "--extract-audio",
            "--audio-format", "wav",
            "--embed-metadata",
            "--output", output_template,
            url,
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        msg = proc.stderr.strip() or proc.stdout.strip()
        raise RuntimeError(f"yt-dlp failed: {msg}")

    prefix = "[ExtractAudio] Destination: "
    for line in (proc.stdout + "\n" + proc.stderr).splitlines():
        stripped = line.strip()
        if stripped.startswith(prefix):
            return Path(stripped[len(prefix):].strip())

    raise RuntimeError(
        f"Could not determine downloaded file path from yt-dlp output.\n"
        f"stdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url", help="YouTube (or any yt-dlp-supported) URL")
    parser.add_argument(
        "--music-dir",
        default=str(Path.home() / "Music"),
        help="Directory to download into (default: ~/Music)",
    )
    args = parser.parse_args()

    music_dir = Path(args.music_dir).expanduser()
    music_dir.mkdir(parents=True, exist_ok=True)

    print(f"Downloading {args.url} into {music_dir} ...", flush=True)
    wav_path = download_wav(args.url, music_dir)
    print(f"Downloaded: {wav_path}", flush=True)

    stems_root = music_dir / "stems"
    print(f"Separating stems into {stems_root} ...", flush=True)
    result = separate_stems(str(wav_path), str(stems_root))

    if not result["success"]:
        print(f"Stem separation failed: {result['error']}", file=sys.stderr)
        return 1

    print(f"Stems written to: {result['stem_dir']}")
    for name in result["generated_files"]:
        print(f"  - {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
