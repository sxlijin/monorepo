pragma Singleton
import QtQuick 2.15

// Dark mode color palette for Stems Player
QtObject {
    // Main backgrounds
    // NB(sam): I'd really like to try a different color here, but I have no idea what to use.
    // Tried out burnt orange (from Rust, Ferris) but couldn't get it to be dark enough for a good
    // background color. Ivy green also felt weird. All of the colors felt like they had awkward contrast
    // with the waveform background color.
    readonly property color windowBackgroundColor: "#1e1e1e"
    readonly property color groupBoxBackgroundColor: "#2a2a2a"
    readonly property color controlsPanelBackgroundColor: "#2a2a2a"
    readonly property color stemRectBackgroundColor: "#1a1a1a"
    readonly property color waveformContainerBackgroundColor: "#0f0f0f"
    readonly property color waveformCanvasBackgroundColor: "#0f0f0f"
    readonly property color stemLabelBackgroundColor: "#0f0f0f"
    
    // Borders
    readonly property color groupBoxBorderColor: "#404040"
    readonly property color stemRectBorderColor: "#404040"
    readonly property color controlsPanelBorderColor: "#404040"
    readonly property color sliderBorderColor: "#666666"
    readonly property color buttonBorderColor: "#666666"
    readonly property color buttonBorderInactiveColor: "#666666"
    
    // Text colors
    readonly property color primaryTextColor: "#ffffff"
    readonly property color secondaryTextColor: "#cccccc"
    readonly property color statusReadyColor: "#4ade80"
    readonly property color statusInitializingColor: "#fbbf24"
    readonly property color waveformCenterLineColor: "#333333"
    readonly property color loadingTextColor: "#999999"
    
    // Slider colors
    readonly property color sliderTrackColor: "#404040"
    readonly property color sliderTrackGradientTop: "#4a4a4a"  // Slightly lighter for gradient
    readonly property color sliderTrackGradientBottom: "#353535"  // Slightly darker for gradient
    readonly property color sliderHandleColor: "#666666"  // This will be overridden by stem colors
    readonly property color sliderTickColor: "#666666"  // Color for tick marks
    readonly property real sliderTickOpacity: 0.9  // Opacity for tick marks
    readonly property real sliderFillOpacity: 0.6  // Opacity for active fill
    readonly property color sliderHoverLineColor: "#888888"  // Color for hover preview line
    readonly property real sliderHoverLineOpacity: 0.6  // Opacity for hover line
    
    // ScrollBar colors
    readonly property color scrollBarBackgroundColor: "#2a2a2a"
    readonly property color scrollBarHandleColor: "#555555"
    readonly property color scrollBarHandleHoverColor: "#666666"
    readonly property color scrollBarHandlePressedColor: "#777777"
    
    // Button states
    readonly property color buttonInactiveColor: "#404040"
    readonly property color buttonTextInactiveColor: "#cccccc"
    readonly property color buttonTextActiveColor: "#ffffff"
    
    // Button active colors
    readonly property color soloButtonActiveColor: "#d0966d"
    readonly property color soloButtonActiveBorderColor: "#cc8800"
    readonly property color muteButtonActiveColor: "#ff6666"
    readonly property color muteButtonActiveBorderColor: "#cc0000"
    
    // Transport control colors (lighter grey for better visibility)
    readonly property color transportButtonColor: "#505050"
    readonly property color transportButtonHoverColor: "#686868"
    readonly property color transportButtonPressedColor: "#383838"
    readonly property color transportButtonDisabledColor: "#2a2a2a"
    readonly property color transportSliderColor: "#707070"
    readonly property color transportSliderHoverColor: "#909090"  // Brighter on hover
    readonly property color transportSliderHandleColor: "#ffffff"  // White handle circle
    readonly property color transportIconColor: "#ffffff"
    readonly property color transportIconDisabledColor: "#888888"
    
    // Transport control dimensions
    readonly property int transportSeekBarHeight: 4  // Height for timeline/seekbar
    readonly property int transportVolumeSliderHeight: 4  // Height for master volume slider
    
    // Icon sizes
    readonly property int muteButtonIconSize: 40  // Size for mute button icons
    readonly property int volumeIconSize: 15  // Size for general volume icons
    
    // UI update rate
    readonly property int uiUpdateFps: 500  // Target FPS for UI state updates
    readonly property int uiUpdateIntervalMs: Math.round(1000 / uiUpdateFps)  // Milliseconds between updates
    
    // Stem colors (kept bright for visibility)
    readonly property color stemVocalsColor: "#3498db"
    readonly property color stemDrumsColor: "#e74c3c"
    readonly property color stemOtherColor: "#2ecc71"
    readonly property color stemBassColor: "#f39c12"
    
    // Darker stem colors for default thumb state
    readonly property color stemVocalsColorDark: "#2f89c5"
    readonly property color stemDrumsColorDark: "#d04436"
    readonly property color stemOtherColorDark: "#29b866"
    readonly property color stemBassColorDark: "#db8d10"
    
    // Brighter stem colors for hover thumb state
    readonly property color stemVocalsColorBright: "#39a8f2"
    readonly property color stemDrumsColorBright: "#ff5442"
    readonly property color stemOtherColorBright: "#33e27c"
    readonly property color stemBassColorBright: "#ffac14"
    
    // Custom dark palette for ApplicationWindow
    readonly property QtObject darkPalette: QtObject {
        readonly property color windowColor: "#1e1e1e"
        readonly property color windowTextColor: "#ffffff"
        readonly property color baseColor: "#2a2a2a"
        readonly property color alternateBaseColor: "#404040"
        readonly property color textColor: "#ffffff"
        readonly property color buttonColor: "#404040"
        readonly property color buttonTextColor: "#ffffff"
        readonly property color brightTextColor: "#ffffff"
        readonly property color highlightColor: "#3498db"
        readonly property color highlightedTextColor: "#ffffff"
        readonly property color toolTipBaseColor: "#2a2a2a"
        readonly property color toolTipTextColor: "#ffffff"
        readonly property color linkColor: "#3498db"
        readonly property color linkVisitedColor: "#9b59b6"
        readonly property color lightColor: "#666666"
        readonly property color midlightColor: "#555555"
        readonly property color darkColor: "#333333"
        readonly property color midColor: "#444444"
        readonly property color shadowColor: "#000000"
    }
}
