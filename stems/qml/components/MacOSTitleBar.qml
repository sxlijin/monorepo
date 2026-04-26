import QtQuick 2.15
import QtQuick.Window 2.15
import ".."

// Custom title bar component that mimics native macOS appearance
// Provides window dragging and traffic light buttons for frameless windows
Rectangle {
    id: titleBar
    height: 28
    color: Styles.windowBackgroundColor  // Match main window background
    
    // Window reference for dragging and control
    property var targetWindow: null
    
    // Title text properties
    property string title: "Stems Player"
    property color titleColor: Styles.secondaryTextColor
    
    // Mouse area for window dragging
    MouseArea {
        id: dragArea
        anchors.fill: parent
        anchors.leftMargin: 78  // Leave space for traffic lights
        
        onPressed: function(mouse) {
            if (titleBar.targetWindow && mouse.button === Qt.LeftButton) {
                // Use Qt's built-in system move for frameless windows
                titleBar.targetWindow.startSystemMove()
            }
        }
        
        onDoubleClicked: {
            if (titleBar.targetWindow) {
                if (titleBar.targetWindow.visibility === Window.Maximized) {
                    titleBar.targetWindow.showNormal()
                } else {
                    titleBar.targetWindow.showMaximized()
                }
            }
        }
    }
    
    // Traffic light buttons (close, minimize, maximize)
    Row {
        id: trafficLights
        anchors.left: parent.left
        anchors.leftMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
        
        // Close button (red)
        Rectangle {
            id: closeButton
            width: 12
            height: 12
            radius: 6
            color: closeMouseArea.containsMouse ? "#ff5f56" : "#ff5f56"
            border.color: "#e0443e"
            border.width: 0.5
            
            Rectangle {
                anchors.centerIn: parent
                width: 6
                height: 1
                color: closeMouseArea.containsMouse ? "#4c0000" : "transparent"
                rotation: 45
                transformOrigin: Item.Center
                
                Rectangle {
                    anchors.centerIn: parent
                    width: 6
                    height: 1
                    color: closeMouseArea.containsMouse ? "#4c0000" : "transparent"
                    rotation: -90
                    transformOrigin: Item.Center
                }
            }
            
            MouseArea {
                id: closeMouseArea
                anchors.fill: parent
                hoverEnabled: true
                onClicked: {
                    if (titleBar.targetWindow) {
                        titleBar.targetWindow.close()
                    }
                }
            }
        }
        
        // Minimize button (yellow)
        Rectangle {
            id: minimizeButton
            width: 12
            height: 12
            radius: 6
            color: minimizeMouseArea.containsMouse ? "#ffbd2e" : "#ffbd2e"
            border.color: "#dea123"
            border.width: 0.5
            
            Rectangle {
                anchors.centerIn: parent
                width: 6
                height: 1
                color: minimizeMouseArea.containsMouse ? "#995700" : "transparent"
            }
            
            MouseArea {
                id: minimizeMouseArea
                anchors.fill: parent
                hoverEnabled: true
                onClicked: {
                    if (titleBar.targetWindow) {
                        titleBar.targetWindow.showMinimized()
                    }
                }
            }
        }
        
        // Maximize button (green)
        Rectangle {
            id: maximizeButton
            width: 12
            height: 12
            radius: 6
            color: maximizeMouseArea.containsMouse ? "#27c93f" : "#27c93f"
            border.color: "#1aad29"
            border.width: 0.5
            
            // Maximize/restore icon
            Item {
                anchors.centerIn: parent
                width: 6
                height: 6
                visible: maximizeMouseArea.containsMouse
                
                Rectangle {
                    anchors.centerIn: parent
                    width: 4
                    height: 4
                    color: "transparent"
                    border.color: "#0d5016"
                    border.width: 1
                }
                
                Rectangle {
                    anchors.right: parent.right
                    anchors.top: parent.top
                    width: 2
                    height: 1
                    color: "#0d5016"
                }
                
                Rectangle {
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.topMargin: 1
                    width: 1
                    height: 2
                    color: "#0d5016"
                }
            }
            
            MouseArea {
                id: maximizeMouseArea
                anchors.fill: parent
                hoverEnabled: true
                onClicked: {
                    if (titleBar.targetWindow) {
                        if (titleBar.targetWindow.visibility === Window.Maximized) {
                            titleBar.targetWindow.showNormal()
                        } else {
                            titleBar.targetWindow.showMaximized()
                        }
                    }
                }
            }
        }
    }
    
    // Title text
    Text {
        id: titleText
        text: titleBar.title
        color: titleBar.titleColor
        font.pointSize: 13
        font.weight: Font.Medium
        anchors.centerIn: parent
        elide: Text.ElideRight
        
        // Ensure title doesn't overlap with traffic lights
        anchors.leftMargin: 100
        anchors.rightMargin: 20
    }
    
    // Subtle top border
    Rectangle {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: "#404040"
    }
    
    // Subtle bottom border
    Rectangle {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: "#1a1a1a"
    }
}