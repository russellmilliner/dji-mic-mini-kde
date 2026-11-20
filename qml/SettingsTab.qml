import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

Item {
    id: tab
    required property var app

    Flickable {
        id: settingsFlick
        objectName: "settingsFlick"
        anchors.fill: parent
        anchors.margins: 12
        contentWidth: width - 16
        contentHeight: settingsCol.implicitHeight + 16
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        QQC2.ScrollBar.vertical: HighContrastScrollBar { app: tab.app }

        ColumnLayout {
            id: settingsCol
            width: settingsFlick.width - 16
            spacing: 12

            // USB udev Rule Check & Set Card
            InfoCard {
                app: tab.app
                cardTitle: "USB udev Rule & Permission Check (2ca3:4011)"
                cardSubtitle: "Verifies hardened 0660 + uaccess permissions for DJI Mic Mini USB Interface 6 and hidraw (/etc/udev/rules.d/99-dji-mic.rules)"
                rightBadgeText: (controller && controller.udevReady) ? "UDEV VERIFIED" : "ACTION NEEDED"
                rightBadgeColor: (controller && controller.udevReady) ? tab.app.okCol : tab.app.warnCol

                KeyValueRow { app: tab.app; keyText: "Rule File Status:"; valText: controller ? controller.udevRuleFileStatus : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "USB Device Writable:"; valText: controller ? controller.udevDeviceAccessStatus : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Overall udev Status:"; valText: controller ? controller.udevSummary : "—" }
                KeyValueRow {
                    app: tab.app
                    keyText: "udev Rule Definition:"
                    valText: "SUBSYSTEM==\"usb\", ATTR{idVendor}==\"2ca3\", ATTR{idProduct}==\"4011\", MODE=\"0660\", TAG+=\"uaccess\""
                    monoValue: true
                }

                Flow {
                    Layout.fillWidth: true
                    Layout.minimumHeight: implicitHeight
                    Layout.preferredHeight: implicitHeight
                    spacing: 10

                    QQC2.Button {
                        text: "Re-Check udev Status"
                        icon.name: "view-refresh"
                        font.pointSize: tab.app.smallPt
                        onClicked: {
                            if (controller) controller.recheckUdevStatus()
                        }
                    }

                    QQC2.Button {
                        text: "Install / Repair udev Rule (Polkit)"
                        icon.name: "dialog-password"
                        font.pointSize: tab.app.smallPt
                        onClicked: {
                            if (controller) controller.installUdevRuleInteractive()
                        }
                    }

                    QQC2.Button {
                        text: "Copy udev Setup Command"
                        icon.name: "edit-copy"
                        font.pointSize: tab.app.smallPt
                        onClicked: {
                            if (controller) controller.copyUdevCommandToClipboard()
                        }
                    }
                }
            }

            InfoCard {
                app: tab.app
                cardTitle: "Startup & System Tray Behavior (Instant Save)"
                cardSubtitle: "All changes apply and persist to ~/.config/dji-mic-tray/config.json and systemd user unit immediately."
                rightBadgeText: "INSTANT SAVE"
                rightBadgeColor: tab.app.okCol

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Start KDE DJI Mic Mini Control Automatically on Login"
                    boldLabel: true
                    checkedState: controller ? controller.startOnLogin : true
                    onToggledByUser: function(c) {
                        if (controller) controller.setStartOnLogin(c)
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Start Minimized to System Tray"
                    boldLabel: true
                    checkedState: controller ? controller.startMinimized : true
                    onToggledByUser: function(c) {
                        if (controller) controller.setStartMinimized(c)
                    }
                }
            }

            InfoCard {
                app: tab.app
                cardTitle: "Desktop Popup Alert Rules (Instant Save)"
                cardSubtitle: "Sends a KDE Plasma desktop popup notification if the DJI Mic Mini Receiver or Wireless Transmitter is not connected."

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Popup Alert when USB-C Receiver (2ca3:4011) is Unplugged / Disconnected"
                    checkedState: controller ? controller.alertOnRxDisconnect : true
                    onToggledByUser: function(c) {
                        if (controller) controller.setAlertOnRxDisconnect(c)
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Popup Alert when All Wireless Transmitters (TX1 & TX2) are Off / Unlinked"
                    checkedState: controller ? controller.alertOnTxDisconnect : true
                    onToggledByUser: function(c) {
                        if (controller) controller.setAlertOnTxDisconnect(c)
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Popup Alert when Transmitter Battery Drops Below Threshold"
                    checkedState: controller ? controller.alertOnLowBattery : true
                    onToggledByUser: function(c) {
                        if (controller) controller.setAlertOnLowBattery(c)
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: "Low Battery Alert Threshold (%)"
                        color: tab.app.fgColor
                        font.pointSize: tab.app.bodyPt
                        wrapMode: Text.Wrap
                    }
                    QQC2.SpinBox {
                        from: 5
                        to: 70
                        stepSize: 5
                        value: controller ? controller.lowBatteryThresholdPct : 25
                        font.pointSize: tab.app.smallPt
                        onValueModified: {
                            if (controller) controller.setLowBatteryThresholdPct(value)
                        }
                    }
                }

                Flow {
                    Layout.fillWidth: true
                    Layout.minimumHeight: implicitHeight
                    Layout.preferredHeight: implicitHeight
                    spacing: 10

                    QQC2.Button {
                        text: "Send Test Desktop Popup Alert Now"
                        icon.name: "preferences-desktop-notification"
                        font.pointSize: tab.app.smallPt
                        onClicked: {
                            if (controller) controller.sendTestAlert()
                        }
                    }

                    QQC2.Button {
                        text: "Configure KDE Application Notifications"
                        icon.name: "preferences-desktop-notification-bell"
                        font.pointSize: tab.app.smallPt
                        onClicked: {
                            if (controller) controller.openKdeNotificationsSettings()
                        }
                    }
                }
            }

            InfoCard {
                app: tab.app
                cardTitle: "KDE System Settings → Global Keyboard Shortcuts (kglobalaccel)"
                cardSubtitle: "15 global microphone, noise cancellation, and battery actions registered under 'KDE DJI Mic Mini Control' in KDE System Settings → Shortcuts"
                rightBadgeText: "15 SHORTCUTS"
                rightBadgeColor: tab.app.okCol

                QQC2.Label {
                    Layout.fillWidth: true
                    text: "Bind global hotkeys anywhere on your KDE Plasma 6 desktop to control your DJI Mic Mini even when this window is minimized:"
                    color: tab.app.fgColor
                    font.pointSize: tab.app.smallPt
                    wrapMode: Text.Wrap
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    text: "• Window, Mute & Status: Toggle Control Window • Show Receiver & Transmitter Battery Notification • Toggle System Microphone Mute"
                    color: tab.app.mutedFg
                    font.pointSize: tab.app.captionPt
                    wrapMode: Text.Wrap
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    text: "• Noise Cancellation & AI DSP: Toggle Hardware Noise Cancellation On/Off • Set Hardware NC: Basic / Strong • Cycle AI Noise Cancellation Mode • Toggle PipeWire AI Denoised Virtual Mic • Start 3-Second AI Room Noise Learning"
                    color: tab.app.mutedFg
                    font.pointSize: tab.app.captionPt
                    wrapMode: Text.Wrap
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    text: "• Hardware Filters & LEDs: Toggle 150 Hz Low-Cut Filter • Toggle Hardware Auto Clip Limiter • Toggle Audio Track Mode (Mono / Stereo) • Toggle -6 dB Safety Backup Track • Toggle Transmitter Status LEDs"
                    color: tab.app.mutedFg
                    font.pointSize: tab.app.captionPt
                    wrapMode: Text.Wrap
                }

                Flow {
                    Layout.fillWidth: true
                    Layout.minimumHeight: implicitHeight
                    Layout.preferredHeight: implicitHeight
                    spacing: 10

                    QQC2.Button {
                        text: "Open KDE System Settings → Shortcuts (kcmshell6 kcm_keys)"
                        icon.name: "configure-shortcuts"
                        font.pointSize: tab.app.smallPt
                        font.bold: true
                        onClicked: {
                            if (controller) controller.openKdeShortcutsSettings()
                        }
                    }
                }
            }

            InfoCard {
                app: tab.app
                cardTitle: "Appearance & UI Scaling (KDE Plasma Theme Integration)"
                cardSubtitle: "All settings apply and save immediately to ~/.config/dji-mic-tray/config.json"

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: "Application Color Scheme"
                        color: tab.app.fgColor
                        font.pointSize: tab.app.bodyPt
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
                        font.pointSize: tab.app.smallPt
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
                        color: tab.app.fgColor
                        font.pointSize: tab.app.bodyPt
                        wrapMode: Text.Wrap
                    }
                    QQC2.SpinBox {
                        from: 75
                        to: 175
                        stepSize: 5
                        value: controller ? controller.uiZoomPercent : 100
                        font.pointSize: tab.app.smallPt
                        onValueModified: {
                            if (controller) controller.setUiZoomPercent(value)
                        }
                    }
                }
            }

            InfoCard {
                app: tab.app
                cardTitle: "Recent Connection & Alert Event Log"
                cardSubtitle: "Timestamped history of DJI Mic Mini receiver, transmitter, and battery events"

                QQC2.Label {
                    Layout.fillWidth: true
                    text: (controller && controller.eventLogText.length > 0)
                          ? controller.eventLogText
                          : "No disconnection or battery alerts recorded in this session."
                    color: tab.app.fgColor
                    font.pointSize: tab.app.smallPt
                    font.family: "monospace"
                    wrapMode: Text.WrapAnywhere
                }
            }
        }
    }
}
