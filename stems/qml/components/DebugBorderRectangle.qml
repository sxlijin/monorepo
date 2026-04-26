import QtQuick 2.15

// Debug border component to visualize element boundaries
// Usage: Simply add as a child to any element you want to debug
Rectangle {
    id: debugBorder

    property bool isDebugMode: true
    
    // Properties for customization
    property color borderColor: "red"
    property string label: ""
    
    // Always fill parent and stay on top
    anchors.fill: parent
    color: "transparent"
    border.width: isDebugMode ? 1: 0
    border.color: borderColor
    z: 1000  // Ensure debug border is visible on top
    
    // Optional label to identify what's being debugged
    Rectangle {
        visible: isDebugMode
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.margins: 2
        width: labelText.width + 4
        height: labelText.height + 2
        color: "white"
        opacity: 0.8
        
        Text {
            id: labelText
            anchors.centerIn: parent
            text: debugBorder.label
            color: debugBorder.borderColor
            font.pointSize: 8
            font.bold: true
        }
    }
}