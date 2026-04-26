import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import StemsUI 1.0
import "../components"
import "../"
import "."

// Qt Storybook - Unified Canvas vs QPainter Performance Testing
ApplicationWindow {
    id: mainWindow
    objectName: "QtStorybook"
    visible: true
    width: 1200
    height: 800
    title: "Qt Storybook - Canvas vs QPainter Performance Comparison"
    
    // Marquee animation configuration
    readonly property real marqueeSpeed: 400.0  // pixels per second

    // Tab bar at the top
    header: TabBar {
        id: tabBar
        TabButton {
            text: "Loading States"
        }
        TabButton {
            text: "Gallery"
        }
        TabButton {
            text: "Canvas Demo"
        }
        TabButton {
            text: "QPainter Demo"
        }
        TabButton {
            text: "Performance Comparison"
        }
        TabButton {
            text: "Qt Integration Test"
        }
    }

    // Main content area with stack layout
    StackLayout {
        anchors.fill: parent
        currentIndex: tabBar.currentIndex

        // Loading States Demo Tab
        Rectangle {
            id: loadingStatesTab
            color: Styles.windowBackgroundColor
            
            // Mock bridge objects for different loading states
            property var mockIdleBridge: QtObject {
                function get_loading_state() {
                    return {
                        type: "Idle",
                        file_count: 0,
                        is_loading: false,
                        stage_message: ""
                    }
                }
                function get_song_metadata() {
                    return {
                        title: null,
                        artist: null
                    }
                }
                function get_load_song_display() {
                    return {
                        artist_text: "",
                        title_text: "No song loaded",
                        status_text: "",
                        status_visible: false
                    }
                }
                property var loading_state: get_loading_state()
            }
            
            property var mockLoadingAudioBridge: QtObject {
                function get_loading_state() {
                    return {
                        type: "LoadingAudio",
                        file_count: 1,
                        is_loading: true,
                        stage_message: "Reading file data",
                        progress: 0.3
                    }
                }
                function get_song_metadata() {
                    return {
                        title: "Loading audio...",
                        artist: "bohemian_rhapsody.wav",
                        filename: "bohemian_rhapsody.wav"
                    }
                }
                function get_load_song_display() {
                    return {
                        artist_text: "",
                        title_text: "Loading audio files...",
                        status_text: "30%",
                        status_visible: true
                    }
                }
                property var loading_state: get_loading_state()
            }
            
            property var mockGeneratingWaveformsBridge: QtObject {
                function get_loading_state() {
                    return {
                        type: "GeneratingWaveforms",
                        file_count: 4,
                        is_loading: true,
                        stage_message: "Generating waveforms... (2/4)",
                        progress: 0.5,
                        waveforms_completed: 2,
                        waveforms_total: 4
                    }
                }
                function get_song_metadata() {
                    return {
                        title: "Bohemian Rhapsody",
                        artist: "Queen"
                    }
                }
                function get_load_song_display() {
                    return {
                        artist_text: "",
                        title_text: "Analyzing audio...",
                        status_text: "50%",
                        status_visible: true
                    }
                }
                property var loading_state: get_loading_state()
            }
            
            property var mockCompleteBridge: QtObject {
                function get_loading_state() {
                    return {
                        type: "Complete",
                        file_count: 4,
                        is_loading: false,
                        stage_message: "Complete",
                        all_waveforms_ready: true
                    }
                }
                function get_song_metadata() {
                    return {
                        title: "Bohemian Rhapsody",
                        artist: "Queen",
                        bpm: 120
                    }
                }
                function get_load_song_display() {
                    return {
                        artist_text: "Queen",
                        title_text: "Bohemian Rhapsody",
                        status_text: "",
                        status_visible: false
                    }
                }
                property var loading_state: get_loading_state()
            }
            
            property var mockFailedBridge: QtObject {
                function get_loading_state() {
                    return {
                        type: "Failed",
                        file_count: 1,
                        is_loading: false,
                        stage_message: "Invalid audio format",
                        error_message: "Invalid audio format"
                    }
                }
                function get_song_metadata() {
                    return {
                        title: "Failed to load",
                        artist: "corrupted_file.wav"
                    }
                }
                function get_load_song_display() {
                    return {
                        artist_text: "",
                        title_text: "Failed to load",
                        status_text: "",
                        status_visible: false
                    }
                }
                property var loading_state: get_loading_state()
            }
            
            property var mockFileLoader: QtObject {
                function openFileDialog() {
                    // Mock file dialog - no actual dialog in storybook
                }

                function requestDownloadUrl() {
                    console.log("storybook: download button clicked")
                }
            }
            
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 20

                Text {
                    text: "Loading States Demo"
                    font.pointSize: 18
                    font.bold: true
                    Layout.alignment: Qt.AlignHCenter
                }
                
                Text {
                    text: "Shows the Load Song section in different loading states"
                    font.pointSize: 12
                    color: "#666666"
                    Layout.alignment: Qt.AlignHCenter
                }

                // 2x3 Grid of loading states (5 states + 1 empty slot)
                GridLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    columns: 2
                    rowSpacing: 20
                    columnSpacing: 25
                    
                    // Idle State
                    GroupBox {
                        title: "Idle State"
                        Layout.preferredWidth: 550
                        Layout.preferredHeight: 180
                        
                        label: Text {
                            text: "Idle State"
                            color: "#ffffff"
                            font.bold: true
                            font.pointSize: 12
                        }
                        
                        LoadSongSection {
                            id: idleSection
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.margins: 10
                            anchors.leftMargin: 20
                            multiBridge: loadingStatesTab.mockIdleBridge
                            fileLoader: loadingStatesTab.mockFileLoader
                            
                            DebugBorderRectangle {
                                label: "LoadSongSection - Idle"
                                borderColor: "blue"
                            }
                        }
                    }
                    
                    // Loading Audio State
                    GroupBox {
                        title: "Loading Audio State"
                        Layout.preferredWidth: 550
                        Layout.preferredHeight: 180
                        
                        label: Text {
                            text: "Loading Audio State"
                            color: "#ffffff"
                            font.bold: true
                            font.pointSize: 12
                        }
                        
                        LoadSongSection {
                            id: loadingAudioSection
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.margins: 10
                            anchors.leftMargin: 20
                            multiBridge: loadingStatesTab.mockLoadingAudioBridge
                            fileLoader: QtObject {
                                function openFileDialog() { /* Mock file dialog */ }
                            }
                            
                            DebugBorderRectangle {
                                label: "LoadSongSection - Loading"
                                borderColor: "orange"
                            }
                        }
                    }
                    
                    // Generating Waveforms State
                    GroupBox {
                        title: "Generating Waveforms State"
                        Layout.preferredWidth: 550
                        Layout.preferredHeight: 180
                        
                        label: Text {
                            text: "Generating Waveforms State"
                            color: "#ffffff"
                            font.bold: true
                            font.pointSize: 12
                        }
                        
                        LoadSongSection {
                            id: generatingWaveformsSection
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.margins: 10
                            anchors.leftMargin: 20
                            multiBridge: loadingStatesTab.mockGeneratingWaveformsBridge
                            fileLoader: QtObject {
                                function openFileDialog() { /* Mock file dialog */ }
                            }
                            
                            DebugBorderRectangle {
                                label: "LoadSongSection - Generating"
                                borderColor: "yellow"
                            }
                        }
                    }
                    
                    // Complete State
                    GroupBox {
                        title: "Complete State"
                        Layout.preferredWidth: 550
                        Layout.preferredHeight: 180
                        
                        label: Text {
                            text: "Complete State"
                            color: "#ffffff"
                            font.bold: true
                            font.pointSize: 12
                        }
                        
                        LoadSongSection {
                            id: completeSection
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.margins: 10
                            anchors.leftMargin: 20
                            multiBridge: loadingStatesTab.mockCompleteBridge
                            fileLoader: loadingStatesTab.mockFileLoader
                            
                            DebugBorderRectangle {
                                label: "LoadSongSection - Complete"
                                borderColor: "green"
                            }
                        }
                    }
                    
                    // Failed State
                    GroupBox {
                        title: "Failed State"
                        Layout.preferredWidth: 550
                        Layout.preferredHeight: 180
                        
                        label: Text {
                            text: "Failed State"
                            color: "#ffffff"
                            font.bold: true
                            font.pointSize: 12
                        }
                        
                        LoadSongSection {
                            id: failedSection
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.margins: 10
                            anchors.leftMargin: 20
                            multiBridge: loadingStatesTab.mockFailedBridge
                            fileLoader: QtObject {
                                function openFileDialog() { /* Mock file dialog */ }
                            }
                            
                            DebugBorderRectangle {
                                label: "LoadSongSection - Failed"
                                borderColor: "red"
                            }
                        }
                    }
                }
            }
        }

        Item {
            id: galleryTab
            
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 20
                spacing: 20

                Text {
                    text: "Icon Gallery"
                    font.pointSize: 18
                    font.bold: true
                    Layout.alignment: Qt.AlignHCenter
                }
                
                Text {
                    text: "All available icons from resources/icons/svg/"
                    font.pointSize: 12
                    color: "#666666"
                    Layout.alignment: Qt.AlignHCenter
                }

                // Icon grid
                Grid {
                    Layout.alignment: Qt.AlignHCenter
                    columns: 3
                    spacing: 30

                    // loader-circle icon
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: loaderCircleBtn
                            iconName: "loader-circle"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "loader-circle"
                            font.pointSize: 10
                            anchors.top: loaderCircleBtn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                    // volume icon
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volumeBtn
                            iconName: "volume"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "volume"
                            font.pointSize: 10
                            anchors.top: volumeBtn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                    // volume-1 icon
                    Item {
                        width: 100
                        height: 70

                        DebugBorderRectangle {}
                        
                        Button {
                            id: volume1Btn
                            iconName: "volume-1"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "volume-1"
                            font.pointSize: 10
                            anchors.top: volume1Btn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                    // volume-2 icon
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volume2Btn
                            iconName: "volume-2"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "volume-2"
                            font.pointSize: 10
                            anchors.top: volume2Btn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                    // volume-off icon
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volumeOffBtn
                            iconName: "volume-off"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "volume-off"
                            font.pointSize: 10
                            anchors.top: volumeOffBtn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                    // volume-x icon
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volumeXBtn
                            iconName: "volume-x"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "volume-x"
                            font.pointSize: 10
                            anchors.top: volumeXBtn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: microscopeBtn
                            iconName: "microscope"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "microscope"
                            font.pointSize: 10
                            anchors.top: microscopeBtn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: rotateCcwBtn
                            iconName: "rotate-ccw"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "rotate-ccw"
                            font.pointSize: 10
                            anchors.top: rotateCcwBtn.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volumeXBtn2
                            iconName: "volume-x"
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                        
                        Text {
                            text: "volume-x"
                            font.pointSize: 10
                            anchors.top: volumeXBtn2.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volumeIconBtn1
                            iconName: "loader-circle"
                            anchors.horizontalCenter: parent.horizontalCenter
                            contentItem: VolumeIcon {
                                volume: 0.2
                                iconColor: Styles.buttonTextActiveColor
                                iconSize: 25
                            }
                        }

                        Text {
                            text: "VolumeIcon"
                            font.pointSize: 10
                            anchors.top: volumeIconBtn1.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volumeIconBtn2
                            iconName: "loader-circle"
                            anchors.horizontalCenter: parent.horizontalCenter
                            contentItem: VolumeIcon {
                                volume: 0.5
                                iconColor: Styles.buttonTextActiveColor
                                iconSize: 25
                                // anchors.centerIn: parent
                            }
                        }

                        Text {
                            text: "VolumeIcon"
                            font.pointSize: 10
                            anchors.top: volumeIconBtn2.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }
                    Item {
                        width: 100
                        height: 70
                        
                        DebugBorderRectangle {}
                        
                        Button {
                            id: volumeIconBtn3
                            anchors.horizontalCenter: parent.horizontalCenter
                            contentItem: VolumeIcon {
                                volume: 0.8
                                iconColor: Styles.buttonTextActiveColor
                                iconSize: 25
                                // anchors.centerIn: parent
                            }
                        }

                        Text {
                            text: "VolumeIcon"
                            font.pointSize: 10
                            anchors.top: volumeIconBtn3.bottom
                            anchors.topMargin: 5
                            anchors.horizontalCenter: parent.horizontalCenter
                        }
                    }

                }

                // Spacer to push content up
                Item {
                    Layout.fillHeight: true
                }
            }
            
        }
        // Tab 1: Canvas Demo
        Item {
            id: canvasTab
            
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 10
                
                Text {
                    text: "Canvas-based Waveform Rendering"
                    font.pointSize: 16
                    font.bold: true
                    Layout.alignment: Qt.AlignHCenter
                }
                
                Text {
                    text: "Uses HTML5 Canvas with JavaScript for rendering waveforms"
                    font.pointSize: 10
                    color: "#666666"
                    Layout.alignment: Qt.AlignHCenter
                }
                
                // Performance stats for Canvas
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 60
                    color: "#f5f5f5"
                    border.color: "#ddd"
                    radius: 5
                    
                    GridLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        columns: 3
                        
                        Text {
                            text: "Target FPS: " + (latencyBridge ? latencyBridge.target_fps : 0)
                            font.pointSize: 12
                        }
                        
                        Text {
                            text: "Actual FPS: " + (latencyBridge ? latencyBridge.actual_fps.toFixed(1) : "0.0")
                            font.pointSize: 12
                            color: latencyBridge && latencyBridge.actual_fps > 0.9 * latencyBridge.target_fps ? "green" : "red"
                        }
                        
                        Text {
                            text: "Frame Time: " + (latencyBridge ? latencyBridge.frame_time_ms.toFixed(2) : "0.00") + " ms"
                            font.pointSize: 12
                        }
                    }
                }
                
                // Canvas waveform component with marquee animation
                CanvasWaveformComponent {
                    id: canvasWaveform
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    marqueeEnabled: true
                    marqueeSpeed: mainWindow.marqueeSpeed
                }
            }
        }

        // Tab 2: QPainter Demo  
        Item {
            id: qpainterTab
            
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 10
                
                Text {
                    text: "QPainter-based Waveform Rendering"
                    font.pointSize: 16
                    font.bold: true
                    Layout.alignment: Qt.AlignHCenter
                }
                
                Text {
                    text: "Uses Qt's native QPainter with hardware acceleration"
                    font.pointSize: 10
                    color: "#666666"
                    Layout.alignment: Qt.AlignHCenter
                }
                
                // Performance stats for QPainter
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 60
                    color: "#f5f5f5"
                    border.color: "#ddd"
                    radius: 5
                    
                    GridLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        columns: 3
                        
                        Text {
                            text: "Target FPS: " + (latencyBridge ? latencyBridge.target_fps : 0)
                            font.pointSize: 12
                        }
                        
                        Text {
                            text: "Actual FPS: " + (latencyBridge ? latencyBridge.actual_fps.toFixed(1) : "0.0")
                            font.pointSize: 12
                            color: latencyBridge && latencyBridge.actual_fps > 0.9 * latencyBridge.target_fps ? "green" : "red"
                        }
                        
                        Text {
                            text: "Frame Time: " + (latencyBridge ? latencyBridge.frame_time_ms.toFixed(2) : "0.00") + " ms"
                            font.pointSize: 12
                        }
                    }
                }
                
                // QPainter waveform component
                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: "#000000"
                    radius: 4
                    
                    LatencyWaveformComponent {
                        id: qpainterWaveform
                        anchors.fill: parent
                        anchors.margins: 2
                        
                        // Bind properties from latency bridge
                        current_position: latencyBridge ? latencyBridge.current_position : 0
                        duration: latencyBridge ? latencyBridge.duration : 30.0
                        zoom_level: latencyBridge ? latencyBridge.zoom_level : 1.0
                        is_playing: latencyBridge ? latencyBridge.is_playing : false
                        waveform_complexity: latencyBridge ? latencyBridge.waveform_complexity : 1000
                        
                        // Set visual properties
                        background_color: "#000000"
                        waveform_color: "#4CAF50"
                        center_line_color: "#666666"
                        cursor_color: "#FF0000"
                        
                        // Enable marquee animation
                        marquee_enabled: true
                        marquee_speed: mainWindow.marqueeSpeed
                        
                        // Marquee animation timer
                        Timer {
                            id: qpainterMarqueeTimer
                            interval: 16  // ~60 FPS
                            running: qpainterWaveform.marquee_enabled
                            repeat: true
                            property real startTime: 0
                            
                            onTriggered: {
                                if (startTime === 0) startTime = Date.now()
                                var elapsed = (Date.now() - startTime) / 1000.0
                                var offset = (elapsed * qpainterWaveform.marquee_speed) % qpainterWaveform.width
                                qpainterWaveform.set_marquee_offset(offset)
                            }
                            
                            onRunningChanged: {
                                if (running) startTime = 0
                            }
                        }
                        
                        // Update when bridge properties change
                        Connections {
                            target: latencyBridge
                            function onCurrent_position_changed() {
                                qpainterWaveform.set_position(latencyBridge.current_position)
                            }
                            function onZoom_level_changed() {
                                qpainterWaveform.set_zoom(latencyBridge.zoom_level)
                            }
                            function onWaveform_complexity_changed() {
                                qpainterWaveform.set_complexity(latencyBridge.waveform_complexity)
                            }
                        }
                        
                        // Mouse interaction
                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton
                            
                            onClicked: function(mouse) {
                                if (latencyBridge && qpainterWaveform.duration > 0) {
                                    let timelineProgress = mouse.x / width
                                    let newPosition = timelineProgress * qpainterWaveform.duration
                                    latencyBridge.seek(newPosition)
                                }
                            }
                        }
                    }
                }
            }
        }

        // Tab 3: Performance Comparison
        Item {
            id: comparisonTab
            
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 10
                
                Text {
                    text: "Performance Comparison: Canvas vs QPainter"
                    font.pointSize: 16
                    font.bold: true
                    Layout.alignment: Qt.AlignHCenter
                }
                
                // Side-by-side comparison
                RowLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 10
                    
                    // Canvas side
                    GroupBox {
                        title: "Canvas Rendering"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        
                        ColumnLayout {
                            anchors.fill: parent
                            
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 40
                                color: "#ffe6e6"
                                border.color: "#ffcccc"
                                radius: 3
                                
                                Text {
                                    anchors.centerIn: parent
                                    text: "JavaScript + Canvas 2D"
                                    font.bold: true
                                }
                            }
                            
                            CanvasWaveformComponent {
                                id: comparisonCanvas
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                marqueeEnabled: true
                                marqueeSpeed: mainWindow.marqueeSpeed
                            }
                        }
                    }
                    
                    // QPainter side
                    GroupBox {
                        title: "QPainter Rendering"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        
                        ColumnLayout {
                            anchors.fill: parent
                            
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 40
                                color: "#e6ffe6"
                                border.color: "#ccffcc"
                                radius: 3
                                
                                Text {
                                    anchors.centerIn: parent
                                    text: "Qt Native + Hardware Acceleration"
                                    font.bold: true
                                }
                            }
                            
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                color: "#000000"
                                radius: 4
                                
                                LatencyWaveformComponent {
                                    id: comparisonQPainter
                                    anchors.fill: parent
                                    anchors.margins: 2
                                    
                                    // Same properties as other QPainter instance
                                    current_position: latencyBridge ? latencyBridge.current_position : 0
                                    duration: latencyBridge ? latencyBridge.duration : 30.0
                                    zoom_level: latencyBridge ? latencyBridge.zoom_level : 1.0
                                    is_playing: latencyBridge ? latencyBridge.is_playing : false
                                    waveform_complexity: latencyBridge ? latencyBridge.waveform_complexity : 1000
                                    
                                    background_color: "#000000"
                                    waveform_color: "#4CAF50"
                                    center_line_color: "#666666"
                                    cursor_color: "#FF0000"
                                    
                                    // Enable marquee animation
                                    marquee_enabled: true
                                    marquee_speed: mainWindow.marqueeSpeed
                                    
                                    // Marquee animation timer
                                    Timer {
                                        id: comparisonMarqueeTimer
                                        interval: 16  // ~60 FPS
                                        running: comparisonQPainter.marquee_enabled
                                        repeat: true
                                        property real startTime: 0
                                        
                                        onTriggered: {
                                            if (startTime === 0) startTime = Date.now()
                                            var elapsed = (Date.now() - startTime) / 1000.0
                                            var offset = (elapsed * comparisonQPainter.marquee_speed) % comparisonQPainter.width
                                            comparisonQPainter.set_marquee_offset(offset)
                                        }
                                        
                                        onRunningChanged: {
                                            if (running) startTime = 0
                                        }
                                    }
                                    
                                    // Update when bridge properties change
                                    Connections {
                                        target: latencyBridge
                                        function onCurrent_position_changed() {
                                            comparisonQPainter.set_position(latencyBridge.current_position)
                                        }
                                        function onZoom_level_changed() {
                                            comparisonQPainter.set_zoom(latencyBridge.zoom_level)
                                        }
                                        function onWaveform_complexity_changed() {
                                            comparisonQPainter.set_complexity(latencyBridge.waveform_complexity)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Performance metrics comparison
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 100
                    color: "#f9f9f9"
                    border.color: "#ddd"
                    radius: 5
                    
                    GridLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        columns: 2
                        
                        Text {
                            text: "Rendering Method Comparison"
                            font.pointSize: 14
                            font.bold: true
                            Layout.columnSpan: 2
                            Layout.alignment: Qt.AlignHCenter
                        }
                        
                        Text {
                            text: "Canvas: Software rendering, JavaScript overhead"
                            font.pointSize: 11
                        }
                        
                        Text {
                            text: "QPainter: Hardware acceleration, native code"
                            font.pointSize: 11
                        }
                    }
                }
            }
        }

        // Tab 4: Qt Integration Test (original content)
        Item {
            id: integrationTab
            
            Column {
                anchors.centerIn: parent
                spacing: 20

                Text {
                    text: "Qt Storybook Integration Test"
                    font.pointSize: 16
                    font.bold: true
                    anchors.horizontalCenter: parent.horizontalCenter
                }

                Button {
                    text: "Test Bridge Connection"
                    anchors.horizontalCenter: parent.horizontalCenter
                    onClicked: {
                        console.log("Button clicked")
                        if (typeof playerBridge !== 'undefined') {
                            console.log("Bridge available!")
                            var info = playerBridge.get_player_info()
                            console.log("Player info:", JSON.stringify(info))
                        } else {
                            console.log("Bridge not available")
                        }
                    }
                }

                Text {
                    text: typeof playerBridge !== 'undefined' ? "Player Bridge: Connected" : "Player Bridge: Not Found"
                    anchors.horizontalCenter: parent.horizontalCenter
                    color: typeof playerBridge !== 'undefined' ? "green" : "red"
                }

                Text {
                    text: typeof latencyBridge !== 'undefined' ? "Latency Bridge: Connected" : "Latency Bridge: Not Found"
                    anchors.horizontalCenter: parent.horizontalCenter
                    color: typeof latencyBridge !== 'undefined' ? "green" : "red"
                }
                
                // Latency bridge controls
                GroupBox {
                    title: "Latency Bridge Controls"
                    anchors.horizontalCenter: parent.horizontalCenter
                    
                    GridLayout {
                        columns: 2
                        
                        Text { text: "Target FPS:" }
                        SpinBox {
                            from: 10
                            to: 500
                            value: latencyBridge ? latencyBridge.target_fps : 500
                            onValueChanged: {
                                if (latencyBridge) {
                                    latencyBridge.set_target_fps(value)
                                }
                            }
                        }
                        
                        Text { text: "Complexity:" }
                        SpinBox {
                            from: 100
                            to: 10000
                            stepSize: 100
                            value: latencyBridge ? latencyBridge.waveform_complexity : 1000
                            onValueChanged: {
                                if (latencyBridge) {
                                    latencyBridge.set_waveform_complexity(value)
                                }
                            }
                        }
                        
                        Button {
                            text: latencyBridge && latencyBridge.is_playing ? "Pause" : "Play"
                            Layout.columnSpan: 2
                            Layout.alignment: Qt.AlignHCenter
                            onClicked: {
                                if (latencyBridge) {
                                    latencyBridge.toggle_playback()
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Component.onCompleted: {
        // Qt Storybook loaded successfully
    }
}
