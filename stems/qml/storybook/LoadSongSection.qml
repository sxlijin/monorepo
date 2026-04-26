import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import "../components"
import ".."

// Extracted Load Song section from multi_player.qml for storybook demos
Item {
    id: root
    width: 500  // Explicit width to contain button + spacing + metadata
    height: 100 // Explicit height for the content
    
    // Properties to connect with mock bridges
    property var globalMultiBridge
    property var fileLoader
    
    // Signal connections to ensure UI updates when loading state changes
    Connections {
        target: typeof globalMultiBridge !== 'undefined' ? globalMultiBridge : null
    }
    
    readonly property bool isBusy: {
        if (!globalMultiBridge) return false
        let state = globalMultiBridge.loading_state
        if (!state) return false
        return state.is_loading || state.type === "Downloading" || state.type === "SeparatingStems"
    }

    Row {
        anchors.left: parent.left
        anchors.leftMargin: 20
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        Button {
            id: downloadButton
            iconName: "file-music"
            width: 40
            height: 50
            anchors.verticalCenter: parent.verticalCenter
            enabled: !root.isBusy
            tooltip: "Download and separate"

            DebugBorderRectangle {}

            icon.width: 26
            icon.height: 26

            onClicked: {
                if (fileLoader && fileLoader.requestDownloadUrl) {
                    fileLoader.requestDownloadUrl()
                }
            }

            background: Rectangle {
                color: "transparent"
                radius: 6
            }
        }

        // Load Song button with icon that changes on hover
        Button {
            id: loadSongButton
            iconName: hovered ? "folder-open" : "music-4"
            width: 50
            height: 50
            anchors.verticalCenter: parent.verticalCenter

            DebugBorderRectangle {}
            
            // Override icon size for larger button
            icon.width: 30
            icon.height: 30
            
            enabled: !root.isBusy

            onClicked: {
                if (fileLoader) {
                    fileLoader.openFileDialog()
                }
            }
            
            background: Rectangle {
                color: "transparent"
                radius: 6
            }
        }
        
        // Song metadata display with fixed positioning
        Item {
            anchors.verticalCenter: parent.verticalCenter
            width: 200
            height: 60  // Fixed height container
            
            // Artist text positioned above center
            
            Text {
                id: songArtistText
                width: parent.width
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: songTitleText.top
                anchors.bottomMargin: 2
                text: {
                    if (!globalMultiBridge) return ""
                    var display = globalMultiBridge.load_song_display
                    return display.artist_text || ""
                }
                font.pointSize: 11
                color: Styles.secondaryTextColor
                wrapMode: Text.WordWrap
                elide: Text.ElideRight
                maximumLineCount: 1
            }
            
            // Primary text always centered
            Text {
                id: songTitleText
                width: parent.width
                anchors.centerIn: parent
                text: {
                    if (!globalMultiBridge) return "(loading...)"
                    var display = globalMultiBridge.load_song_display
                    return display.title_text || "[Loading...]"
                }
                font.pointSize: 13
                font.bold: true
                color: Styles.primaryTextColor
                wrapMode: Text.WordWrap
                elide: Text.ElideRight
                maximumLineCount: 2
            }
            
            // Status text positioned below center
            Text {
                id: loadingStatusText
                width: parent.width
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: songTitleText.bottom
                anchors.topMargin: 2
                text: {
                    if (!globalMultiBridge) return ""
                    var display = globalMultiBridge.load_song_display
                    return display.status_text || ""
                }
                font.pointSize: 11
                color: {
                    if (!globalMultiBridge) return "#ff8000"
                    let loadingState = globalMultiBridge.loading_state
                    return (loadingState && loadingState.type === "Failed") ? "#ff4444" : "#ff8000"
                }
                visible: {
                    if (!globalMultiBridge) return false
                    var display = globalMultiBridge.load_song_display
                    return display.status_visible || false
                }
                
                // Add subtle opacity pulse during loading (not for error states)
                SequentialAnimation on opacity {
                    loops: Animation.Infinite
                    running: {
                        if (!globalMultiBridge) return false
                        var display = globalMultiBridge.load_song_display
                        // Animate when status is visible and contains a percentage
                        return display.status_visible && display.status_text.includes("%")
                    }
                    
                    NumberAnimation { 
                        from: 1.0; to: 0.6
                        duration: 1000
                        easing.type: Easing.InOutSine
                    }
                    NumberAnimation { 
                        from: 0.6; to: 1.0
                        duration: 1000
                        easing.type: Easing.InOutSine
                    }
                }
            }
            
        }  // End of Song metadata display Item
    }  // End of Row
}
