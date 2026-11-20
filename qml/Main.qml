import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root
    objectName: "mainWindow"
    visible: controller ? controller.windowVisible : true
    width: Math.max(1220, Math.ceil((mainTabBar ? mainTabBar.implicitWidth : 1140) + 40))
    height: 780
    minimumWidth: 760
    minimumHeight: 540
    readonly property string appVer: (controller && controller.appVersion) ? controller.appVersion : "0.2.5"
    title: "KDE DJI Mic Mini Control v" + appVer

    readonly property real basePointSize: (Kirigami.Theme.defaultFont && Kirigami.Theme.defaultFont.pointSize > 0)
                                          ? Kirigami.Theme.defaultFont.pointSize : 10.0
    readonly property real zoomScale: (controller ? controller.uiZoomPercent : 100) / 100.0

    readonly property real captionPt: Math.max(7.5, basePointSize * 0.85 * zoomScale)
    readonly property real smallPt: Math.max(8.0, basePointSize * 0.92 * zoomScale)
    readonly property real bodyPt: Math.max(8.5, basePointSize * 1.00 * zoomScale)
    readonly property real subheadPt: Math.max(9.5, basePointSize * 1.12 * zoomScale)
    readonly property real headPt: Math.max(10.5, basePointSize * 1.26 * zoomScale)

    readonly property string schemeMode: controller ? controller.colorScheme : "system"
    readonly property color winBg: schemeMode === "dark" ? "#1b1e20"
                                 : (schemeMode === "light" ? "#eff0f1" : Kirigami.Theme.backgroundColor)
    readonly property color cardBg: schemeMode === "dark" ? "#232629"
                                  : (schemeMode === "light" ? "#ffffff" : Kirigami.Theme.alternateBackgroundColor)
    readonly property color fgColor: schemeMode === "dark" ? "#fcfcfc"
                                   : (schemeMode === "light" ? "#232629" : Kirigami.Theme.textColor)
    readonly property color mutedFg: Qt.alpha(fgColor, 0.72)
    readonly property color borderCol: Qt.alpha(fgColor, 0.18)
    readonly property color accentCol: Kirigami.Theme.highlightColor
    readonly property color linkCol: schemeMode === "light" ? "#0284c7" : "#38bdf8"
    readonly property color okCol: Kirigami.Theme.positiveTextColor
    readonly property color warnCol: Kirigami.Theme.neutralTextColor
    readonly property color errCol: Kirigami.Theme.negativeTextColor

    color: winBg

    onClosing: function(close) {
        if (controller && !controller.quitting) {
            close.accepted = false
            controller.setWindowVisible(false)
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Top Header Banner
        Rectangle {
            Layout.fillWidth: true
            color: root.cardBg
            border.width: 1
            border.color: root.borderCol
            implicitHeight: topHeaderLayout.implicitHeight + 20

            RowLayout {
                id: topHeaderLayout
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 10
                spacing: 10

                Kirigami.Icon {
                    source: controller ? controller.appIconUrl : "audio-input-microphone"
                    Layout.preferredWidth: Math.round(36 * root.zoomScale)
                    Layout.preferredHeight: Math.round(36 * root.zoomScale)
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Kirigami.Heading {
                        Layout.fillWidth: true
                        level: 2
                        text: "KDE DJI Mic Mini Control v" + root.appVer
                        color: root.fgColor
                        font.pointSize: root.headPt
                        font.bold: true
                        wrapMode: Text.Wrap
                    }

                    QQC2.Label {
                        Layout.fillWidth: true
                        text: controller ? controller.headerSubtitle : "Initializing USB & PipeWire telemetry..."
                        color: root.mutedFg
                        font.pointSize: root.smallPt
                        wrapMode: Text.Wrap
                    }
                }

                StatusBadge {
                    app: root
                    badgeText: controller ? controller.overallStatusBadge : "CHECKING"
                    badgeColor: {
                        if (!controller) return root.warnCol
                        if (controller.overallStatusState === "ok") return root.okCol
                        if (controller.overallStatusState === "warn") return root.warnCol
                        return root.errCol
                    }
                }

                KofiPillButton {
                    app: root
                    buttonText: root.width >= 920 ? "Ko-fi" : "☕"
                }

                QQC2.Button {
                    text: "Refresh"
                    icon.name: "view-refresh"
                    font.pointSize: root.smallPt
                    onClicked: {
                        if (controller) controller.refreshNow()
                    }
                }
            }
        }

        // Standard KDE Plasma Primary Navigation TabBar Attached Directly to Its Content Box (Rule 7.6)
        ColumnLayout {
            id: tabGroupContainer
            objectName: "tabGroupContainer"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 8
            Layout.bottomMargin: 8
            spacing: 0

            QQC2.TabBar {
                id: mainTabBar
                objectName: "mainTabBar"
                Layout.fillWidth: true
                spacing: Math.round(10 * root.zoomScale)
                currentIndex: controller ? controller.activeTabIndex : 0
                onCurrentIndexChanged: {
                    if (controller && controller.activeTabIndex !== currentIndex) {
                        controller.setActiveTabIndex(currentIndex)
                    }
                }
                onImplicitWidthChanged: {
                    var needed = Math.ceil(implicitWidth + 40)
                    if (root.width < needed) {
                        root.width = needed
                    }
                }

                // Override org.kde.desktop TabBar's internal ListView.spacing (-tabOverlap)
                // so tabs have genuine visual separation between each other
                Binding {
                    target: mainTabBar.contentItem
                    property: "spacing"
                    value: Math.round(10 * root.zoomScale)
                    when: mainTabBar.contentItem !== null
                }

                QQC2.TabButton {
                    text: "Overview & Telemetry"
                    icon.name: "audio-input-microphone"
                    font.pointSize: root.bodyPt
                    width: implicitWidth + Math.round(16 * root.zoomScale)
                }
                QQC2.TabButton {
                    text: "Hardware Controls"
                    icon.name: "configure"
                    font.pointSize: root.bodyPt
                    width: implicitWidth + Math.round(16 * root.zoomScale)
                }
                QQC2.TabButton {
                    text: "USB & Protocol Inspector"
                    icon.name: "utilities-terminal"
                    font.pointSize: root.bodyPt
                    width: implicitWidth + Math.round(16 * root.zoomScale)
                }
                QQC2.TabButton {
                    text: "Alerts, udev & Settings"
                    icon.name: "preferences-system"
                    font.pointSize: root.bodyPt
                    width: implicitWidth + Math.round(16 * root.zoomScale)
                }
                QQC2.TabButton {
                    objectName: "tabButtonAbout"
                    text: "About"
                    icon.name: "help-about"
                    font.pointSize: root.bodyPt
                    width: implicitWidth + Math.round(16 * root.zoomScale)
                }
            }

            // Attached Bordered Box Surrounding Tab Content (Rule 7.6)
            Rectangle {
                id: tabContentBox
                objectName: "tabContentBox"
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: Qt.alpha(root.cardBg, 0.55)
                border.width: 1
                border.color: root.borderCol
                radius: 6
                clip: true

                // Square top-left and top-right corners where TabBar attaches directly to the box
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 6
                    color: parent.color
                    border.width: 0
                }
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 1
                    color: root.borderCol
                }
                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    width: 1
                    height: 6
                    color: root.borderCol
                }
                Rectangle {
                    anchors.right: parent.right
                    anchors.top: parent.top
                    width: 1
                    height: 6
                    color: root.borderCol
                }

                // Main Tab Views
                StackLayout {
                    anchors.fill: parent
                    currentIndex: mainTabBar.currentIndex

                    OverviewTab { app: root }
                    ControlsTab { app: root }
                    InspectorTab { app: root }
                    SettingsTab { app: root }
                    AboutTab { app: root }
                }
            }
        }

        // Dedicated Bottom Status Bar with Zoom Controls (Rule 7.2)
        Rectangle {
            id: bottomStatusBar
            objectName: "bottomStatusBar"
            Layout.fillWidth: true
            color: root.cardBg
            border.width: 1
            border.color: root.borderCol
            implicitHeight: statusBarRow.implicitHeight + 12

            RowLayout {
                id: statusBarRow
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 6
                spacing: 10

                QQC2.Label {
                    Layout.fillWidth: true
                    text: controller ? controller.statusBarSummary : "Ready"
                    color: root.mutedFg
                    font.pointSize: root.captionPt
                    elide: Text.ElideRight
                }

                Rectangle {
                    width: 1
                    height: 18
                    color: root.borderCol
                }

                QQC2.Label {
                    text: "Zoom:"
                    color: root.mutedFg
                    font.pointSize: root.captionPt
                }

                QQC2.ToolButton {
                    objectName: "zoomOutBtn"
                    text: "−"
                    font.pointSize: root.smallPt
                    font.bold: true
                    onClicked: {
                        if (controller) controller.zoomOut()
                    }
                }

                QQC2.ToolButton {
                    objectName: "zoomResetBtn"
                    text: (controller ? controller.uiZoomPercent : 100) + "%"
                    font.pointSize: root.captionPt
                    font.bold: true
                    onClicked: {
                        if (controller) controller.zoomReset()
                    }
                }

                QQC2.ToolButton {
                    objectName: "zoomInBtn"
                    text: "+"
                    font.pointSize: root.smallPt
                    font.bold: true
                    onClicked: {
                        if (controller) controller.zoomIn()
                    }
                }
            }
        }
    }
}
