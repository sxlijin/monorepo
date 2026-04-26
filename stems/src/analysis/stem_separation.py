import math
import shutil
import subprocess
from pathlib import Path
from typing import List, Optional

import librosa
import numpy as np
import soundfile as sf


def _design_fir_lowpass(cutoff_hz: float, sample_rate: int, taps: int) -> np.ndarray:
    if taps % 2 == 0:
        raise ValueError("FIR filter length must be odd for linear phase")

    nyquist = sample_rate / 2.0
    if not 0 < cutoff_hz < nyquist:
        raise ValueError("Cutoff must be between 0 and Nyquist frequency")

    normalized_cutoff = cutoff_hz / nyquist
    center = (taps - 1) / 2
    n = np.arange(taps, dtype=np.float64) - center
    h = np.sinc(2 * normalized_cutoff * n)
    window = np.hamming(taps)
    h *= window
    h /= np.sum(h)
    return h.astype(np.float32)


def _design_fir_highpass(cutoff_hz: float, sample_rate: int, taps: int) -> np.ndarray:
    lowpass = _design_fir_lowpass(cutoff_hz, sample_rate, taps)
    impulse = np.zeros_like(lowpass)
    impulse[taps // 2] = 1.0
    return (impulse - lowpass).astype(np.float32)


def _apply_fir_filter(audio: np.ndarray, kernel: np.ndarray) -> np.ndarray:
    if audio.ndim != 2:
        raise ValueError("Audio array must be 2D (samples, channels)")

    filtered_channels: List[np.ndarray] = []
    for channel in range(audio.shape[1]):
        filtered = np.convolve(audio[:, channel], kernel, mode="same")
        filtered_channels.append(filtered.astype(np.float32))

    return np.stack(filtered_channels, axis=1)


def _spectral_subtract(
    full_band: np.ndarray,
    low_band: np.ndarray,
    sample_rate: int,
    alpha: float = 0.7,
    n_fft: int = 2048,
    hop_length: int = 512,
) -> np.ndarray:
    if full_band.ndim != 2 or low_band.ndim != 2:
        raise ValueError("Spectral subtraction expects 2D arrays (samples, channels)")
    if full_band.shape[1] != low_band.shape[1]:
        raise ValueError("Channel mismatch between full_band and low_band")

    length = min(full_band.shape[0], low_band.shape[0])
    if length == 0:
        return full_band

    full_view = full_band[:length]
    low_view = low_band[:length]
    result = np.zeros_like(full_view, dtype=np.float32)

    for channel in range(full_view.shape[1]):
        full_channel = full_view[:, channel].astype(np.float32)
        low_channel = low_view[:, channel].astype(np.float32)

        stft_full = librosa.stft(
            full_channel,
            n_fft=n_fft,
            hop_length=hop_length,
            win_length=n_fft,
            window="hann",
        )
        stft_low = librosa.stft(
            low_channel,
            n_fft=n_fft,
            hop_length=hop_length,
            win_length=n_fft,
            window="hann",
        )

        mag_full = np.abs(stft_full)
        mag_low = np.abs(stft_low)

        mag_est = np.maximum(mag_full - alpha * mag_low, 0.0)
        phase_full = np.angle(stft_full)
        stft_est = mag_est * np.exp(1j * phase_full)

        reconstructed = librosa.istft(
            stft_est,
            hop_length=hop_length,
            win_length=n_fft,
            window="hann",
            length=length,
        )

        result[:, channel] = reconstructed.astype(np.float32)

    if full_band.shape[0] > length:
        padded = np.zeros_like(full_band, dtype=np.float32)
        padded[:length] = result
        return padded

    return result


def ensure_drum_placeholders(stem_dir: Path) -> bool:
    drums = stem_dir / "drums.wav"
    hi = stem_dir / "drums-hi.wav"
    lo = stem_dir / "drums-lo.wav"

    if not drums.exists():
        return False

    try:
        audio, sample_rate = sf.read(drums, dtype="float32", always_2d=True)
    except (OSError, ValueError, sf.SoundFileError):
        return False

    cutoff_hz = 200.0
    taps = 129

    try:
        low_kernel = _design_fir_lowpass(cutoff_hz, sample_rate, taps)
        high_kernel = _design_fir_highpass(cutoff_hz, sample_rate, taps)
    except ValueError:
        return False

    low_band = _apply_fir_filter(audio, low_kernel)
    high_band = _apply_fir_filter(audio, high_kernel)
    high_band = _spectral_subtract(high_band, low_band, sample_rate)

    modified = False
    for band, target in ((low_band, lo), (high_band, hi)):
        peak = np.max(np.abs(band))
        if peak > 1.0 and not math.isclose(peak, 0.0):
            band = band / peak
        sf.write(target, band, sample_rate)
        modified = True

    return modified


def separate_stems(audio_path: str, stems_root: str) -> dict:
    result = {
        "success": False,
        "stem_dir": None,
        "generated_files": [],
        "drum_split_performed": False,
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

        drum_split = ensure_drum_placeholders(final_dir)

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
                "drum_split_performed": drum_split,
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
