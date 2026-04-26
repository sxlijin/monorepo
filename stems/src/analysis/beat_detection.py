import sys
import os

# Ensure our project and optional virtualenv site-packages are available without
# clobbering the interpreter's default sys.path (which provides the stdlib).
EXTRA_IMPORT_PATHS = [
    '.',
    '/Users/sam/sam-repos/stems/src/analysis',
    '/Users/sam/sam-repos/stems/.venv/lib/python3.13/site-packages',
]

for path in EXTRA_IMPORT_PATHS:
    if path and path not in sys.path:
        sys.path.insert(0, path)

import numpy as np
import librosa

def detect_beats(audio_data, sample_rate):
    import numpy as np
    import librosa
    """
    Detect beats in audio data using librosa.
    
    Args:
        audio_data: List of audio samples (normalized to [-1.0, 1.0])
        sample_rate: Audio sample rate in Hz
    
    Returns:
        Dictionary containing beat detection results
    """
    # Convert audio_data to numpy array
    y = np.array(audio_data, dtype=np.float32)

    # Try onset-based approach first for very early beat detection
    onset_frames = librosa.onset.onset_detect(
        y=y, 
        sr=sample_rate,
        # hop_length=512,
        # pre_max=20,           # Longer pre-max for better peak picking
        # post_max=20,          # Longer post-max for better peak picking
        # pre_avg=100,          # Longer pre-avg for better background estimation
        # post_avg=100,         # Longer post-avg for better background estimation
        # delta=0.07,           # Lower threshold for onset detection
        # wait=15               # Shorter wait between onsets
    )
    
    # Detect beats using librosa's beat tracker with aggressive early detection
    tempo, beat_frames = librosa.beat.beat_track(
        y=y, 
        sr=sample_rate,
        onset_envelope=None,  # Let it compute its own
        units='frames',       # Work in frames for precision
        trim=False,          # Don't trim silence at start
        # hop_length=512,      # Higher resolution
        # tightness=10,        # Very flexible timing for earlier detection
        # prior=None           # No tempo prior constraint
    )
    
    # If we have onsets but no beats early enough, try to align beats with early onsets
    # if len(onset_frames) > 0 and len(beat_frames) > 0:
    #     onset_times_temp = librosa.frames_to_time(onset_frames, sr=sample_rate, hop_length=512)
    #     beat_times_temp = librosa.frames_to_time(beat_frames, sr=sample_rate, hop_length=512)
        
    #     # If first beat is later than first onset + 1 second, try to add earlier beats
    #     if len(beat_times_temp) > 0 and len(onset_times_temp) > 0:
    #         first_onset = onset_times_temp[0]
    #         first_beat = beat_times_temp[0]
            
    #         print(f'First onset: {first_onset:.3f}s, First beat: {first_beat:.3f}s')
            
    #         if first_beat > first_onset + 0.2:  # If beat is more than 0.2s after first onset
    #             # Calculate beat interval from detected beats
    #             if len(beat_times_temp) > 1:
    #                 beat_interval = beat_times_temp[1] - beat_times_temp[0]
    #                 # Try to prepend beats before the first detected beat
    #                 earlier_beats = []
    #                 test_time = first_beat - beat_interval
    #                 while test_time >= first_onset and test_time >= 0:
    #                     earlier_beats.insert(0, test_time)
    #                     test_time -= beat_interval
                    
    #                 if earlier_beats:
    #                     print(f'Adding {len(earlier_beats)} earlier beats starting at {earlier_beats[0]:.3f}s')
    #                     # Convert back to frames and combine
    #                     earlier_frames = librosa.time_to_frames(earlier_beats, sr=sample_rate, hop_length=512)
    #                     beat_frames = np.concatenate([earlier_frames, beat_frames])

    # Convert beat frames to timestamps (seconds)
    beat_times = librosa.frames_to_time(beat_frames, sr=sample_rate)
    print('beat_times', beat_times[:5])

    # Get additional tempo and rhythm information
    onset_frames = librosa.onset.onset_detect(y=y, sr=sample_rate)
    onset_times = librosa.frames_to_time(onset_frames, sr=sample_rate)
    print('onset_times', onset_times[:5])

    # Calculate beat intervals for consistency checking
    if len(beat_times) > 1:
        beat_intervals = np.diff(beat_times)
        avg_beat_interval = float(np.mean(beat_intervals))
        std_deviation = float(np.std(beat_intervals))
        mean_interval = float(np.mean(beat_intervals))
        cv = std_deviation / mean_interval if mean_interval > 0 else float('inf')
        regularity = 1.0 / (1.0 + cv)
        consistency_score = max(0.0, 1.0 - (std_deviation / mean_interval))
    else:
        avg_beat_interval = 0.0
        std_deviation = 0.0
        mean_interval = 0.0
        regularity = 0.0
        consistency_score = 0.0

    # Create result dictionary
    return {
        "success": True,
        "tempo": float(tempo),
        "beat_count": len(beat_times),
        "beat_timestamps": beat_times.tolist(),
        "onset_count": len(onset_times),
        "onset_timestamps": onset_times.tolist(),
        "avg_beat_interval": avg_beat_interval,
        "duration": float(len(y) / sample_rate),
        "sample_rate": int(sample_rate),
        "consistency": {
            "consistency": consistency_score,
            "std_deviation": std_deviation,
            "regularity": regularity,
            "mean_interval": mean_interval
        }
    }


# if __name__ == "__main__":
#     # Test the beat detection function on a drums file
#     drums_file = "/Users/sam/sam-repos/stems/demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/drums.wav"
    
#     if not os.path.exists(drums_file):
#         print(f"Test drums file not found: {drums_file}")
#         sys.exit(1)
    
#     print(f"Loading audio file: {drums_file}")
    
#     # Load audio file using librosa
#     y, sr = librosa.load(drums_file, sr=None)
    
#     print(f"Loaded audio: {len(y)} samples, {sr} Hz, {len(y)/sr:.2f}s duration")
    
#     # Convert to list for our function (which expects a list)
#     audio_data = y.tolist()
    
#     print("Running beat detection...")
#     result = detect_beats(audio_data, sr)
    
#     if result["success"]:
#         print(f"Beat detection successful!")
#         print(f"Tempo: {result['tempo']:.1f} BPM")
#         print(f"Beat count: {result['beat_count']}")
#         print(f"Onset count: {result['onset_count']}")
#         print(f"Average beat interval: {result['avg_beat_interval']:.3f}s")
#         print(f"Duration: {result['duration']:.2f}s")
#         if result['consistency']:
#             consistency = result['consistency']
#             print(f"Consistency: {consistency['consistency']:.3f}")
#             print(f"Regularity: {consistency['regularity']:.3f}")
#             print(f"Standard deviation: {consistency['std_deviation']:.3f}s")
#     else:
#         print(f"Beat detection failed: {result.get('error', 'Unknown error')}")
    
#     print("Done.")
