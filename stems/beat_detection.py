#!/usr/bin/env python3
"""
Beat detection module using librosa for the Stems music player.
Provides beat tracking functionality for audio files.
"""

import librosa
import numpy as np
import sys
import json
from typing import List, Tuple, Dict, Any


def detect_beats(audio_file_path: str, sr: int = 22050) -> Dict[str, Any]:
    """
    Detect beats in an audio file using librosa.
    
    Args:
        audio_file_path: Path to the audio file
        sr: Sample rate for analysis (default 22050 Hz)
        
    Returns:
        Dictionary containing beat timestamps and tempo information
    """
    try:
        # Load the audio file
        print(f"Loading audio file: {audio_file_path}", file=sys.stderr)
        y, sr = librosa.load(audio_file_path, sr=sr)
        
        # Detect beats using librosa's beat tracker
        print("Detecting beats...", file=sys.stderr)
        tempo, beat_frames = librosa.beat.beat_track(y=y, sr=sr)
        
        # Convert beat frames to timestamps (seconds)
        beat_times = librosa.frames_to_time(beat_frames, sr=sr)
        
        # Get additional tempo and rhythm information
        onset_frames = librosa.onset.onset_detect(y=y, sr=sr)
        onset_times = librosa.frames_to_time(onset_frames, sr=sr)
        
        # Calculate beat intervals for consistency checking
        beat_intervals = np.diff(beat_times) if len(beat_times) > 1 else []
        avg_beat_interval = np.mean(beat_intervals) if len(beat_intervals) > 0 else 0.0
        
        result = {
            "success": True,
            "tempo": float(tempo),
            "beat_count": len(beat_times),
            "beat_timestamps": beat_times.tolist(),
            "onset_count": len(onset_times),
            "onset_timestamps": onset_times.tolist(),
            "avg_beat_interval": float(avg_beat_interval),
            "duration": float(len(y) / sr),
            "sample_rate": int(sr)
        }
        
        print(f"Beat detection complete: {len(beat_times)} beats found", file=sys.stderr)
        print(f"Estimated tempo: {float(tempo):.1f} BPM", file=sys.stderr)
        
        return result
        
    except Exception as e:
        print(f"Error during beat detection: {e}", file=sys.stderr)
        return {
            "success": False,
            "error": str(e),
            "beat_timestamps": [],
            "onset_timestamps": [],
            "tempo": 0.0,
            "beat_count": 0,
            "onset_count": 0
        }


def analyze_beat_consistency(beat_times: List[float]) -> Dict[str, float]:
    """
    Analyze the consistency of detected beats.
    
    Args:
        beat_times: List of beat timestamps in seconds
        
    Returns:
        Dictionary with consistency metrics
    """
    if len(beat_times) < 2:
        return {"consistency": 0.0, "std_deviation": 0.0, "regularity": 0.0}
    
    intervals = np.diff(beat_times)
    mean_interval = np.mean(intervals)
    std_deviation = np.std(intervals)
    
    # Calculate regularity as inverse of coefficient of variation
    cv = std_deviation / mean_interval if mean_interval > 0 else float('inf')
    regularity = 1.0 / (1.0 + cv)
    
    # Overall consistency score (0-1, higher is more consistent)
    consistency = max(0.0, 1.0 - (std_deviation / mean_interval))
    
    return {
        "consistency": float(consistency),
        "std_deviation": float(std_deviation),
        "regularity": float(regularity),
        "mean_interval": float(mean_interval)
    }


def main():
    """
    Command-line interface for beat detection.
    Usage: python beat_detection.py <audio_file_path>
    """
    if len(sys.argv) != 2:
        print("Usage: python beat_detection.py <audio_file_path>", file=sys.stderr)
        sys.exit(1)
    
    audio_file = sys.argv[1]
    
    # Perform beat detection
    result = detect_beats(audio_file)
    
    if result["success"]:
        # Add consistency analysis
        consistency = analyze_beat_consistency(result["beat_timestamps"])
        result["consistency"] = consistency
        
        # Print summary to stderr for debugging
        print(f"\nBeat Detection Summary:", file=sys.stderr)
        print(f"  File: {audio_file}", file=sys.stderr)
        print(f"  Duration: {result['duration']:.2f} seconds", file=sys.stderr)
        print(f"  Tempo: {float(result['tempo']):.1f} BPM", file=sys.stderr)
        print(f"  Beats: {result['beat_count']}", file=sys.stderr)
        print(f"  Onsets: {result['onset_count']}", file=sys.stderr)
        print(f"  Consistency: {consistency['consistency']:.3f}", file=sys.stderr)
        print(f"  Regularity: {consistency['regularity']:.3f}", file=sys.stderr)
        
        # Print first few beats for verification
        if result["beat_timestamps"]:
            print(f"  First 5 beats: {result['beat_timestamps'][:5]}", file=sys.stderr)
    
    # Output JSON result to stdout for consumption by Rust
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()