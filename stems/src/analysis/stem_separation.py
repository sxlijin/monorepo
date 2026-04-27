import shutil
import subprocess
from pathlib import Path
from typing import Optional


def separate_stems(audio_path: str, stems_root: str) -> dict:
    result = {
        "success": False,
        "stem_dir": None,
        "generated_files": [],
        "error": None,
    }

    temp_wav_path: Optional[Path] = None

    try:
        audio_path = Path(audio_path).expanduser().resolve()
        stems_root_path = Path(stems_root).expanduser()
        stems_root_path.mkdir(parents=True, exist_ok=True)

        if not audio_path.exists():
            raise FileNotFoundError(f"Audio file not found: {audio_path}")

        song_name = audio_path.stem
        demucs_out_root = stems_root_path.parent if stems_root_path.parent != stems_root_path else stems_root_path

        working_audio_path = audio_path
        # if audio_path.suffix.lower() != ".wav":
        #     temp_wav_path = stems_root_path / f"{audio_path.stem}.converted.wav"
        #     try:
        #         data, sample_rate = sf.read(audio_path, always_2d=True)
        #         data = data.astype(np.float32)
        #     except (OSError, ValueError, sf.SoundFileError):
        #         loaded, sample_rate = librosa.load(audio_path, sr=None, mono=False)
        #         if loaded.ndim == 1:
        #             loaded = loaded[np.newaxis, :]
        #         data = loaded.T.astype(np.float32)
        #     sf.write(temp_wav_path, data, sample_rate)
        #     working_audio_path = temp_wav_path

        cmd = [
            "uv",
            "run",
            "demucs",
            "--out",
            str(demucs_out_root),
            str(working_audio_path),
        ]

        process = subprocess.run(cmd, capture_output=True, text=True)
        if process.returncode != 0:
            stderr = process.stderr.strip()
            stdout = process.stdout.strip()
            raise RuntimeError(
                f"demucs failed (exit {process.returncode}): {stderr or stdout}"
            )

        separated_dir = demucs_out_root / "htdemucs" / song_name
        if not separated_dir.exists():
            raise FileNotFoundError(
                f"Separated stems directory not found: {separated_dir}"
            )

        final_dir = stems_root_path / song_name
        if final_dir.exists():
            shutil.rmtree(final_dir)
        final_dir.mkdir(parents=True, exist_ok=True)

        generated = []
        for stem_file in sorted(separated_dir.glob("*.wav")):
            target_path = final_dir / stem_file.name
            shutil.move(str(stem_file), str(target_path))
            generated.append(target_path.name)

        # Clean up demucs artifacts if possible
        try:
            shutil.rmtree(separated_dir)
            htdemucs_dir = separated_dir.parent
            if htdemucs_dir.exists() and not any(htdemucs_dir.iterdir()):
                htdemucs_dir.rmdir()
        except Exception:
            pass

        result.update(
            {
                "success": True,
                "stem_dir": str(final_dir),
                "generated_files": generated,
            }
        )
    except Exception as exc:
        result["error"] = str(exc)
    finally:
        if temp_wav_path and temp_wav_path.exists():
            try:
                temp_wav_path.unlink()
            except Exception:
                pass

    return result
