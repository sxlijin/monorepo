import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Dialogs
import StemsUI 1.0
import "components"
import "."
import "storybook"

// Multi-file audio player interface for Phase 2
// Provides synchronized playback of multiple audio files with individual controls
ApplicationWindow {
    id: mainWindow
    objectName: "MultiPlayer"
    visible: true
    width: 1600
    height: 1080
    minimumWidth: 1200
    minimumHeight: 700
    title: "Stems Player - Multi-File Mode "
    
    // Enable dark title bar on macOS
    flags: Qt.Window | Qt.WindowTitleHint | Qt.WindowSystemMenuHint | Qt.WindowMinMaxButtonsHint | Qt.WindowCloseButtonHint | Qt.FramelessWindowHint
    
    color: Styles.windowBackgroundColor
    palette: Styles.darkPalette
    
    // Custom title bar for frameless window
    MacOSTitleBar {
        id: customTitleBar
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        targetWindow: mainWindow
        title: "Stems Player - Multi-File Mode"
        z: 1000  // Ensure title bar stays on top
    }
    
    // Waveform sizing constants
    readonly property int windowTopPadding: 15
    readonly property int windowLeftPadding: 15
    readonly property int windowRightPadding: 15
    readonly property int windowBottomPadding: 15
    readonly property int minWaveformHeight: 150   // Minimum height per waveform
    readonly property int waveformSpacing: 20
    readonly property int stemCount: 5
    property real waveformTimeWidthSecs: 5.0  // seconds of audio to show in waveform viewport (modifiable for zoom)
    readonly property int waveformControlsWidth: 120  // fixed width for controls panel
    readonly property int transportControlsHeight: 100  // fixed height for transport controls section
    
    // Control panel layout constants (preserve exact positioning)
    readonly property int controlsPanelMargins: 4  // Inner margins of controls panel
    readonly property int controlsRowSpacing: 8  // Spacing between volume controls and buttons
    readonly property int volumeSliderLeftOffset: 4  // Offset volume slider from left
    readonly property int stemButtonsRightMargin: 18  // Right margin for per-stem solo/mute buttons
    readonly property int masterButtonsRightMargin: stemButtonsRightMargin - 6  // Derived from stem margin (14 - 6 = 8)
    
    // Adaptive height calculations
    readonly property bool useAdaptiveHeight: {
        if (!mainContentScroll) return false
        let minTotalHeight = (stemCount * minWaveformHeight) + ((stemCount - 1) * waveformSpacing)
        return mainContentScroll.height >= minTotalHeight
    }
    readonly property int adaptiveWaveformHeight: {
        if (!useAdaptiveHeight) return minWaveformHeight  // Use minimum height when not adaptive
        if (!mainContentScroll) return minWaveformHeight
        let availableHeight = mainContentScroll.height - ((stemCount - 1) * waveformSpacing)
        return Math.max(minWaveformHeight, Math.floor(availableHeight / stemCount))
    }
    readonly property int volumeSliderHeight: adaptiveWaveformHeight - 60  // slider height derived from adaptive height
    
    // Calculated heights for consistent sizing
    readonly property int totalWaveformHeight: 
        (stemCount * adaptiveWaveformHeight) + 
        ((stemCount - 1) * waveformSpacing)
        
    // Transport controls calculated dimensions
    readonly property int transportSectionMargins: 10
    readonly property int transportSectionHeight: transportControlsHeight - (transportSectionMargins * 2)  // Account for GroupBox margins
    readonly property int transportRowHeight: 70  // Current fixed height for the main Row
    readonly property int transportColumnHeight: transportRowHeight  // Columns should match Row height
    readonly property int transportRowSpacing: 20  // Spacing between columns in transport Row
    readonly property int transportColumnWidth: (width - windowLeftPadding - windowRightPadding - (transportRowSpacing * 2)) / 3  // Account for 2 gaps between 3 columns
    
    // Track whether the Rust multi-bridge is successfully connected
    property bool playerInitialized: false
    
    // Helper function to find first .wav file in a directory
    function findFirstWavFile(directory) {
        try {
            if (typeof multiBridge !== 'undefined') {
                var result = multiBridge.find_first_wav_in_directory(directory)
                if (result && result.length > 0) {
                    return result
                }
            }
            return null
        } catch (e) {
            console.log("Error finding .wav files:", e)
            return null
        }
    }
    
    // Application initialization - sets up multi-audio system
    Component.onCompleted: {
        console.log("Multi-file UI loaded")
        
        // Request dark title bar on macOS
        if (Qt.platform.os === "osx") {
            // This will make the title bar dark on macOS when the app content is dark
            mainWindow.flags = mainWindow.flags | Qt.WindowFullscreenButtonHint
        }
        
        if (typeof multiBridge === 'undefined') {
            console.log("Multi bridge not available")
            return
        }
        
        console.log("Multi bridge available")
        playerInitialized = true
        
        // Audio device initialization removed - using default device

        // Try to auto-load the default reference track first
        var defaultFile = "/Users/sam/Music/Alannah Myles - Black Velvet.wav"
        console.log("Default audio file:", defaultFile)
        var loadSucceeded = multiBridge.load_single_file(defaultFile)

        if (!loadSucceeded) {
            console.log("Default file load failed, searching music directory")

            // Auto-load first .wav file found in ~/Music using single file loading
            var musicDir = "/Users/sam/Music"
            console.log("Music directory:", musicDir)

            var firstWavFile = findFirstWavFile(musicDir)
            if (firstWavFile) {
                console.log("Attempting to auto-load:", firstWavFile)
                loadSucceeded = multiBridge.load_single_file(firstWavFile)
                if (loadSucceeded) {
                    console.log("Successfully auto-loaded:", firstWavFile)
                } else {
                    console.log("Failed to auto-load:", firstWavFile)
                }
            } else {
                console.log("No .wav files found in ~/Music directory")
            }
        }

        if (!loadSucceeded) {
            // Fallback to hardcoded test file for development if everything else fails
            var fallbackFile = "/Users/sam/sam-repos/stems/demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/vocals.wav"
            console.log("Using fallback file:", fallbackFile)
            loadSucceeded = multiBridge.load_single_file(fallbackFile)
            if (loadSucceeded) {
                console.log("Successfully loaded fallback file")
            } else {
                console.log("Failed to load fallback file")
            }
        }
    }
    // Error handling connection to Rust multi-bridge
    // Displays error messages from multi-audio engine in popup dialog
    Connections {
        id: multiBridgeErrorHandler
        target: typeof multiBridge !== 'undefined' ? multiBridge : null
        
        // Show error popup when multi-audio engine reports problems
        function onError_occurred(message) {
            errorDialog.text = message
            errorDialog.open()
        }
    }
    
    // Loading state change handler - metadata bindings refresh automatically
    Connections {
        id: loadingStateHandler
        target: typeof multiBridge !== 'undefined' ? multiBridge : null
        
        function onLoading_state_changed() {
            console.log("QML MAIN DEBUG: Loading state changed signal received")
            if (multiBridge) {
                var loadingState = multiBridge.get_loading_state()
                console.log("QML MAIN DEBUG: Current loading state:", JSON.stringify(loadingState))
            } else {
                console.log("QML MAIN DEBUG: multiBridge is null/undefined")
            }
            // Metadata bindings refresh automatically when loading state changes
        }
    }
    
    // Main layout with transport controls pinned to bottom
    Item {
        anchors.fill: parent
        anchors.topMargin: customTitleBar.height + windowTopPadding   // Account for custom title bar
        anchors.leftMargin: windowLeftPadding
        anchors.rightMargin: windowRightPadding
        anchors.bottomMargin: windowBottomPadding
        
        // Scrollable content area (everything except transport controls)
        ScrollView {
            id: mainContentScroll
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: transportControlsContainer.top
            anchors.bottomMargin: 8
            
            // Custom scrollbar with adaptive behavior
            ScrollBar.vertical: ScrollBar {
                id: adaptiveScrollBar
                policy: mainWindow.useAdaptiveHeight ? ScrollBar.AlwaysOff : ScrollBar.AsNeeded
                
                background: Rectangle {
                    color: Styles.scrollBarBackgroundColor
                    radius: 6
                    width: 12
                }
                
                contentItem: Rectangle {
                    color: adaptiveScrollBar.pressed ? Styles.scrollBarHandlePressedColor :
                           adaptiveScrollBar.hovered ? Styles.scrollBarHandleHoverColor :
                           Styles.scrollBarHandleColor
                    radius: 6
                    width: 12
                    
                    Behavior on color {
                        ColorAnimation { duration: 150 }
                    }
                }
            }
            
            // Waveform Visualization
            GroupBox {
                id: waveformVisualizationSection
                width: mainContentScroll.availableWidth
                height: mainWindow.totalWaveformHeight

                background: Rectangle {
                    color: Styles.windowBackgroundColor
                    radius: 8
                }

                DebugBorderRectangle {}

                Column {
                    width: parent.width
                    height: parent.height
                    spacing: waveformSpacing

                    // Waveform display for each stem
                    Repeater {
                        id: waveformRepeater
                        model: mainWindow.stemCount

                        Rectangle {
                            id: stemRect
                            width: parent.width
                            height: mainWindow.adaptiveWaveformHeight
                            color: Styles.stemRectBackgroundColor
                            radius: 4
                            
                            required property int index
                            property var stemColors: [
                                Styles.stemVocalsColor,
                                Styles.stemBassColor,
                                Styles.stemDrumsColor,
                                Styles.stemDrumsColor,
                                Styles.stemOtherColor
                            ]
                            property var stemColorsDark: [
                                Styles.stemVocalsColorDark,
                                Styles.stemBassColorDark,
                                Styles.stemDrumsColorDark,
                                Styles.stemDrumsColorDark,
                                Styles.stemOtherColorDark
                            ]
                            property var stemColorsBright: [
                                Styles.stemVocalsColorBright,
                                Styles.stemBassColorBright,
                                Styles.stemDrumsColorBright,
                                Styles.stemDrumsColorBright,
                                Styles.stemOtherColorBright
                            ]
                            property string currentStemColor: stemColors[index]
                            property string currentStemColorDark: stemColorsDark[index]
                            property string currentStemColorBright: stemColorsBright[index]
                            
                            // Stem label positioned above waveform with higher z-index
                            Rectangle {
                                id: stemLabelBackground
                                anchors.left: parent.left
                                anchors.top: parent.top
                                anchors.margins: 5
                                z: 10
                                width: stemLabel.contentWidth + 8
                                height: stemLabel.contentHeight + 4
                                color: Styles.stemLabelBackgroundColor || "#2a2a2a"
                                radius: 3
                                
                                Text {
                                    id: stemLabel
                                    anchors.centerIn: parent
                                    text: {
                                        if (!multiBridge) return "Loading..."
                                        // Force reactive update when file count changes
                                        multiBridge.file_count
                                        if (multiBridge.waveform_failed(stemRect.index)) {
                                            return "Failed to load waveform"
                                        }
                                        if (stemRect.index === 2) {
                                            return "drums-hi"
                                        } else if (stemRect.index === 3) {
                                            return "drums-lo"
                                        }
                                        return multiBridge.get_file_name(stemRect.index)
                                    }
                                    font.bold: true
                                    color: stemRect.currentStemColor
                                    font.pointSize: 10
                                }
                            }
                            
                            Row {
                                anchors.fill: parent
                                
                                // Waveform section (derived width)
                                Rectangle {
                                    id: waveformContainer
                                    width: parent.width - mainWindow.waveformControlsWidth
                                    height: parent.height
                                    color: Styles.waveformContainerBackgroundColor
                                    radius: 3
                                    
                                    // Native waveform rendering using QPainter
                                    WaveformComponent {
                                        DebugBorderRectangle {
                                            label: "Waveform Component"
                                            borderColor: "purple"
                                        }
                                        id: waveformView
                                        anchors.fill: parent
                                        anchors.margins: 2

                                        // Core properties from bridge
                                        stem_index: stemRect.index
                                        current_position: multiBridge ? multiBridge.current_position : 0
                                        duration: multiBridge ? multiBridge.duration : 0
                                        is_playing: multiBridge ? multiBridge.is_playing : false
                                        current_volume: multiBridge ? multiBridge.get_file_volume(stemRect.index) : 1.0
                                        zoom_level: mainWindow.waveformTimeWidthSecs

                                        // Visual properties
                                        waveform_color: stemRect.currentStemColor
                                        background_color: Styles.backgroundColor
                                        beat_color: Qt.rgba(1.0, 1.0, 1.0, 0.4)  // Semi-transparent white for better visibility
                                        cursor_color: "#ff0000"  // Bright red for clear visibility

                                        // Auto-update on property changes
                                        onCurrent_position_changed: request_update()
                                        onIs_playing_changed: request_update()
                                        onCurrent_volume_changed: request_update()
                                        onZoom_level_changed: request_update()
                                    }

                                    // Mouse interaction for seeking and zooming
                                    MouseArea {
                                        anchors.fill: waveformView
                                        acceptedButtons: Qt.LeftButton

                                        property bool wasPlayingBeforeDrag: false
                                        property bool isDragging: false
                                        property real initialMouseX: 0
                                        property real initialPosition: 0
                                        property real dragSensitivity: 0.02

                                        onPressed: function(mouse) {
                                            if (multiBridge && multiBridge.file_count > 0) {
                                                wasPlayingBeforeDrag = multiBridge.is_playing
                                                if (wasPlayingBeforeDrag) {
                                                    multiBridge.pause()
                                                }

                                                isDragging = true
                                                initialMouseX = mouse.x
                                                initialPosition = multiBridge.current_position
                                            }
                                        }

                                        onPositionChanged: function(mouse) {
                                            if (isDragging && multiBridge) {
                                                // Inverted delta: drag right rewinds, drag left fast-forwards
                                                var secondsPerPixel = mainWindow.waveformTimeWidthSecs / width
                                                var deltaSeconds = (initialMouseX - mouse.x) * secondsPerPixel
                                                var newPosition = Math.max(0, Math.min(multiBridge.duration, initialPosition + deltaSeconds))
                                                multiBridge.seek(newPosition)
                                            }
                                        }

                                        onReleased: function(mouse) {
                                            if (isDragging) {
                                                isDragging = false

                                                if (wasPlayingBeforeDrag && multiBridge) {
                                                    multiBridge.play()
                                                }
                                            }
                                        }

                                        onWheel: function(wheel) {
                                            if (wheel.modifiers & Qt.ShiftModifier) {
                                                // Shift + scroll: seek through audio
                                                var seekAmount = wheel.angleDelta.y > 0 ? 2.0 : -2.0
                                                if (multiBridge) {
                                                    var newPosition = Math.max(0, Math.min(multiBridge.duration, multiBridge.current_position + seekAmount))
                                                    multiBridge.seek(newPosition)
                                                }
                                                return
                                            }

                                            // Zoom requires Cmd (Qt.ControlModifier on macOS)
                                            if (!(wheel.modifiers & Qt.ControlModifier)) {
                                                return
                                            }

                                            // Trackpad two-finger drag: only react to vertical-dominant events
                                            if (Math.abs(wheel.angleDelta.y) <= Math.abs(wheel.angleDelta.x)) {
                                                return
                                            }

                                            var zoomFactor = wheel.angleDelta.y > 0 ? 0.95 : 1.05
                                            var currentZoom = mainWindow.waveformTimeWidthSecs
                                            var newZoom = Math.max(1.0, Math.min(30.0, currentZoom * zoomFactor))
                                            mainWindow.waveformTimeWidthSecs = newZoom
                                        }

                                        function calculateDragPosition(currentMouseX) {
                                            // Full width interaction - cursor at 20% shows current position

                                            // Map mouse position to time within the visible duration
                                            var visible_duration = mainWindow.waveformTimeWidthSecs
                                            var time_ratio = currentMouseX / width
                                            var visible_start_time = multiBridge.current_position - (visible_duration * 0.2)
                                            var target_time = visible_start_time + (time_ratio * visible_duration)

                                            return Math.max(0, Math.min(multiBridge.duration, target_time))
                                        }
                                    }

                                    // Connect to MultiBridge signal for when files change
                                    Connections {
                                        target: multiBridge
                                        function onFile_count_changed() {
                                            waveformView.request_update()
                                        }
                                        function onFile_states_changed() {
                                            waveformView.request_update()
                                        }
                                        function onLoading_state_changed() {
                                            waveformView.request_update()
                                        }
                                    }
                                }
                                
                                // Controls section (fixed width)
                                Rectangle {
                                    id: stemControlsPanel
                                    width: mainWindow.waveformControlsWidth
                                    height: parent.height
                                    color: Styles.controlsPanelBackgroundColor
                                    radius: 3
                                    
                                    Row {
                                        anchors.fill: parent
                                        anchors.margins: mainWindow.controlsPanelMargins
                                        spacing: mainWindow.controlsRowSpacing
                                        
                                        // Volume control section
                                        Column {
                                            width: 50
                                            anchors.verticalCenter: parent.verticalCenter
                                            anchors.left: parent.left
                                            anchors.leftMargin: mainWindow.volumeSliderLeftOffset
                                            spacing: 6
                                            
                                            // Volume percentage indicator
                                            Text {
                                                text: Math.round(volumeSlider.value * 100) + "%"
                                                font.pointSize: 8
                                                color: Styles.secondaryTextColor
                                                anchors.horizontalCenter: parent.horizontalCenter
                                            }
                                            
                                            // Volume slider with mouse wheel support
                                            MouseArea {
                                                width: 50
                                                height: mainWindow.volumeSliderHeight
                                                anchors.horizontalCenter: parent.horizontalCenter
                                                
                                                onWheel: function(wheel) {
                                                    if (multiBridge) {
                                                        var delta = wheel.angleDelta.y / 120.0  // Standard wheel delta
                                                        var volumeStep = 0.1  // 10% of full range per wheel notch
                                                        var newVolume = volumeSlider.value + (delta * volumeStep)
                                                        
                                                        // Clamp to slider range (0.0 to 2.0)
                                                        newVolume = Math.max(0.0, Math.min(2.0, newVolume))
                                                        
                                                        volumeSlider.value = newVolume
                                                        multiBridge.set_file_volume(stemRect.index, newVolume)
                                                    }
                                                }
                                                
                                                Rectangle {
                                                    id: volumeSlider
                                                    width: 5
                                                    height: mainWindow.volumeSliderHeight
                                                    anchors.horizontalCenter: parent.horizontalCenter
                                                    color: Styles.sliderTrackColor
                                                    radius: 2.5
                                                    
                                                    property bool updatingFromBridge: false
                                                    property real value: multiBridge ? Math.min(2.0, multiBridge.get_file_volume(stemRect.index)) : 1.0
                                                    property bool isHovered: volumeSliderMouseArea.containsMouse
                                                    
                                                    // Active fill indicator
                                                    Rectangle {
                                                        id: volumeProgress
                                                        anchors.bottom: parent.bottom
                                                        anchors.left: parent.left
                                                        anchors.right: parent.right
                                                        height: Math.min(1.0, volumeSlider.value / 2.0) * parent.height
                                                        color: stemRect.currentStemColor
                                                        opacity: 0.4
                                                        radius: 2.5
                                                    }
                                                    
                                                    // Position indicator circle
                                                    Rectangle {
                                                        visible: true
                                                        width: 12
                                                        height: 12
                                                        radius: 6
                                                        color: volumeSlider.isHovered ? stemRect.currentStemColorBright : stemRect.currentStemColorDark
                                                        x: parent.width / 2 - width / 2
                                                        y: (1 - Math.min(1.0, volumeSlider.value / 2.0)) * (parent.height - height)
                                                        
                                                        Behavior on color {
                                                            ColorAnimation { duration: 150 }
                                                        }
                                                    }
                                                    
                                                    
                                                    MouseArea {
                                                        id: volumeSliderMouseArea
                                                        anchors.centerIn: parent
                                                        width: 12  // Wider than 5px slider for easier interaction while keeping hover accurate
                                                        height: parent.height
                                                        hoverEnabled: true
                                                        
                                                        onClicked: function(mouse) {
                                                            if (!multiBridge) return
                                                            let normalized = Math.max(0, Math.min(1, 1 - (mouse.y / height)))
                                                            let newVolume = normalized * 2.0
                                                            multiBridge.set_file_volume(stemRect.index, newVolume)
                                                        }
                                                        
                                                        onPositionChanged: function(mouse) {
                                                            if (pressed && multiBridge) {
                                                                let normalized = Math.max(0, Math.min(1, 1 - (mouse.y / height)))
                                                                let newVolume = normalized * 2.0
                                                                multiBridge.set_file_volume(stemRect.index, newVolume)
                                                            }
                                                        }
                                                    }
                                                    
                                                    // React to playback settings changes
                                                    Connections {
                                                        target: multiBridge
                                                        function onPlayback_settings_changed() {
                                                            if (multiBridge) {
                                                                volumeSlider.updatingFromBridge = true
                                                                volumeSlider.value = Math.min(2.0, multiBridge.get_file_volume(stemRect.index))
                                                                volumeSlider.updatingFromBridge = false
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Mute and Solo buttons section
                                        Column {
                                            anchors.right: parent.right
                                            anchors.rightMargin: mainWindow.stemButtonsRightMargin
                                            anchors.verticalCenter: parent.verticalCenter
                                            spacing: 4
                                            
                                            Button {
                                                id: soloButton
                                                checkable: false
                                                iconName: "funnel"
                                                
                                                onClicked: {
                                                    if (multiBridge) {
                                                        multiBridge.solo_track(stemRect.index)
                                                    }
                                                }
                                                
                                                background: Rectangle {
                                                    color: Styles.buttonInactiveColor
                                                    radius: 3
                                                }
                                            }
                                            
                                            Button {
                                                id: muteButton
                                                checkable: true
                                                iconName: "volume-x"
                                                
                                                // Backend controls the state, UI reflects it
                                                checked: multiBridge ? multiBridge.get_file_mute(stemRect.index) : false
                                                
                                                onClicked: {
                                                    if (multiBridge) {
                                                        multiBridge.toggle_mute(stemRect.index)
                                                    }
                                                }
                                                
                                                // React to playback settings changes
                                                Connections {
                                                    target: multiBridge
                                                    function onPlayback_settings_changed() {
                                                        // Force mute button binding to re-evaluate
                                                        muteButton.checked = Qt.binding(function() { 
                                                            return multiBridge ? multiBridge.get_file_mute(stemRect.index) : false 
                                                        })
                                                    }
                                                }
                                                
                                                background: Rectangle {
                                                    color: muteButton.checked ? Styles.muteButtonActiveColor : Styles.buttonInactiveColor
                                                    radius: 3
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Transport Controls Section pinned to bottom
        Rectangle {
            id: transportControlsContainer
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: mainWindow.transportControlsHeight
            color: "transparent"
            radius: 8

            DebugBorderRectangle {}
            
            GroupBox {
                id: transportSection
                anchors.fill: parent
                
                background: Rectangle {
                    color: "transparent"
                }
                
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width
                    height: transportRowHeight
                    spacing: transportRowSpacing
                    
                    // Column 1: Load Song button and song name (narrow)
                    Item {
                        id: fileLoader  // Moved from fileLoadingSection
                        width: transportColumnWidth
                        height: transportColumnHeight

                        // Properties moved from fileLoadingSection
                        property int fileCount: multiBridge ? multiBridge.file_count : 0
                        readonly property bool isBusy: {
                            if (!multiBridge) return false
                            let state = multiBridge.loading_state
                            if (!state) return false
                            return state.is_loading || state.type === "Downloading" || state.type === "SeparatingStems"
                        }

                        function openFileDialog() {
                            if (fileLoader.isBusy) return
                            originalFileDialog.open()
                        }

                        function requestDownloadUrl() {
                            if (fileLoader.isBusy) return
                            downloadPromptDialog.open()
                        }

                        function submitDownloadUrl(candidateUrl) {
                            downloadUrlError.visible = false
                            let rawValue = candidateUrl || ""
                            let trimmed = rawValue.trim()

                            if (!trimmed.length) {
                                downloadUrlError.text = "Enter a link to download."
                                downloadUrlError.visible = true
                                downloadUrlField.forceActiveFocus()
                                return false
                            }

                            if (!multiBridge || !multiBridge.download_stems_from_url) {
                                downloadUrlError.text = "Player is not ready."
                                downloadUrlError.visible = true
                                return false
                            }

                            downloadPromptDialog.close()
                            multiBridge.download_stems_from_url(trimmed)
                            return true
                        }


                        // Loading timeout timer
                        Timer {
                            id: loadingTimeoutTimer
                            interval: 30000  // 30 second timeout
                            running: {
                                if (!multiBridge) return false
                                let loadingState = multiBridge.get_loading_state()
                                return loadingState && loadingState.is_loading
                            }
                            repeat: false
                            
                            onTriggered: {
                                if (multiBridge) {
                                    let loadingState = multiBridge.get_loading_state()
                                    if (loadingState && loadingState.is_loading) {
                                        console.warn("Loading timeout detected after 30 seconds")
                                        // Loading will be handled by existing error mechanisms
                                    }
                                }
                            }
                        }
                        
                        // Load Song section using storybook component
                        LoadSongSection {
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            globalMultiBridge: multiBridge
                            fileLoader: parent
                        }

                        Dialog {
                            id: downloadPromptDialog
                            modal: true
                            focus: true
                            width: 420
                            closePolicy: Popup.NoAutoClose
                            title: "Download and separate"

                            onOpened: {
                                downloadUrlField.text = ""
                                downloadUrlError.visible = false
                                downloadUrlField.forceActiveFocus()
                            }

                            contentItem: Column {
                                spacing: 12
                                padding: 20

                                Text {
                                    text: "Paste a video link to download audio and generate stems."
                                    color: Styles.secondaryTextColor
                                    wrapMode: Text.WordWrap
                                }

                                TextField {
                                    id: downloadUrlField
                                    placeholderText: "https://..."
                                    selectByMouse: true
                                    onAccepted: downloadAction.click()
                                }

                                Text {
                                    id: downloadUrlError
                                    visible: false
                                    color: Styles.muteButtonActiveColor
                                    text: ""
                                    wrapMode: Text.WordWrap
                                }
                            }

                            footer: Item {
                                width: parent.width
                                implicitHeight: actionRow.height + 20

                                Row {
                                    id: actionRow
                                    spacing: 12
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    anchors.margins: 20

                                    Button {
                                        text: "Cancel"
                                        onClicked: downloadPromptDialog.close()
                                    }

                                    Button {
                                        id: downloadAction
                                        text: "Download"
                                        enabled: downloadUrlField.text.length > 0
                                        onClicked: {
                                            if (fileLoader.submitDownloadUrl(downloadUrlField.text)) {
                                                downloadUrlField.text = ""
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        DebugBorderRectangle {
                            label: "File Loader Column"
                            borderColor: "lime"
                        }
                    }
                    
                    // Column 2: Play/pause button above timeline (main section)
                    Item {
                        width: transportColumnWidth
                        height: transportColumnHeight
                        
                        // Play/pause button centered with absolute positioning
                        Button {
                            id: playButton
                            width: 40
                            height: 40
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            anchors.topMargin: 0
                            enabled: multiBridge && multiBridge.file_count > 0
                            
                            onClicked: {
                                if (!multiBridge) return
                                if (multiBridge.is_playing) {
                                    multiBridge.pause()
                                } else {
                                    multiBridge.play()
                                }
                            }
                            
                            background: Rectangle {
                                color: playButton.enabled ? 
                                    (playButton.pressed ? Styles.transportButtonPressedColor : playButton.hovered ? Styles.transportButtonHoverColor : Styles.transportButtonColor) :
                                    Styles.transportButtonDisabledColor
                                radius: 22
                            }
                            
                            contentItem: Text {
                                text: (multiBridge && multiBridge.is_playing) ? "⏸" : "▶"
                                color: playButton.enabled ? Styles.transportIconColor : Styles.transportIconDisabledColor
                                font.pointSize: 16
                                horizontalAlignment: Text.AlignHCenter
                                verticalAlignment: Text.AlignVCenter
                            }
                        }
                        
                        // Timeline and time display - flattened structure with absolute positioning
                        Row {
                            width: parent.width - 20  // Add margin to prevent overlap
                            height: 20
                            spacing: 12  // Increased spacing between elements
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.bottom: parent.bottom
                            anchors.bottomMargin: mainWindow.transportControlsHeight * 0.05
                            
                            Text {
                                id: currentTimeLabel
                                text: {
                                    if (!multiBridge) return "00:00"
                                    let pos = multiBridge.current_position
                                    let mins = Math.floor(pos / 60)
                                    let secs = Math.floor(pos % 60)
                                    return mins.toString().padStart(2, '0') + ":" + secs.toString().padStart(2, '0')
                                }
                                font.pointSize: 10  // Smaller font size
                                color: Styles.primaryTextColor
                                width: 40  // Slightly narrower
                                horizontalAlignment: Text.AlignRight
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            
                            Rectangle {
                                id: seekBarContainer
                                width: parent.width - 40 - 40 - 24  // Adjusted for new widths and spacing
                                height: Styles.transportSeekBarHeight
                                color: Styles.sliderTrackColor
                                radius: Styles.transportSeekBarHeight / 2
                                anchors.verticalCenter: parent.verticalCenter
                                
                                property bool isHovered: seekBarMouseArea.containsMouse
                                
                                Rectangle {
                                    id: seekBarProgress
                                    anchors.left: parent.left
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: {
                                        if (!multiBridge || multiBridge.duration <= 0) return 0
                                        return (multiBridge.current_position / multiBridge.duration) * parent.width
                                    }
                                    height: parent.height
                                    color: seekBarContainer.isHovered ? Styles.transportSliderHoverColor : Styles.transportSliderColor
                                    radius: Styles.transportSeekBarHeight / 2
                                    
                                    Behavior on color {
                                        ColorAnimation { duration: 150 }
                                    }
                                }
                                
                                // Position indicator circle
                                Rectangle {
                                    visible: seekBarContainer.isHovered
                                    width: 12
                                    height: 12
                                    radius: 6
                                    color: Styles.transportSliderHandleColor
                                    anchors.verticalCenter: parent.verticalCenter
                                    x: seekBarProgress.width - width / 2
                                    
                                    Behavior on opacity {
                                        NumberAnimation { duration: 150 }
                                    }
                                }
                                
                                MouseArea {
                                    id: seekBarMouseArea
                                    anchors.centerIn: parent
                                    width: parent.width
                                    height: 20  // Larger hit area for easier interaction
                                    hoverEnabled: true
                                    enabled: multiBridge && multiBridge.file_count > 0 && multiBridge.duration > 0
                                    
                                    onClicked: function(mouse) {
                                        if (!multiBridge) return
                                        let position = (mouse.x / width) * multiBridge.duration
                                        multiBridge.seek(position)
                                    }
                                    
                                    onPositionChanged: function(mouse) {
                                        if (pressed && multiBridge) {
                                            let position = Math.max(0, Math.min(1, mouse.x / width)) * multiBridge.duration
                                            multiBridge.seek(position)
                                        }
                                    }
                                }
                            }
                            
                            Text {
                                id: durationLabel
                                text: {
                                    if (!multiBridge) return "00:00"
                                    let dur = multiBridge.duration
                                    let mins = Math.floor(dur / 60)
                                    let secs = Math.floor(dur % 60)
                                    return mins.toString().padStart(2, '0') + ":" + secs.toString().padStart(2, '0')
                                }
                                font.pointSize: 10  // Smaller font size to match current time
                                color: Styles.primaryTextColor
                                width: 40  // Match width of current time label
                                horizontalAlignment: Text.AlignLeft
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                        
                        DebugBorderRectangle {
                            label: "Timeline Column"
                            borderColor: "lime"
                        }
                    }
                    
                    // Column 3: Master volume control (narrow)
                    Item {
                        width: transportColumnWidth
                        height: transportColumnHeight

                        // Use Item container for better control over positioning
                        Item {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10

                            DebugBorderRectangle {}
                            
                            // Master volume controls (positioned just left of buttons)
                            Row {
                                anchors.right: buttonsColumn.left
                                anchors.rightMargin: 12  // Small gap between volume controls and buttons
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 8
                                
                                Text {
                                    text: Math.round((multiBridge ? multiBridge.master_volume : 1.0) * 100) + "%"
                                    font.pointSize: 10
                                    color: Styles.secondaryTextColor
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: 35
                                    horizontalAlignment: Text.AlignRight
                                }
                                
                                VolumeIcon {
                                    volume: multiBridge ? multiBridge.master_volume : 1.0
                                    muted: false
                                    iconColor: Styles.primaryTextColor
                                    iconSize: Styles.volumeIconSize
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: 20
                                }

                                Rectangle {
                                    id: masterVolumeSlider
                                    width: 120
                                    height: Styles.transportVolumeSliderHeight
                                    color: Styles.sliderTrackColor
                                    radius: Styles.transportVolumeSliderHeight / 2
                                    anchors.verticalCenter: parent.verticalCenter
                                    
                                    property real value: multiBridge ? Math.min(2.0, multiBridge.master_volume) : 1.0
                                    property bool isHovered: volumeMouseArea.containsMouse
                                    
                                    Rectangle {
                                        id: masterVolumeProgress
                                        anchors.left: parent.left
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: Math.min(1.0, masterVolumeSlider.value / 2.0) * parent.width
                                        height: parent.height
                                        color: masterVolumeSlider.isHovered ? Styles.transportSliderHoverColor : Styles.transportSliderColor
                                        radius: Styles.transportVolumeSliderHeight / 2
                                        
                                        Behavior on color {
                                            ColorAnimation { duration: 150 }
                                        }
                                    }
                                    
                                    // Position indicator circle
                                    Rectangle {
                                        visible: masterVolumeSlider.isHovered
                                        width: 12
                                        height: 12
                                        radius: 6
                                        color: Styles.transportSliderHandleColor
                                        anchors.verticalCenter: parent.verticalCenter
                                        x: Math.min(1.0, masterVolumeSlider.value / 2.0) * parent.width - width / 2
                                        
                                        Behavior on opacity {
                                            NumberAnimation { duration: 150 }
                                        }
                                    }
                                    
                                    MouseArea {
                                        id: volumeMouseArea
                                        anchors.centerIn: parent
                                        width: parent.width
                                        height: 20  // Larger hit area for easier interaction
                                        hoverEnabled: true
                                        
                                        onClicked: function(mouse) {
                                            if (!multiBridge) return
                                            let normalized = Math.max(0, Math.min(1, mouse.x / width))
                                            let newVolume = normalized * 2.0
                                            multiBridge.set_master_volume(newVolume)
                                        }
                                        
                                        onPositionChanged: function(mouse) {
                                            if (pressed && multiBridge) {
                                                let normalized = Math.max(0, Math.min(1, mouse.x / width))
                                                let newVolume = normalized * 2.0
                                                multiBridge.set_master_volume(newVolume)
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Reset and Mute All buttons (right side, arranged like per-waveform controls)
                            Column {
                                id: buttonsColumn
                                anchors.right: parent.right
                                anchors.rightMargin: mainWindow.masterButtonsRightMargin
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4
                                
                                // Reset button (like Solo button)
                                Button {
                                    id: resetVolumesButton
                                    enabled: multiBridge && multiBridge.file_count > 0
                                    iconName: "rotate-ccw"
                                    
                                    onClicked: {
                                        if (multiBridge) {
                                            multiBridge.reset_all_volumes()
                                        }
                                    }
                                    
                                    background: Rectangle {
                                        color: Styles.buttonInactiveColor
                                        radius: 3
                                    }
                                }
                                
                                // Mute All button (like individual Mute button)
                                Button {
                                    id: muteAllButton
                                    checkable: true
                                    enabled: multiBridge && multiBridge.file_count > 0
                                    iconName: "volume-x"
                                    
                                    // Backend controls the state, UI reflects it
                                    checked: multiBridge ? multiBridge.get_all_muted() : false
                                    
                                    onClicked: {
                                        if (multiBridge) {
                                            multiBridge.toggle_mute_all()
                                        }
                                    }
                                    
                                    // React to playback settings changes
                                    Connections {
                                        target: multiBridge
                                        function onPlayback_settings_changed() {
                                            // Force mute button binding to re-evaluate
                                            muteAllButton.checked = Qt.binding(function() { 
                                                return multiBridge ? multiBridge.get_all_muted() : false 
                                            })
                                        }
                                    }
                                    
                                    background: Rectangle {
                                        color: muteAllButton.checked ? Styles.muteButtonActiveColor : Styles.buttonInactiveColor
                                        radius: 3
                                    }
                                }
                            }
                        }
                        
                        DebugBorderRectangle {
                            label: "Volume Column"
                        }
                    }
                }
            }
        }
    }
    
    // Error dialog for displaying error messages
    MessageDialog {
        id: errorDialog
        title: "Error"
        text: ""
        buttons: MessageDialog.Ok
    }
    
    // Keyboard shortcuts for multi-file player
    Shortcut {
        sequence: "Space"
        onActivated: {
            if (multiBridge && multiBridge.file_count > 0) {
                if (multiBridge.is_playing) {
                    multiBridge.pause()
                } else {
                    multiBridge.play()
                }
            }
        }
    }
    
    Shortcut {
        sequence: "Ctrl+O"
        onActivated: {
            fileLoader.openFileDialog()
        }
    }
    
    
    
    // Single file selection dialog for original audio
    FileDialog {
        id: originalFileDialog
        title: "Select Original Audio File"
        nameFilters: [
            "Audio files (*.wav *.mp3 *.flac *.m4a *.aac *.ogg *.opus *.aiff *.aif *.caf *.mp4 *.mkv *.mov)",
            "All files (*)"
        ]
        
        onAccepted: {
            let originalFilePath = selectedFile.toString()
            
            if (typeof multiBridge !== 'undefined') {
                let success = multiBridge.load_single_file(originalFilePath)
                if (success) {
                    console.log("Single file loaded successfully with stems")
                } else {
                    console.log("Single file loading failed")
                }
            }
        }
    }
}
