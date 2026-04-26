# Mute Button Inconsistency Bug Analysis

## Problem Statement

The mute button on waveforms is currently buggy and does not consistently toggle the mute state. Users report that clicking the mute button sometimes doesn't change the state properly.

## Desired Behavior

- The per-waveform-mute state should be purely derived: `waveform-volume == 0`
- Mute button highlighting should be controlled by `waveform-volume == 0` 
- Clicking mute when `waveform volume != 0` should set it to `0`
- Clicking mute when `waveform volume == 0` should set it to `1.0`
- Waveform volume state should be backed by an `AtomicF32` to minimize UI latency

## Current Implementation Analysis

### Volume State Management
✅ **CORRECT**: Volume is already backed by `AtomicF32`
- `src/audio/multi_engine.rs:58`: `file_volumes: Vec<AtomicF32>`
- Volume operations use `Ordering::SeqCst` for predictable cross-thread visibility

### Mute State Derivation  
✅ **CORRECT**: Mute state is derived from volume
- `src/audio/multi_engine.rs:409`: `let file_mutes = file_volumes.iter().map(|&vol| vol <= 0.0).collect();`
- Uses `<= 0.0` threshold, which is appropriate

### Toggle Implementation
⚠️ **POTENTIAL ISSUE**: Current toggle logic in `multi_engine.rs:356-369`
```rust
fn toggle_mute(inner: &MultiAudioEngineInner, file_idx: usize) {
    let current_volume = inner.file_volumes[file_idx].load(Ordering::SeqCst);
    
    if current_volume > 0.0 {
        // Currently audible, mute it (set to 0.0)
        inner.file_volumes[file_idx].store(0.0, Ordering::SeqCst);
    } else {
        // Currently muted, unmute it (set to 1.0)
        inner.file_volumes[file_idx].store(1.0, Ordering::SeqCst);
    }
}
```

### UI State Synchronization
⚠️ **MAIN ISSUE**: Race condition between UI and backend
- QML: `checked: multiBridge ? multiBridge.get_file_mute(stemRect.index) : false`
- `get_file_mute` calls `engine.get_state()` which rebuilds the entire state including derived mute flags
- UI updates are triggered by `playback_settings_changed` signal
- **Gap**: No immediate UI update after mute toggle

## Root Cause Analysis

The inconsistency occurs due to **asynchronous state updates**:

1. User clicks mute button → `toggle_mute()` called
2. Volume is updated atomically in Rust backend  
3. `playback_settings_changed` signal is emitted
4. **Race condition**: QML may call `get_file_mute()` before signal propagates
5. UI shows stale state until next update cycle

### Code Flow Issues

1. **Signal Timing**: `multi_bridge.rs:295` emits `playback_settings_changed()` after `toggle_mute`, but QML binding may not update immediately

2. **State Reconstruction**: `get_file_mute()` rebuilds mute state from current volume, but there's a window where UI hasn't refreshed the binding

3. **No Immediate Feedback**: Unlike volume sliders that may have direct property bindings, mute state requires a full state query

## Proposed Solution

### Option A: Immediate UI Update (Recommended)
Modify `toggle_mute` in `multi_bridge.rs` to force immediate UI refresh:

```rust
fn toggle_mute(&mut self, file_index: i32) {
    // ... existing toggle logic ...
    
    // Force immediate signal emission and UI update
    self.playback_settings_changed();
    
    // Optional: Force QML to re-evaluate bindings
    // by making the mute check synchronous
}
```

### Option B: Direct Property Binding
Create a dedicated mute property that updates synchronously:
- Add `mute_states: Vec<AtomicBool>` alongside `file_volumes`
- Update both atomically in `toggle_mute`
- Expose direct mute property to QML instead of derived state

### Option C: Optimistic UI Updates
Update QML button state immediately, then reconcile with backend:
```qml
onClicked: {
    // Optimistic update
    checked = !checked
    multiBridge.toggle_mute(stemRect.index)
}
```

## Recommendation

**Option A (Immediate UI Update)** is the best approach because:
- Minimal code changes required
- Maintains single source of truth (volume-based mute state)
- Fixes the race condition directly
- Preserves desired behavior model
- No risk of UI/backend state divergence

The root issue is not the logic but the timing of UI updates. The current implementation correctly derives mute from volume state and uses `AtomicF32` as requested, but needs better UI synchronization.
