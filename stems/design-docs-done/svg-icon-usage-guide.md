# SVG Icon System Migration to Gyroflow Pattern

## Overview

This guide outlines migrating your current SVG icon system to follow the proven Gyroflow pattern, which provides better maintainability, consistency, and developer experience.

## Current vs Target Architecture

### Current System
- **XML-based QRC**: `resources.qrc` with `/icons` prefix
- **Direct source URLs**: `"qrc:/icons/resources/icons/volume.svg"`
- **Custom components**: `VolumeIcon.qml` with hardcoded logic
- **File structure**: `resources/icons/*.svg` (flat)

### Target Gyroflow Pattern
- **Rust qrc! macro**: Resources embedded at compile time with `qmetaobject::qrc`
- **Centralized icon mapping**: String-based `iconName` properties
- **URL construction**: `"qrc:/resources/icons/svg/" + iconName + ".svg"`
- **Qt integration**: Leverage Button's built-in `icon.source` property
- **File structure**: `resources/icons/svg/*.svg` (organized)

## Migration Plan

### Phase 1: Restructure Icon Files

#### Current Structure
```
resources/
├── icons/
│   ├── volume.svg
│   ├── volume-1.svg
│   ├── volume-2.svg
│   ├── volume-off.svg
│   └── volume-x.svg
```

#### Target Structure
```
resources/
├── icons/
│   └── svg/
│       ├── volume.svg
│       ├── volume-1.svg
│       ├── volume-2.svg
│       ├── volume-off.svg
│       ├── volume-x.svg
│       ├── play.svg
│       ├── pause.svg
│       ├── stop.svg
│       ├── folder.svg
│       └── settings.svg
```

**Actions Required:**
1. Create `resources/icons/svg/` directory
2. Move existing SVG files to new location
3. Add missing music player icons (play, pause, stop, folder, etc.)

### Phase 2: Migrate to Rust qrc! Macro (Gyroflow Pattern)

#### Current resources.qrc (XML-based)
```xml
<!DOCTYPE RCC>
<RCC version="1.0">
    <qresource prefix="/icons">
        <file>resources/icons/volume.svg</file>
        <file>resources/icons/volume-1.svg</file>
        <file>resources/icons/volume-2.svg</file>
        <file>resources/icons/volume-off.svg</file>
        <file>resources/icons/volume-x.svg</file>
    </qresource>
</RCC>
```

#### Target: Rust qrc! Macro (Gyroflow Pattern)

**Create `src/resources.rs`:**
```rust
use qmetaobject::qrc;

qrc!(pub rsrc,
    "/" {
        "resources/icons/svg/volume.svg",
        "resources/icons/svg/volume-1.svg", 
        "resources/icons/svg/volume-2.svg",
        "resources/icons/svg/volume-off.svg",
        "resources/icons/svg/volume-x.svg",
        "resources/icons/svg/play.svg",
        "resources/icons/svg/pause.svg",
        "resources/icons/svg/stop.svg",
        "resources/icons/svg/folder.svg",
        "resources/icons/svg/settings.svg",
    }
);
```

**Update your main.rs to initialize resources:**
```rust
// In your main application initialization (multi_main.rs or equivalent)
mod resources;

fn main() {
    // Initialize Qt resources
    crate::resources::rsrc();
    
    // ... rest of your application setup
}
```

**Update Cargo.toml dependencies:**
```toml
[dependencies]
# Add qmetaobject for the qrc! macro
qmetaobject = "0.2"
# ... your existing dependencies
```

### Phase 3: Enhance Existing Button Component (Gyroflow Pattern)

**Update `qml/components/Button.qml` or create if it doesn't exist:**
```qml
import QtQuick 2.15
import QtQuick.Controls 2.15 as QQC
import ".."

QQC.Button {
    id: root
    
    // Gyroflow-style iconName property
    property string iconName: ""
    property bool accent: false
    property color textColor: accent ? Styles.transportIconColor : Styles.primaryTextColor
    property bool fadeWhenDisabled: true
    
    // Icon configuration (Gyroflow pattern)
    icon.name: iconName || ""
    icon.source: iconName ? "qrc:/resources/icons/svg/" + iconName + ".svg" : ""
    icon.width: 24
    icon.height: 24
    icon.color: textColor
    
    // Sizing and behavior
    height: 35
    leftPadding: 15
    rightPadding: 15
    topPadding: 8
    bottomPadding: 8
    font.pixelSize: 14
    hoverEnabled: enabled
    
    Component.onCompleted: {
        if (contentItem.color) {
            contentItem.color = Qt.binding(() => root.textColor)
            icon.color = Qt.binding(() => root.textColor)
            if (fadeWhenDisabled) {
                contentItem.opacity = Qt.binding(() => !root.enabled ? 0.75 : 1.0)
            }
        }
    }
    
    background: Rectangle {
        color: root.accent ? 
            (root.hovered || root.activeFocus ? Qt.lighter(Styles.transportButtonColor, 1.1) : Styles.transportButtonColor) :
            (root.hovered || root.activeFocus ? Qt.lighter(Styles.buttonInactiveColor, 1.2) : Styles.buttonInactiveColor)
        opacity: (!parent.enabled && fadeWhenDisabled ? 0.75 : root.down ? 0.75 : 1.0)
        radius: 6
        anchors.fill: parent
        
        Behavior on opacity { 
            NumberAnimation { duration: 100 }
        }
    }
    
    // Scale animation on press (Gyroflow style)
    scale: root.down ? 0.970 : 1.0
    Behavior on scale {
        NumberAnimation { duration: 100 }
    }
    
    font.capitalization: Font.Normal
    
    // Tooltip support
    property alias tooltip: tt.text
    ToolTip { 
        id: tt
        visible: text.length > 0 && root.hovered
    }
    
    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Enter || event.key === Qt.Key_Return) {
            root.clicked()
        }
    }
}
```

### Phase 4: Update VolumeIcon Component

**Updated `qml/components/VolumeIcon.qml`**
```qml
import QtQuick 2.15
import QtQuick.Controls 2.15
import ".."

Item {
    id: root
    
    property real volume: 1.0
    property bool muted: false
    property color iconColor: Styles.primaryTextColor
    property real iconSize: 16
    
    width: iconSize
    height: iconSize
    
    // Gyroflow pattern: iconName property
    readonly property string iconName: {
        if (muted) return "volume-x"
        if (volume <= 0.0) return "volume-off"
        if (volume <= 0.5) return "volume"
        if (volume <= 1.0) return "volume-1"
        return "volume-2"
    }
    
    Image {
        id: iconImage
        anchors.fill: parent
        source: "qrc:/resources/icons/svg/" + root.iconName + ".svg"
        sourceSize.width: root.iconSize
        sourceSize.height: root.iconSize
        fillMode: Image.PreserveAspectFit
        visible: false
    }
    
    ColorOverlay {
        anchors.fill: iconImage
        source: iconImage
        color: root.iconColor
        antialiasing: true
    }
}
```

### Phase 5: Update Transport Controls

**Replace emoji icons in `multi_player.qml`:**

#### Current (Line ~1203)
```qml
Text {
    text: "🔊"
    font.pointSize: 14
    anchors.verticalCenter: parent.verticalCenter
    width: 20
    horizontalAlignment: Text.AlignHCenter
}
```

#### Target
```qml
VolumeIcon {
    volume: multiBridge ? multiBridge.master_volume : 1.0
    muted: false
    iconColor: Styles.primaryTextColor
    iconSize: 14
    anchors.verticalCenter: parent.verticalCenter
    width: 20
}
```

#### Current Play Button (Line ~1054)
```qml
contentItem: Text {
    text: (multiBridge && multiBridge.is_playing) ? "⏸" : "▶"
    color: playButton.enabled ? Styles.transportIconColor : Styles.transportIconDisabledColor
    font.pointSize: 16
    horizontalAlignment: Text.AlignHCenter
    verticalAlignment: Text.AlignVCenter
}
```

#### Target (Using Enhanced Button)
```qml
Button {
    iconName: (multiBridge && multiBridge.is_playing) ? "pause" : "play"
    text: "" // Icon-only button
    textColor: playButton.enabled ? Styles.transportIconColor : Styles.transportIconDisabledColor
    accent: true
    anchors.fill: parent
    
    onClicked: {
        if (!multiBridge) return
        if (multiBridge.is_playing) {
            multiBridge.pause()
        } else {
            multiBridge.play()
        }
    }
}
```

## Required Icon Files

To complete the migration, you'll need these additional SVG files in `resources/icons/svg/`:

### Essential Music Player Icons
- `play.svg` - Play button
- `pause.svg` - Pause button  
- `stop.svg` - Stop button
- `folder.svg` - File/folder browser
- `settings.svg` - Settings/preferences

### Optional Enhancement Icons
- `next.svg` - Next track
- `previous.svg` - Previous track
- `loop.svg` - Loop/repeat
- `shuffle.svg` - Shuffle/random
- `mute.svg` - Alternative mute icon

## Icon Sources

You can source these icons from:
1. **Lucide Icons** (your current volume icons): https://lucide.dev/
2. **Heroicons**: https://heroicons.com/
3. **Feather Icons**: https://feathericons.com/
4. **Copy from Gyroflow**: Their icons are available in their repo

## Benefits of Migration

### Developer Experience
- **Consistent API**: All components use `iconName` string properties
- **Discoverable**: Easy to see available icons and add new ones
- **Type-safe**: String-based icon names can be validated/documented

### Maintainability  
- **Centralized**: Icon logic concentrated in IconButton component
- **Extensible**: Easy to add new icons without touching component code
- **Consistent**: All icons follow same sizing and coloring patterns

### Performance
- **Qt integration**: Leverage Qt's optimized icon rendering
- **Resource efficiency**: Proper resource bundling and caching
- **Scalable**: Vector icons look crisp at any size

## Migration Checklist

### Phase 1: File Structure
- [ ] Create `resources/icons/svg/` directory
- [ ] Move existing SVG files to new location
- [ ] Add missing music player icons (play, pause, stop, folder, settings)

### Phase 2: Rust Resources 
- [ ] Add `qmetaobject` dependency to Cargo.toml
- [ ] Create `src/resources.rs` with qrc! macro
- [ ] Initialize resources in main.rs with `crate::resources::rsrc()`
- [ ] Remove old `resources.qrc` file
- [ ] Update build.rs if needed to handle new resource system

### Phase 3: Component Updates
- [ ] Enhance `Button.qml` component with Gyroflow iconName pattern
- [ ] Update `VolumeIcon.qml` to use iconName pattern
- [ ] Replace emoji icons in transport controls with Button components
- [ ] Test icon rendering and coloring

### Phase 4: Integration
- [ ] Update any hardcoded icon paths in other components
- [ ] Verify all icons load correctly with new qrc:/ URLs
- [ ] Test build and runtime resource loading
- [ ] Document iconName conventions for future development