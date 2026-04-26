# Beat Bar Flickering Analysis and Fix Plan

## Problem Description

The app refresh rate is 100Hz, but during playback the beat bars appear to flicker. The issue is with Canvas painting performance during real-time updates.

## Root Cause Analysis

The beat bars are flickering because:

1. **High Frequency Canvas Repaints**: The Canvas is being repainted at 100Hz (10ms intervals) due to `onCurrentPositionChanged: requestPaint()` triggers from the Rust backend's position updates.

2. **Inefficient Beat Rendering**: Each Canvas repaint redraws ALL beat markers from scratch in a loop, even when most are off-screen or haven't changed.

3. **Position Update Threshold**: The Rust backend updates position whenever the change is > 0.05 seconds (line 195 in multi_bridge.rs), which at 100Hz refresh rate causes frequent repaints.

4. **Complex Coordinate Calculations**: Beat marker positions are recalculated every frame with complex timeline scaling and scroll offset math.

5. **Lack of Canvas Optimization**: The Canvas lacks performance optimizations like:
   - `renderTarget: Canvas.Image` for hardware acceleration
   - `renderStrategy: Canvas.Cooperative` for better threading
   - Dirty region repainting

## Performance Optimization Plan

### 1. Add Canvas Performance Properties

```qml
Canvas {
    id: waveformCanvas
    renderTarget: Canvas.Image        // Hardware acceleration
    renderStrategy: Canvas.Cooperative // Better threading
    antialiasing: false              // Faster for simple lines
    // ... existing properties
}
```

### 2. Implement Smart Repaint Logic

- Track last drawn beat positions to avoid redundant calculations
- Only repaint beats when visible viewport changes significantly
- Use dirty region tracking instead of full canvas clears
- Separate beat marker updates from position cursor updates

### 3. Optimize Beat Marker Rendering

Current inefficient approach (lines 566-588 in multi_player.qml):
```javascript
for (var b = 0; b < beatData.length; b++) {
    var beatTime = beatData[b]
    // Complex calculations for each beat
    ctx.beginPath()  // Individual path for each beat
    ctx.moveTo(beatX, 0)
    ctx.lineTo(beatX, height)
    ctx.stroke()     // Individual stroke for each beat
}
```

Optimized approach:
```javascript
// Pre-calculate visible beat range
var visibleBeats = getVisibleBeats(scrollOffset, width, zoomFactor)

// Single path for all visible beats
ctx.beginPath()
for (var beat of visibleBeats) {
    ctx.moveTo(beat.x, 0)
    ctx.lineTo(beat.x, height)
}
ctx.stroke() // Single stroke operation
```

### 4. Reduce Update Frequency

- Decouple beat marker updates from position updates
- Only update beats when zoom level or scroll position changes significantly
- Consider lower refresh rate for beat markers vs. playback cursor
- Cache beat positions until zoom/scroll changes

### 5. Implementation Strategy

1. **Phase 1**: Add Canvas performance properties
   - Set `renderTarget: Canvas.Image`
   - Set `renderStrategy: Canvas.Cooperative` 
   - Test performance impact

2. **Phase 2**: Optimize beat rendering loop
   - Implement viewport culling for off-screen beats
   - Use single stroke operation for all visible beats
   - Pre-calculate beat positions

3. **Phase 3**: Smart invalidation
   - Track when beat positions actually need recalculation
   - Separate beat rendering from cursor rendering
   - Add caching mechanism for beat positions

## Expected Results

This should eliminate the flickering by:
- Reducing unnecessary canvas repaints
- Making individual repaints more efficient
- Leveraging hardware acceleration where available
- Minimizing JavaScript execution during high-frequency updates

## Files to Modify

- `qml/multi_player.qml` - Canvas rendering optimization (lines 275-644)
- Potentially `src/player/multi_bridge.rs` - Position update frequency tuning