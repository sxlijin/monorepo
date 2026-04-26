import QtQuick 2.15
import Qt5Compat.GraphicalEffects
import ".."

Item {
    id: root
    
    property real volume: 1.0          // Volume level (0.0 - 2.0)
    property bool muted: false         // Mute state
    property color iconColor: Styles.primaryTextColor  // Icon color
    property real iconSize: 16         // Icon size
    
    width: iconSize
    height: iconSize
    
    // Gyroflow pattern: iconName property
    readonly property string iconName: {
        if (volume <= 0.0) return "volume-off"
        if (volume <= 0.3) return "volume"
        if (volume <= 0.6) return "volume-1"
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