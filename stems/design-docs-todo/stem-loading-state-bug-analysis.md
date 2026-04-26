# Stem Loading State Bug Analysis & Solution

## Problem Statement

When a set of stems has already been fully loaded, opening a new set of stems after clicking "Load Stems" and choosing a directory does not immediately show a "loading" state. The previously loaded stems waveform & beats remain visible during the loading process, which creates a confusing user experience where it appears nothing is happening.

## Current Loading Flow Analysis

### UI State Management (`multi_player.qml`)

The loading state is managed via several properties in the `fileLoader` component:

```qml
property bool isLoading: false
property real loadingProgress: 0.0
```

These are updated in the `stateUpdateTimer` (lines 138-140):
```qml
fileLoader.isLoading = multiBridge.is_loading
fileLoader.loadingProgress = multiBridge.loading_progress
```

### Backend State Management (`multi_bridge.rs`)

The loading state is controlled in the `load_files` method (lines 372-376):

```rust
// Set loading state
self.is_loading = true;
self.loading_progress = 0.0;
self.is_loading_changed();
self.loading_progress_changed();
```

And cleared after successful loading (lines 409-411):
```rust
// Clear loading state
self.is_loading = false;
self.is_loading_changed();
```

## Root Cause Analysis

The bug occurs because:

1. **Loading state is set correctly in the backend** - The Rust `MultiBridge` properly sets `is_loading = true` at the start of `load_files()`

2. **UI state updates are delayed** - The QML UI only updates the loading state via the `stateUpdateTimer` which runs every 50ms, creating a delay of up to 50ms before the loading state becomes visible

3. **No immediate state clearing** - When new files are loaded, there's no immediate clearing of the previous waveform data, so old waveforms remain visible until new ones are generated

4. **Waveform persistence** - The waveform Canvas elements retain their `peakData` and `beatData` from the previous load, showing stale visualization during loading

## Impact Assessment

- **User Experience**: Creates confusion about whether the load action was successful
- **Visual Feedback**: Users may think the application is frozen or unresponsive
- **Workflow Disruption**: Users might click "Load Stems" multiple times thinking it didn't work

## Proposed Solutions

### Solution 1: Immediate Loading State Update (Recommended)

**Approach**: Update the loading state immediately when the folder dialog is accepted, before calling the backend.

**Implementation**:

In `multi_player.qml`, modify the `folderDialog.onAccepted` handler:

```qml
onAccepted: {
    let folderPath = selectedFolder.toString()
    
    console.log("Selected folder:", folderPath)
    
    // IMMEDIATELY set loading state before backend call
    fileLoader.isLoading = true
    fileLoader.loadingProgress = 0.0
    
    // Clear previous waveform data
    for (let i = 0; i < waveformRepeater.count; i++) {
        let waveformCanvas = waveformRepeater.itemAt(i).children[0].children[0] // Navigate to Canvas
        if (waveformCanvas) {
            waveformCanvas.updatePeakData([])  // Clear peaks
            waveformCanvas.updateBeatData([], 0.0)  // Clear beats
        }
    }
    
    // ... rest of existing code
}
```

**Pros**:
- Immediate visual feedback
- Simple to implement
- No backend changes required
- Clears stale data immediately

**Cons**:
- Slight code duplication (loading state set in both UI and backend)
- Need to navigate QML object hierarchy

### Solution 2: Synchronous State Signal (Alternative)

**Approach**: Add a new signal that fires immediately when loading starts, separate from the property-based state updates.

**Implementation**:

In `multi_bridge.rs`, add a new signal:
```rust
pub loading_started: qt_signal!(),
```

Fire it immediately in `load_files()`:
```rust
fn load_files(&mut self, paths: QVariantList) -> bool {
    // ... validation code ...
    
    self.loading_started();  // Fire immediately
    
    // Set loading state
    self.is_loading = true;
    // ... rest of method
}
```

In `multi_player.qml`, add a connection:
```qml
Connections {
    target: multiBridge
    function onLoading_started() {
        fileLoader.isLoading = true
        fileLoader.loadingProgress = 0.0
        // Clear waveforms...
    }
}
```

**Pros**:
- Clean separation of immediate vs. periodic updates
- Backend controls the timing
- More extensible for future loading events

**Cons**:
- Requires backend changes
- Additional signal to maintain

### Solution 3: Pre-clear on Load Button Click

**Approach**: Clear the loading state and waveforms when the "Load Stems" button is clicked, before the folder dialog opens.

**Implementation**:

Modify the `loadStemsButton.onClicked` handler:

```qml
onClicked: {
    // Pre-clear state before dialog
    fileLoader.isLoading = true
    fileLoader.loadingProgress = 0.0
    // Clear waveforms...
    
    fileLoader.openFolderDialog()
}
```

**Pros**:
- Even more immediate feedback
- Very simple implementation

**Cons**:
- Shows loading state even if user cancels dialog
- Less accurate representation of actual loading process

## Recommendation

**Implement Solution 1** for the following reasons:

1. **Immediate visual feedback** without backend complexity
2. **Addresses both aspects** of the bug (loading state + stale waveforms)
3. **Low risk** - doesn't change core loading logic
4. **Easy to test** - visible immediately in UI

## Implementation Steps

1. Modify `folderDialog.onAccepted` to set loading state immediately
2. Add waveform clearing logic in the same handler
3. Test with various stem sets to ensure proper clearing
4. Verify that the 50ms timer still properly updates the states from backend

## Testing Strategy

1. **Load initial stems** - verify normal loading works
2. **Load different stems** - verify immediate loading state appears
3. **Cancel during load** - verify loading state clears properly  
4. **Multiple rapid loads** - verify no race conditions
5. **Empty folder selection** - verify error handling still works

This solution provides immediate user feedback while maintaining the existing robust loading architecture.