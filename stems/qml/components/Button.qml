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
    icon.width: 15
    icon.height: 15
    icon.color: textColor
    
    // Sizing and behavior
    width: 40
    height:35 
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
        opacity: (!parent.enabled && root.fadeWhenDisabled ? 0.75 : root.down ? 0.75 : 1.0)
        radius: 0
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
        visible: root.text.length > 0 && root.hovered
    }
    
    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Enter || event.key === Qt.Key_Return) {
            root.clicked()
        }
    }
}