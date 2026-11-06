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
    title: "KDE DJI Mic Mini Control"

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

    component HighContrastScrollBar: QQC2.ScrollBar {
        id: hcBar
        orientation: Qt.Vertical
        policy: size < 1.0 ? QQC2.ScrollBar.AlwaysOn : QQC2.ScrollBar.AsNeeded
        minimumSize: 0.14
        implicitWidth: 12
        padding: 2

        background: Rectangle {
            implicitWidth: 12
            radius: 6
            color: Qt.alpha(root.fgColor, 0.14)
            border.width: 1
            border.color: Qt.alpha(root.fgColor, 0.28)
        }

        contentItem: Rectangle {
            implicitWidth: 8
            radius: 4
            color: hcBar.pressed
                   ? root.accentCol
                   : (hcBar.hovered
                      ? Qt.lighter(root.accentCol, 1.15)
                      : Qt.alpha(root.fgColor, 0.65))
        }
    }

    component StatusBadge: Rectangle {
        property string badgeText: ""
        property color badgeColor: root.accentCol
        radius: 6
        color: Qt.alpha(badgeColor, 0.16)
        border.width: 1
        border.color: Qt.alpha(badgeColor, 0.55)
        implicitWidth: Math.min(240, badgeLbl.implicitWidth + 16)
        implicitHeight: badgeLbl.implicitHeight + 8

        QQC2.Label {
            id: badgeLbl
            anchors.centerIn: parent
            width: parent.width - 12
            horizontalAlignment: Text.AlignHCenter
            text: parent.badgeText
            color: parent.badgeColor
            font.pointSize: root.captionPt
            font.bold: true
            elide: Text.ElideRight
        }
    }

    component KofiPillButton: Rectangle {
        property string buttonText: "Buy Me a Coffee"
        radius: 15
        color: kofiMouse.containsMouse || activeFocus ? "#c81e1e" : "#d93834"
        border.color: activeFocus ? "#ffffff" : "#ffffff33"
        border.width: activeFocus ? 2 : 1
        implicitWidth: kofiRow.implicitWidth + 20
        implicitHeight: Math.max(30, kofiRow.implicitHeight + 10)
        activeFocusOnTab: true
        Accessible.role: Accessible.Button
        Accessible.name: "Buy Russell Milliner a coffee on Ko-fi"
        Keys.onReturnPressed: Qt.openUrlExternally("https://ko-fi.com/russellmilliner")
        Keys.onSpacePressed: Qt.openUrlExternally("https://ko-fi.com/russellmilliner")

        RowLayout {
            id: kofiRow
            anchors.centerIn: parent
            spacing: 6

            QQC2.Label {
                text: "☕"
                font.pointSize: root.smallPt
            }

            QQC2.Label {
                text: buttonText
                font.bold: true
                font.pointSize: root.captionPt
                color: "#ffffff"
            }
        }

        MouseArea {
            id: kofiMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: Qt.openUrlExternally("https://ko-fi.com/russellmilliner")
        }
    }

    component InfoCard: Rectangle {
        default property alias cardContent: innerCol.data
        property string cardTitle: ""
        property string cardSubtitle: ""
        property string rightBadgeText: ""
        property color rightBadgeColor: root.accentCol

        Layout.fillWidth: true
        radius: 8
        color: root.cardBg
        border.width: 1
        border.color: root.borderCol
        implicitHeight: innerCol.implicitHeight + 24

        ColumnLayout {
            id: innerCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 12
            spacing: 8

            RowLayout {
                Layout.fillWidth: true
                visible: cardTitle.length > 0
                spacing: 8

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Kirigami.Heading {
                        Layout.fillWidth: true
                        level: 3
                        text: cardTitle
                        color: root.fgColor
                        font.pointSize: root.subheadPt
                        font.bold: true
                        wrapMode: Text.Wrap
                    }

                    QQC2.Label {
                        Layout.fillWidth: true
                        visible: cardSubtitle.length > 0
                        text: cardSubtitle
                        color: root.mutedFg
                        font.pointSize: root.captionPt
                        wrapMode: Text.Wrap
                    }
                }

                StatusBadge {
                    visible: rightBadgeText.length > 0
                    badgeText: rightBadgeText
                    badgeColor: rightBadgeColor
                }
            }

            Kirigami.Separator {
                Layout.fillWidth: true
                visible: cardTitle.length > 0
            }
        }
    }

    component KeyValueRow: RowLayout {
        property string keyText: ""
        property string valText: ""
        property bool monoValue: false
        property color valColor: root.fgColor

        Layout.fillWidth: true
        spacing: 10

        QQC2.Label {
            Layout.preferredWidth: Math.min(210, root.width * 0.27)
            text: keyText
            color: root.mutedFg
            font.pointSize: root.smallPt
            wrapMode: Text.Wrap
        }

        QQC2.Label {
            Layout.fillWidth: true
            text: valText
            color: valColor
            font.pointSize: root.smallPt
            font.family: monoValue ? "monospace" : Kirigami.Theme.defaultFont.family
            font.bold: !monoValue
            wrapMode: Text.WrapAnywhere
        }
    }

    component ClickableLinkRow: RowLayout {
        property string keyText: ""
        property string linkText: ""
        property string linkUrl: ""

        Layout.fillWidth: true
        spacing: 10

        QQC2.Label {
            Layout.preferredWidth: Math.min(210, root.width * 0.27)
            text: keyText
            color: root.mutedFg
            font.pointSize: root.smallPt
            font.bold: true
            wrapMode: Text.Wrap
        }

        QQC2.Label {
            Layout.fillWidth: true
            text: linkText
            color: root.linkCol
            font.pointSize: root.smallPt
            font.underline: linkMouse.containsMouse
            wrapMode: Text.WrapAnywhere

            MouseArea {
                id: linkMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: Qt.openUrlExternally(linkUrl)
            }
        }
    }

    component TransmitterCard: InfoCard {
        property int txUnit: 1
        property bool txConnected: false
        property int txBatteryPct: 0
        property int txBatteryCode: 0
        property bool txCharging: false
        property int txVuLevel: 0
        property string txSerial: ""
        property string txFirmware: ""
        property string txProduct: ""
        property string txTone: ""
        property string txNcState: ""
        property string txLowCut: ""
        property string txAutoOff: ""
        property string txLeds: ""
        property string txRawHex: ""

        cardTitle: "Transmitter TX " + txUnit + (txProduct.length > 0 ? " (" + txProduct + ")" : "")
        cardSubtitle: txConnected
                      ? ("Wireless Link Active • S/N " + txSerial + " • FW " + txFirmware)
                      : "Powered off, in charging case, or out of wireless range"
        rightBadgeText: txConnected
                        ? ("Battery " + txBatteryPct + "%" + (txCharging ? " ⚡" : "") + " (Code " + txBatteryCode + "/7)")
                        : "DISCONNECTED"
        rightBadgeColor: !txConnected
                         ? root.errCol
                         : (txBatteryPct >= 40 ? root.okCol : (txBatteryPct >= 20 ? root.warnCol : root.errCol))

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 8

            // Battery gauge bar
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: "Battery Charge Level"
                        color: root.mutedFg
                        font.pointSize: root.captionPt
                    }
                    QQC2.Label {
                        text: txConnected
                              ? (txBatteryPct + "% • Gauge " + txBatteryCode + "/7" + (txCharging ? " (Charging ⚡)" : ""))
                              : "Offline"
                        color: root.fgColor
                        font.pointSize: root.captionPt
                        font.bold: true
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: Math.round(10 * root.zoomScale)
                    radius: 5
                    color: Qt.alpha(root.fgColor, 0.12)
                    border.width: 1
                    border.color: Qt.alpha(root.fgColor, 0.22)

                    Rectangle {
                        width: txConnected ? Math.max(6, parent.width * Math.min(1.0, txBatteryPct / 100.0)) : 0
                        height: parent.height
                        radius: 5
                        color: txBatteryPct >= 40 ? root.okCol : (txBatteryPct >= 20 ? root.warnCol : root.errCol)
                    }
                }
            }

            // Live Audio VU Meter Bar
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: "Live Microphone VU Level"
                        color: root.mutedFg
                        font.pointSize: root.captionPt
                    }
                    QQC2.Label {
                        text: txConnected ? (txVuLevel + " / 255 (" + Math.round(Math.min(100, txVuLevel / 1.2)) + "%)") : "0 / 255"
                        color: root.fgColor
                        font.pointSize: root.captionPt
                        font.family: "monospace"
                        font.bold: true
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: Math.round(10 * root.zoomScale)
                    radius: 5
                    color: Qt.alpha(root.fgColor, 0.12)
                    border.width: 1
                    border.color: Qt.alpha(root.fgColor, 0.22)

                    Rectangle {
                        readonly property real frac: txConnected ? Math.min(1.0, Math.max(0.0, txVuLevel / 120.0)) : 0.0
                        width: Math.round(parent.width * frac)
                        height: parent.height
                        radius: 5
                        color: frac > 0.85 ? root.errCol : (frac > 0.60 ? root.warnCol : root.okCol)
                    }
                }
            }

            KeyValueRow { keyText: "Serial Number:"; valText: txConnected ? txSerial : "—"; monoValue: true }
            KeyValueRow { keyText: "Firmware Version:"; valText: txConnected ? txFirmware : "—"; monoValue: true }
            KeyValueRow { keyText: "Noise Cancellation:"; valText: txConnected ? txNcState : "—" }
            KeyValueRow { keyText: "Low-Cut / Voice Tone:"; valText: txConnected ? ("Low-Cut: " + txLowCut + " • Tone: " + txTone) : "—" }
            KeyValueRow { keyText: "LEDs & 15m Auto-Off:"; valText: txConnected ? ("Status LEDs: " + txLeds + " • Auto-Off: " + txAutoOff) : "—" }
            KeyValueRow { keyText: "Raw DUML Slot Bytes:"; valText: txConnected ? txRawHex : "00 40 00 00 00 00 00 00 00 (Absent Stub)"; monoValue: true }
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
                        text: "KDE DJI Mic Mini Control"
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
                    badgeText: controller ? controller.overallStatusBadge : "CHECKING"
                    badgeColor: {
                        if (!controller) return root.warnCol
                        if (controller.overallStatusState === "ok") return root.okCol
                        if (controller.overallStatusState === "warn") return root.warnCol
                        return root.errCol
                    }
                }

                KofiPillButton {
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

        // Standard KDE Plasma Primary Navigation TabBar (Rule 7.6)
        QQC2.TabBar {
            id: mainTabBar
            objectName: "mainTabBar"
            Layout.fillWidth: true
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 6
            Layout.bottomMargin: 4
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

        // Main Tab Views
        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: mainTabBar.currentIndex

            // =========================================================
            // TAB 0: OVERVIEW & LIVE TELEMETRY
            // =========================================================
            Item {
                Flickable {
                    id: overviewFlick
                    objectName: "overviewFlick"
                    anchors.fill: parent
                    anchors.margins: 12
                    contentWidth: width - 16
                    contentHeight: overviewCol.implicitHeight + 16
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    QQC2.ScrollBar.vertical: HighContrastScrollBar {}

                    ColumnLayout {
                        id: overviewCol
                        width: overviewFlick.width - 16
                        spacing: 12

                        // udev Rule Missing Warning Banner
                        InfoCard {
                            visible: controller ? !controller.udevReady : false
                            cardTitle: "USB udev Rule Setup Required for Non-Root Telemetry"
                            cardSubtitle: controller ? controller.udevSummary : "Install /etc/udev/rules.d/99-dji-mic.rules to allow non-root access to USB Interface 6."
                            rightBadgeText: "UDEV SETUP"
                            rightBadgeColor: root.warnCol

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10

                                QQC2.Button {
                                    text: "Install / Repair udev Rule (Konsole sudo)"
                                    icon.name: "utilities-terminal"
                                    font.pointSize: root.smallPt
                                    onClicked: {
                                        if (controller) controller.installUdevRuleInteractive()
                                    }
                                }

                                QQC2.Button {
                                    text: "Copy udev Setup Command"
                                    icon.name: "edit-copy"
                                    font.pointSize: root.smallPt
                                    onClicked: {
                                        if (controller) controller.copyUdevCommandToClipboard()
                                    }
                                }
                            }
                        }

                        // Connection Alert Warning Banner when disconnected
                        InfoCard {
                            visible: controller ? (!controller.rxConnected || controller.txConnectedCount === 0) : false
                            cardTitle: (controller && !controller.rxConnected)
                                       ? "DJI Mic Mini USB-C Receiver Not Connected"
                                       : "No DJI Mic Mini Wireless Transmitter Linked"
                            cardSubtitle: (controller && !controller.rxConnected)
                                          ? "Plug the DJI Mic Mini Receiver (2ca3:4011) into a USB-C port. Desktop popup alerts are active."
                                          : "The USB-C Receiver is plugged in, but both TX1 and TX2 are powered off or inside the charging case."
                            rightBadgeText: "ALERT ACTIVE"
                            rightBadgeColor: root.errCol
                        }

                        // Transmitters Side-by-Side or Stacked depending on width
                        GridLayout {
                            Layout.fillWidth: true
                            columns: root.width >= 900 ? 2 : 1
                            columnSpacing: 12
                            rowSpacing: 12

                            TransmitterCard {
                                txUnit: 1
                                txConnected: controller ? controller.tx1Connected : false
                                txBatteryPct: controller ? controller.tx1BatteryPct : 0
                                txBatteryCode: controller ? controller.tx1BatteryCode : 0
                                txCharging: controller ? controller.tx1Charging : false
                                txVuLevel: controller ? controller.tx1VuLevel : 0
                                txSerial: controller ? controller.tx1Serial : "—"
                                txFirmware: controller ? controller.tx1Firmware : "—"
                                txProduct: controller ? controller.tx1Product : ""
                                txTone: controller ? controller.tx1Tone : "standard"
                                txNcState: controller ? controller.tx1NcSummary : "—"
                                txLowCut: controller ? controller.settingLowCut : "off"
                                txAutoOff: controller ? controller.settingTxAutoOff : "on"
                                txLeds: controller ? controller.settingLeds : "on"
                                txRawHex: controller ? controller.tx1RawFlags : "—"
                            }

                            TransmitterCard {
                                txUnit: 2
                                txConnected: controller ? controller.tx2Connected : false
                                txBatteryPct: controller ? controller.tx2BatteryPct : 0
                                txBatteryCode: controller ? controller.tx2BatteryCode : 0
                                txCharging: controller ? controller.tx2Charging : false
                                txVuLevel: controller ? controller.tx2VuLevel : 0
                                txSerial: controller ? controller.tx2Serial : "—"
                                txFirmware: controller ? controller.tx2Firmware : "—"
                                txProduct: controller ? controller.tx2Product : ""
                                txTone: controller ? controller.tx2Tone : "standard"
                                txNcState: controller ? controller.tx2NcSummary : "—"
                                txLowCut: controller ? controller.settingLowCut : "off"
                                txAutoOff: controller ? controller.settingTxAutoOff : "on"
                                txLeds: controller ? controller.settingLeds : "on"
                                txRawHex: controller ? controller.tx2RawFlags : "—"
                            }
                        }

                        // Receiver (RX) & Audio Overview Card
                        InfoCard {
                            cardTitle: "USB-C Receiver (RX) & System Audio Summary"
                            cardSubtitle: controller ? controller.rxSubtitle : "Receiver hardware & Linux PipeWire status"
                            rightBadgeText: (controller && controller.rxConnected)
                                            ? ("Protocol " + controller.protocolDialect.toUpperCase())
                                            : "USB OFFLINE"
                            rightBadgeColor: (controller && controller.rxConnected) ? root.okCol : root.errCol

                            KeyValueRow { keyText: "Receiver Hardware Model:"; valText: controller ? controller.rxHardwareModel : "—" }
                            KeyValueRow { keyText: "Receiver Serial Number:"; valText: controller ? controller.rxSerial : "—"; monoValue: true }
                            KeyValueRow { keyText: "Receiver Firmware / HW Rev:"; valText: controller ? controller.rxFirmwareAndRev : "—"; monoValue: true }
                            KeyValueRow { keyText: "Receiver Gain Step / Dial:"; valText: controller ? controller.rxGainSummary : "—" }
                            KeyValueRow { keyText: "Audio Channel Track Mode:"; valText: controller ? controller.settingStereo.toUpperCase() : "MONO" }
                            KeyValueRow { keyText: "Noise Cancellation:"; valText: controller ? controller.ncOverallSummary : "—" }
                            KeyValueRow { keyText: "Clip Limiter & Safety (-6dB):"; valText: controller ? ("Clip Limiter: " + controller.settingClipLimiter.toUpperCase() + " • Safety Track: " + controller.settingSafetyTrack.toUpperCase()) : "—" }
                            KeyValueRow { keyText: "Power Management:"; valText: controller ? ("RX 15m Auto-Off: " + controller.settingRxAutoOff.toUpperCase() + " • Camera Sync: " + controller.settingCameraPower.toUpperCase() + " • Plug-Free Speaker: " + controller.settingPlugFree.toUpperCase()) : "—" }
                            KeyValueRow {
                                keyText: "Linux PipeWire / ALSA Capture:"
                                valText: controller ? controller.linuxAudioSummary : "—"
                                valColor: (controller && controller.alsaMuted) ? root.errCol : root.fgColor
                            }
                        }
                    }
                }
            }

            // =========================================================
            // TAB 1: HARDWARE & AUDIO CONTROLS (Instant Apply - Rule 7.7)
            // =========================================================
            Item {
                Flickable {
                    id: controlsFlick
                    objectName: "controlsFlick"
                    anchors.fill: parent
                    anchors.margins: 12
                    contentWidth: width - 16
                    contentHeight: controlsCol.implicitHeight + 16
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    QQC2.ScrollBar.vertical: HighContrastScrollBar {}

                    ColumnLayout {
                        id: controlsCol
                        width: controlsFlick.width - 16
                        spacing: 12

                        InfoCard {
                            cardTitle: "DJI Mic Mini Hardware Controls (Instant Apply over USB Interface 6)"
                            cardSubtitle: "All changes are sent immediately via DUML 0x5B/0x01 and verified against the live receiver heartbeat."
                            rightBadgeText: (controller && controller.rxConnected) ? "USB READY" : "RECEIVER OFFLINE"
                            rightBadgeColor: (controller && controller.rxConnected) ? root.okCol : root.errCol

                            // Noise Cancellation Mode
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Noise Cancellation Strength (Basic vs Strong)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.ComboBox {
                                    model: ["basic", "strong"]
                                    currentIndex: (controller && controller.settingNcMode === "strong") ? 1 : 0
                                    enabled: controller ? controller.rxConnected : false
                                    font.pointSize: root.smallPt
                                    onActivated: function(idx) {
                                        if (controller) controller.applyHardwareSetting("noise-cancel", idx === 1 ? "strong" : "basic")
                                    }
                                }
                            }

                            // Audio Channels (Mono / Stereo)
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Audio Channel Routing (Mono Mixed vs Stereo Split TX1-L / TX2-R)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.ComboBox {
                                    model: ["mono", "stereo"]
                                    currentIndex: (controller && controller.settingStereo === "stereo") ? 1 : 0
                                    enabled: controller ? controller.rxConnected : false
                                    font.pointSize: root.smallPt
                                    onActivated: function(idx) {
                                        if (controller) controller.applyHardwareSetting("stereo", idx === 1 ? "stereo" : "mono")
                                    }
                                }
                            }

                            // Low-Cut Filter
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Low-Cut High-Pass Filter (Attenuates rumble below 150 Hz on TX)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? (controller.settingLowCut === "on") : false
                                    enabled: controller ? controller.rxConnected : false
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.applyHardwareSetting("low-cut", checked ? "on" : "off")
                                    }
                                }
                            }

                            // Safety Track (-6 dB)
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Safety Track (-6 dB Backup Channel in Mono Mode)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? (controller.settingSafetyTrack === "on") : false
                                    enabled: controller ? controller.rxConnected : false
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.applyHardwareSetting("safety-track", checked ? "on" : "off")
                                    }
                                }
                            }

                            // Clip Limiter
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Automatic Clipping Control / Peak Limiter"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? (controller.settingClipLimiter === "on") : false
                                    enabled: controller ? controller.rxConnected : false
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.applyHardwareSetting("clip-limiter", checked ? "on" : "off")
                                    }
                                }
                            }

                            // Transmitter Status LEDs
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Transmitter Status Indicator LEDs (TX1 & TX2)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? (controller.settingLeds === "on") : false
                                    enabled: controller ? controller.rxConnected : false
                                    text: checked ? "LEDs On" : "LEDs Off"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.applyHardwareSetting("mic-leds", checked ? "on" : "off")
                                    }
                                }
                            }

                            // RX Auto Power-Off (15m)
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Auto Power-Off after 15 Minutes of Inactivity (Receiver)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? (controller.settingRxAutoOff === "on") : false
                                    enabled: controller ? controller.rxConnected : false
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.applyHardwareSetting("auto-off-15m", checked ? "on" : "off")
                                    }
                                }
                            }

                            // Power On/Off with Camera
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Synchronize Power On/Off with Host / Camera"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? (controller.settingCameraPower === "on") : false
                                    enabled: controller ? controller.rxConnected : false
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.applyHardwareSetting("camera-power", checked ? "on" : "off")
                                    }
                                }
                            }

                            // Plug-Free External Speaker
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Plug-Free External Speaker Playback Mode"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? (controller.settingPlugFree === "on") : false
                                    enabled: controller ? controller.rxConnected : false
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.applyHardwareSetting("plug-free", checked ? "on" : "off")
                                    }
                                }
                            }
                        }

                        // Linux ALSA / PipeWire Capture Controls Card
                        InfoCard {
                            cardTitle: "Linux ALSA & PipeWire Capture Level Controls"
                            cardSubtitle: controller ? controller.linuxAudioSummary : "ALSA / PipeWire source controls"
                            rightBadgeText: (controller && controller.alsaMuted) ? "MUTED" : ("VOL " + (controller ? controller.alsaVolumePct : 100) + "%")
                            rightBadgeColor: (controller && controller.alsaMuted) ? root.errCol : root.okCol

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Microphone Capture Mute State"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }

                                QQC2.Switch {
                                    checked: controller ? !controller.alsaMuted : true
                                    text: checked ? "Unmuted (Live)" : "Muted"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.setSystemMicMute(!checked)
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12

                                QQC2.Label {
                                    Layout.preferredWidth: 180
                                    text: "Capture Volume (%)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }

                                QQC2.Slider {
                                    id: volSlider
                                    Layout.fillWidth: true
                                    from: 0
                                    to: 100
                                    stepSize: 5
                                    value: controller ? controller.alsaVolumePct : 100
                                    onMoved: {
                                        if (controller) controller.setSystemMicVolume(Math.round(value))
                                    }
                                }

                                QQC2.Label {
                                    Layout.preferredWidth: 64
                                    horizontalAlignment: Text.AlignRight
                                    text: Math.round(volSlider.value) + "%"
                                    color: root.fgColor
                                    font.pointSize: root.smallPt
                                    font.bold: true
                                }
                            }
                        }

                        // AI Noise Cancellation & Ambient Room Noise Learning Card
                        InfoCard {
                            cardTitle: "AI Noise Cancellation & Ambient Room Noise Learning"
                            cardSubtitle: controller
                                          ? ("RNNoise Neural GRU + 4-Band Spectral & Tonal Hum Profiler • Auto-Calibrates TX/RX DSP • " + controller.noiseProfileSource)
                                          : "RNNoise Neural GRU + 4-Band Spectral & Tonal Hum Profiler"
                            rightBadgeText: {
                                if (!controller) return "AI READY"
                                if (controller.noiseLearnActive) return "LEARNING " + controller.noiseLearnProgressPct + "%"
                                if (controller.aiNcMode === "off") return "AI NC OFF"
                                if (controller.aiNcMode === "adaptive") return "AUTO-LEARN (#" + controller.adaptivePasses + ")"
                                return controller.noiseProfileLearned ? "PROFILE LEARNED" : "BASELINE AI"
                            }
                            rightBadgeColor: {
                                if (!controller) return root.accentCol
                                if (controller.noiseLearnActive) return root.warnCol
                                if (controller.aiNcMode === "off") return root.errCol
                                return controller.noiseProfileLearned ? root.okCol : root.accentCol
                            }

                            GridLayout {
                                Layout.fillWidth: true
                                columns: root.width >= 960 ? 2 : 1
                                columnSpacing: 12
                                rowSpacing: 8

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8

                                    QQC2.Label {
                                        Layout.preferredWidth: Math.round(145 * root.zoomScale)
                                        text: "AI Denoise Mode:"
                                        color: root.mutedFg
                                        font.pointSize: root.smallPt
                                        wrapMode: Text.Wrap
                                    }

                                    QQC2.ComboBox {
                                        Layout.fillWidth: true
                                        font.pointSize: root.smallPt
                                        model: [
                                            "Off (Manual Hardware DSP Only)",
                                            "Hardware DSP Only (Basic / Strong)",
                                            "AI Learned Profile + Auto-Tune TX/RX DSP",
                                            "Continuous Adaptive Auto-Learn + AI Denoise"
                                        ]
                                        currentIndex: {
                                            var m = controller ? controller.aiNcMode : "learned_ai"
                                            var keys = ["off", "hardware", "learned_ai", "adaptive"]
                                            var idx = keys.indexOf(m)
                                            return idx >= 0 ? idx : 2
                                        }
                                        onActivated: function(idx) {
                                            var keys = ["off", "hardware", "learned_ai", "adaptive"]
                                            if (controller) controller.setAiNcMode(keys[idx])
                                        }
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8

                                    QQC2.Label {
                                        Layout.preferredWidth: Math.round(145 * root.zoomScale)
                                        text: "Suppression Strength:"
                                        color: root.mutedFg
                                        font.pointSize: root.smallPt
                                        wrapMode: Text.Wrap
                                    }

                                    QQC2.Slider {
                                        Layout.fillWidth: true
                                        from: 0
                                        to: 100
                                        stepSize: 5
                                        value: controller ? controller.aiNcStrengthPct : 80
                                        onMoved: {
                                            if (controller) controller.setAiNcStrength(Math.round(value))
                                        }
                                    }

                                    QQC2.Label {
                                        Layout.preferredWidth: Math.round(48 * root.zoomScale)
                                        horizontalAlignment: Text.AlignRight
                                        text: (controller ? controller.aiNcStrengthPct : 80) + "%"
                                        color: root.fgColor
                                        font.pointSize: root.smallPt
                                        font.bold: true
                                    }
                                }
                            }

                            QQC2.Switch {
                                Layout.fillWidth: true
                                text: controller && controller.aiNcVirtualMicActive
                                      ? "PipeWire AI Noise-Cancelled Virtual Mic ('dji_ai_denoise_mic' — ACTIVE in PipeWire)"
                                      : "Enable PipeWire AI Noise-Cancelled Virtual Microphone ('dji_ai_denoise_mic' for OBS / Zoom / Teams)"
                                font.pointSize: root.smallPt
                                checked: controller ? controller.aiNcVirtualMic : false
                                onToggled: {
                                    if (controller) controller.setAiNcVirtualMic(checked)
                                }
                            }

                            Flow {
                                Layout.fillWidth: true
                                spacing: 8

                                QQC2.Button {
                                    text: (controller && controller.noiseLearnActive)
                                          ? ("Sampling Room Acoustics (" + controller.noiseLearnProgressPct + "%)...")
                                          : "🎙️ Learn Room Noise & Auto-Calibrate TX/RX (3s Sample)"
                                    icon.name: "audio-input-microphone"
                                    font.pointSize: root.smallPt
                                    highlighted: true
                                    enabled: controller ? !controller.noiseLearnActive : false
                                    onClicked: {
                                        if (controller) controller.learnNoiseProfile()
                                    }
                                }

                                QQC2.Button {
                                    text: "Reset Learned Profile"
                                    icon.name: "edit-reset"
                                    font.pointSize: root.smallPt
                                    enabled: controller ? (!controller.noiseLearnActive && controller.noiseProfileLearned) : false
                                    onClicked: {
                                        if (controller) controller.resetNoiseProfile()
                                    }
                                }
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                visible: controller ? (controller.noiseLearnActive || controller.noiseLearnProgressPct > 0) : false
                                height: Math.round(10 * root.zoomScale)
                                radius: 5
                                color: Qt.alpha(root.fgColor, 0.12)
                                border.width: 1
                                border.color: Qt.alpha(root.fgColor, 0.24)

                                Rectangle {
                                    width: Math.max(6, Math.round(parent.width * Math.min(1.0, (controller ? controller.noiseLearnProgressPct : 0) / 100.0)))
                                    height: parent.height
                                    radius: 5
                                    color: (controller && controller.noiseLearnActive) ? root.warnCol : root.okCol
                                }
                            }

                            QQC2.Label {
                                Layout.fillWidth: true
                                text: controller ? controller.noiseLearnStatus : "Ready to sample ambient room acoustics."
                                color: (controller && controller.noiseLearnActive) ? root.warnCol : root.mutedFg
                                font.pointSize: root.captionPt
                                font.bold: controller ? controller.noiseLearnActive : false
                                wrapMode: Text.Wrap
                            }

                            Kirigami.Separator { Layout.fillWidth: true }

                            GridLayout {
                                Layout.fillWidth: true
                                columns: root.width >= 900 ? 2 : 1
                                columnSpacing: 14
                                rowSpacing: 6

                                Repeater {
                                    model: [
                                        { label: "Sub / HVAC Rumble (20–140 Hz)", db: controller ? controller.bandLowRumbleStr : "-55.0" },
                                        { label: "PC Fan / Mains Hum (140–550 Hz)", db: controller ? controller.bandFanHumStr : "-51.0" },
                                        { label: "Mid Room / Reflections (550–2.8kHz)", db: controller ? controller.bandMidRoomStr : "-57.0" },
                                        { label: "High Preamp Hiss (2.8–12 kHz)", db: controller ? controller.bandHighHissStr : "-59.0" }
                                    ]
                                    delegate: ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 3

                                        RowLayout {
                                            Layout.fillWidth: true
                                            QQC2.Label {
                                                Layout.fillWidth: true
                                                text: modelData.label
                                                color: root.mutedFg
                                                font.pointSize: root.captionPt
                                                elide: Text.ElideRight
                                            }
                                            QQC2.Label {
                                                text: modelData.db + " dBFS"
                                                color: root.fgColor
                                                font.pointSize: root.captionPt
                                                font.family: "monospace"
                                                font.bold: true
                                            }
                                        }

                                        Rectangle {
                                            Layout.fillWidth: true
                                            height: Math.round(8 * root.zoomScale)
                                            radius: 4
                                            color: Qt.alpha(root.fgColor, 0.12)
                                            border.width: 1
                                            border.color: Qt.alpha(root.fgColor, 0.22)

                                            Rectangle {
                                                readonly property real norm: Math.max(0.04, Math.min(1.0, (Number(modelData.db) + 85.0) / 65.0))
                                                width: Math.round(parent.width * norm)
                                                height: parent.height
                                                radius: 4
                                                color: norm > 0.68 ? root.errCol : (norm > 0.48 ? root.warnCol : root.okCol)
                                            }
                                        }
                                    }
                                }
                            }

                            KeyValueRow {
                                keyText: "Detected Room Signature:"
                                valText: controller ? controller.noiseSignature : "—"
                            }
                            KeyValueRow {
                                keyText: "Ambient vs RNNoise Floor:"
                                valText: controller ? controller.noiseFloorSummary : "—"
                                monoValue: true
                                valColor: root.okCol
                            }
                            KeyValueRow {
                                keyText: "Learned Filter & HW Rec:"
                                valText: controller ? controller.learnedFilterSummary : "—"
                                monoValue: true
                            }
                            KeyValueRow {
                                keyText: "Last Learned / Pass Count:"
                                valText: controller
                                         ? (controller.noiseProfileLearnedAt + " • Adaptive Passes: " + controller.adaptivePasses)
                                         : "—"
                            }
                        }
                    }
                }
            }

            // =========================================================
            // TAB 2: USB, AUDIO & DUML INSPECTOR
            // =========================================================
            Item {
                Flickable {
                    id: inspectorFlick
                    objectName: "inspectorFlick"
                    anchors.fill: parent
                    anchors.margins: 12
                    contentWidth: width - 16
                    contentHeight: inspectorCol.implicitHeight + 16
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    QQC2.ScrollBar.vertical: HighContrastScrollBar {}

                    ColumnLayout {
                        id: inspectorCol
                        width: inspectorFlick.width - 16
                        spacing: 12

                        InfoCard {
                            cardTitle: "Linux USB Device & Sysfs Topology (VID:PID 2ca3:4011)"
                            cardSubtitle: "All kernel USB device descriptor attributes exposed under /sys/bus/usb/devices"
                            rightBadgeText: (controller && controller.usbPresent) ? "USB PRESENT" : "NOT PRESENT"
                            rightBadgeColor: (controller && controller.usbPresent) ? root.okCol : root.errCol

                            KeyValueRow { keyText: "Vendor / Product ID:"; valText: controller ? controller.usbVidPid : "2ca3:4011"; monoValue: true }
                            KeyValueRow { keyText: "Manufacturer / Product:"; valText: controller ? controller.usbManufacturerProduct : "—" }
                            KeyValueRow { keyText: "USB Descriptor Serial:"; valText: controller ? controller.usbDescriptorSerial : "—"; monoValue: true }
                            KeyValueRow { keyText: "USB Bus / Device / Port:"; valText: controller ? controller.usbTopology : "—"; monoValue: true }
                            KeyValueRow { keyText: "Negotiated Speed / Power:"; valText: controller ? controller.usbSpeedAndPower : "—" }
                            KeyValueRow { keyText: "USBFS Device Node:"; valText: controller ? controller.usbFsNode : "—"; monoValue: true }
                            KeyValueRow { keyText: "Kernel Sysfs Path:"; valText: controller ? controller.usbSysfsPath : "—"; monoValue: true }
                        }

                        InfoCard {
                            cardTitle: "USB HID & Service Interface (Interface 0) + Audio Stack (Interfaces 1 & 3)"
                            cardSubtitle: "Consumer media key interface (Report 0x06), Vendor Service (Report 0x55), and ALSA/PipeWire nodes"

                            KeyValueRow { keyText: "HID Raw Device Node:"; valText: controller ? controller.hidNodeAndPhys : "—"; monoValue: true }
                            KeyValueRow { keyText: "HID Report 0x55 Telemetry:"; valText: controller ? controller.hidServiceSummary : "—"; monoValue: true }
                            KeyValueRow { keyText: "ALSA Sound Card:"; valText: controller ? controller.alsaCardDetails : "—" }
                            KeyValueRow { keyText: "PipeWire Source Node:"; valText: controller ? controller.pipewireNodeDetails : "—"; monoValue: true }
                            KeyValueRow { keyText: "Audio Format & State:"; valText: controller ? controller.pipewireFormatState : "—"; monoValue: true }
                        }

                        InfoCard {
                            cardTitle: "DJI DUML Wire Protocol Inspector (Interface 6 — Bulk IN 0x86 / Bulk OUT 0x06)"
                            cardSubtitle: "Live hexadecimal frame capture and protocol telemetry"
                            rightBadgeText: controller ? ("Seq #" + controller.lastSequence + " (" + controller.heartbeatCount + " frames)") : "0 frames"
                            rightBadgeColor: root.accentCol

                            KeyValueRow { keyText: "Protocol Dialect:"; valText: controller ? controller.protocolDialect.toUpperCase() : "V1"; monoValue: true }
                            KeyValueRow { keyText: "Last 0x5B/0x03 Heartbeat:"; valText: controller ? controller.lastHeartbeatHex : "—"; monoValue: true }
                            KeyValueRow { keyText: "Last 0x5B/0x05 Aux Frame:"; valText: controller ? controller.aux5b05Hex : "—"; monoValue: true }
                            KeyValueRow { keyText: "Last 0x04/0x74 Link Frame:"; valText: controller ? controller.aux0474Hex : "—"; monoValue: true }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10

                                QQC2.Button {
                                    text: "Copy Full JSON Telemetry Snapshot"
                                    icon.name: "edit-copy"
                                    font.pointSize: root.smallPt
                                    onClicked: {
                                        if (controller) controller.copyJsonSnapshotToClipboard()
                                    }
                                }

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: controller ? controller.lastActionMessage : ""
                                    color: root.okCol
                                    font.pointSize: root.captionPt
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }
                }
            }

            // =========================================================
            // TAB 3: ALERTS, UDEV & SETTINGS (Instant Persist - Rule 7.7)
            // =========================================================
            Item {
                Flickable {
                    id: settingsFlick
                    objectName: "settingsFlick"
                    anchors.fill: parent
                    anchors.margins: 12
                    contentWidth: width - 16
                    contentHeight: settingsCol.implicitHeight + 16
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    QQC2.ScrollBar.vertical: HighContrastScrollBar {}

                    ColumnLayout {
                        id: settingsCol
                        width: settingsFlick.width - 16
                        spacing: 12

                        // USB udev Rule Check & Set Card
                        InfoCard {
                            cardTitle: "USB udev Rule & Permission Check (2ca3:4011)"
                            cardSubtitle: "Verifies non-root read/write access to DJI Mic Mini USB Interface 6 (/etc/udev/rules.d/99-dji-mic.rules)"
                            rightBadgeText: (controller && controller.udevReady) ? "UDEV VERIFIED" : "ACTION NEEDED"
                            rightBadgeColor: (controller && controller.udevReady) ? root.okCol : root.warnCol

                            KeyValueRow { keyText: "Rule File Status:"; valText: controller ? controller.udevRuleFileStatus : "—"; monoValue: true }
                            KeyValueRow { keyText: "USB Device Writable:"; valText: controller ? controller.udevDeviceAccessStatus : "—"; monoValue: true }
                            KeyValueRow { keyText: "Overall udev Status:"; valText: controller ? controller.udevSummary : "—" }
                            KeyValueRow { keyText: "udev Rule Definition:"; valText: "SUBSYSTEM==\"usb\", ATTRS{idVendor}==\"2ca3\", ATTRS{idProduct}==\"4011\", MODE=\"0666\", TAG+=\"uaccess\""; monoValue: true }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10

                                QQC2.Button {
                                    text: "Re-Check udev Status"
                                    icon.name: "view-refresh"
                                    font.pointSize: root.smallPt
                                    onClicked: {
                                        if (controller) controller.recheckUdevStatus()
                                    }
                                }

                                QQC2.Button {
                                    text: "Install / Repair udev Rule (Konsole sudo)"
                                    icon.name: "utilities-terminal"
                                    font.pointSize: root.smallPt
                                    onClicked: {
                                        if (controller) controller.installUdevRuleInteractive()
                                    }
                                }

                                QQC2.Button {
                                    text: "Copy udev Setup Command"
                                    icon.name: "edit-copy"
                                    font.pointSize: root.smallPt
                                    onClicked: {
                                        if (controller) controller.copyUdevCommandToClipboard()
                                    }
                                }
                            }
                        }

                        InfoCard {
                            cardTitle: "Desktop Popup Alert Rules (Instant Save)"
                            cardSubtitle: "Sends a KDE Plasma desktop popup notification if the DJI Mic Mini Receiver or Wireless Transmitter is not connected."

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Popup Alert when USB-C Receiver (2ca3:4011) is Unplugged / Disconnected"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? controller.alertOnRxDisconnect : true
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.setAlertOnRxDisconnect(checked)
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Popup Alert when All Wireless Transmitters (TX1 & TX2) are Off / Unlinked"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? controller.alertOnTxDisconnect : true
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.setAlertOnTxDisconnect(checked)
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Popup Alert when Transmitter Battery Drops Below Threshold"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.Switch {
                                    checked: controller ? controller.alertOnLowBattery : true
                                    text: checked ? "Enabled" : "Disabled"
                                    font.pointSize: root.smallPt
                                    onToggled: {
                                        if (controller) controller.setAlertOnLowBattery(checked)
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Low Battery Alert Threshold (%)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.SpinBox {
                                    from: 5
                                    to: 70
                                    stepSize: 5
                                    value: controller ? controller.lowBatteryThresholdPct : 25
                                    font.pointSize: root.smallPt
                                    onValueModified: {
                                        if (controller) controller.setLowBatteryThresholdPct(value)
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12

                                QQC2.Button {
                                    text: "Send Test Desktop Popup Alert Now"
                                    icon.name: "preferences-desktop-notification"
                                    font.pointSize: root.smallPt
                                    onClicked: {
                                        if (controller) controller.sendTestAlert()
                                    }
                                }
                            }
                        }

                        InfoCard {
                            cardTitle: "Appearance & UI Scaling (KDE Plasma Theme Integration)"
                            cardSubtitle: "All settings apply and save immediately to ~/.config/dji-mic-tray/config.json"

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Application Color Scheme"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.ComboBox {
                                    model: ["System Theme (Auto)", "Breeze Dark", "Breeze Light"]
                                    currentIndex: {
                                        if (!controller) return 0
                                        if (controller.colorScheme === "dark") return 1
                                        if (controller.colorScheme === "light") return 2
                                        return 0
                                    }
                                    font.pointSize: root.smallPt
                                    onActivated: function(idx) {
                                        if (!controller) return
                                        if (idx === 1) controller.setColorScheme("dark")
                                        else if (idx === 2) controller.setColorScheme("light")
                                        else controller.setColorScheme("system")
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12
                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "UI Zoom Scale (75% – 175%, or use Ctrl++ / Ctrl+- / Ctrl+0 / Ctrl+Wheel)"
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }
                                QQC2.SpinBox {
                                    from: 75
                                    to: 175
                                    stepSize: 5
                                    value: controller ? controller.uiZoomPercent : 100
                                    font.pointSize: root.smallPt
                                    onValueModified: {
                                        if (controller) controller.setUiZoomPercent(value)
                                    }
                                }
                            }
                        }

                        InfoCard {
                            cardTitle: "Recent Connection & Alert Event Log"
                            cardSubtitle: "Timestamped history of DJI Mic Mini receiver, transmitter, and battery events"

                            QQC2.Label {
                                Layout.fillWidth: true
                                text: (controller && controller.eventLogText.length > 0)
                                      ? controller.eventLogText
                                      : "No disconnection or battery alerts recorded in this session."
                                color: root.fgColor
                                font.pointSize: root.smallPt
                                font.family: "monospace"
                                wrapMode: Text.WrapAnywhere
                            }
                        }
                    }
                }
            }

            // =========================================================
            // TAB 4: ABOUT & SUPPORT (Russell Milliner + Ko-fi)
            // =========================================================
            Item {
                Flickable {
                    id: aboutFlick
                    objectName: "aboutFlick"
                    anchors.fill: parent
                    anchors.margins: 12
                    contentWidth: width - 16
                    contentHeight: aboutCol.implicitHeight + 16
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    QQC2.ScrollBar.vertical: HighContrastScrollBar {}

                    ColumnLayout {
                        id: aboutCol
                        width: aboutFlick.width - 16
                        spacing: 12

                        InfoCard {
                            objectName: "aboutHeaderCard"
                            cardTitle: "KDE DJI Mic Mini Control"
                            cardSubtitle: "Version 0.2.0 • KDE Plasma 6 Kirigami + libusb-1.0 DUML v1/v2 + PipeWire/ALSA"
                            rightBadgeText: "v0.2.0"
                            rightBadgeColor: root.accentCol

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 12

                                QQC2.Label {
                                    Layout.fillWidth: true
                                    text: "Real-time system tray battery gauge, live microphone VU meter, hardware settings controller, USB/HID/DUML protocol inspector, and automatic desktop disconnection popup monitor for the DJI Mic Mini (2ca3:4011) on Linux. Contains zero tracking and zero advertisements — forever."
                                    color: root.fgColor
                                    font.pointSize: root.bodyPt
                                    wrapMode: Text.Wrap
                                }

                                KofiPillButton {
                                    objectName: "aboutKofiButton"
                                    buttonText: "Buy Me a Coffee"
                                }
                            }
                        }

                        InfoCard {
                            objectName: "aboutLinksCard"
                            cardTitle: "Author, Repository & Support Links"
                            cardSubtitle: "Click any link below to open in your default web browser"

                            ClickableLinkRow {
                                keyText: "Author:"
                                linkText: "Russell Milliner (https://github.com/russellmilliner)"
                                linkUrl: "https://github.com/russellmilliner"
                            }

                            ClickableLinkRow {
                                keyText: "GitHub Repository:"
                                linkText: "https://github.com/russellmilliner/dji-mic-mini-kde"
                                linkUrl: "https://github.com/russellmilliner/dji-mic-mini-kde"
                            }

                            ClickableLinkRow {
                                keyText: "Buy Me a Coffee (Ko-fi):"
                                linkText: "https://ko-fi.com/russellmilliner"
                                linkUrl: "https://ko-fi.com/russellmilliner"
                            }

                            ClickableLinkRow {
                                keyText: "License:"
                                linkText: "MIT License (https://opensource.org/licenses/MIT)"
                                linkUrl: "https://opensource.org/licenses/MIT"
                            }

                            ClickableLinkRow {
                                keyText: "Protocol Attribution:"
                                linkText: "DJI DUML USB Interface 6 interoperability notes (piedol/lavdeck & caballeropedro/dji-mic)"
                                linkUrl: "https://github.com/piedol/lavdeck/blob/main/docs/PROTOCOL.md"
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10

                                QQC2.Button {
                                    icon.name: "globe"
                                    text: "GitHub Profile"
                                    font.pointSize: root.smallPt
                                    onClicked: Qt.openUrlExternally("https://github.com/russellmilliner")
                                }

                                QQC2.Button {
                                    icon.name: "favorite"
                                    text: "Support on Ko-fi (Buy Me a Coffee)"
                                    font.pointSize: root.smallPt
                                    onClicked: Qt.openUrlExternally("https://ko-fi.com/russellmilliner")
                                }

                                QQC2.Button {
                                    icon.name: "license"
                                    text: "MIT License"
                                    font.pointSize: root.smallPt
                                    onClicked: Qt.openUrlExternally("https://opensource.org/licenses/MIT")
                                }
                            }
                        }

                        InfoCard {
                            cardTitle: "Security & Architecture Highlights"
                            cardSubtitle: "Hardened local IPC, closed DUML command allowlist, and non-root USB access"

                            KeyValueRow {
                                keyText: "Closed Command Allowlist:"
                                valText: "Strictly blocks firmware-update (0x55/0x35) and reboot (0x52) frames; only verified audio/power TLV settings and read-only version/status queries are permitted."
                            }
                            KeyValueRow {
                                keyText: "Authenticated Local IPC:"
                                valText: "Unix socket (/run/user/<uid>/dji-mic-tray.sock) is created with 0600 permissions and verifies SO_PEERCRED UID on every connection so CLI and Tray share USB Interface 6 safely."
                            }
                            KeyValueRow {
                                keyText: "Subprocess Isolation:"
                                valText: "All system calls (amixer, wpctl, pactl, notify-send, udevadm) execute with explicit argument arrays (shell=False) and strict timeouts."
                            }
                        }
                    }
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
