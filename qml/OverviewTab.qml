import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

Item {
    id: tab
    required property var app

    Flickable {
        id: overviewFlick
        objectName: "overviewFlick"
        anchors.fill: parent
        anchors.margins: 12
        contentWidth: width - 16
        contentHeight: overviewCol.implicitHeight + 16
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        QQC2.ScrollBar.vertical: HighContrastScrollBar { app: tab.app }

        ColumnLayout {
            id: overviewCol
            width: overviewFlick.width - 16
            spacing: 12

            // udev Rule Missing Warning Banner
            InfoCard {
                app: tab.app
                visible: controller ? !controller.udevReady : false
                cardTitle: "USB udev Rule Setup Required for Non-Root Telemetry"
                cardSubtitle: controller ? controller.udevSummary : "Install /etc/udev/rules.d/99-dji-mic.rules to allow non-root access to USB Interface 6."
                rightBadgeText: "UDEV SETUP"
                rightBadgeColor: tab.app.warnCol

                Flow {
                    Layout.fillWidth: true
                    Layout.minimumHeight: implicitHeight
                    Layout.preferredHeight: implicitHeight
                    spacing: 10

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

            // Connection Alert Warning Banner when disconnected
            InfoCard {
                app: tab.app
                visible: controller ? (!controller.rxConnected || controller.txConnectedCount === 0) : false
                cardTitle: (controller && !controller.rxConnected)
                           ? "DJI Mic Mini USB-C Receiver Not Connected"
                           : "No DJI Mic Mini Wireless Transmitter Linked"
                cardSubtitle: (controller && !controller.rxConnected)
                              ? "Plug the DJI Mic Mini Receiver (2ca3:4011) into a USB-C port. Desktop popup alerts are active."
                              : "The USB-C Receiver is plugged in, but both TX1 and TX2 are powered off or inside the charging case."
                rightBadgeText: "ALERT ACTIVE"
                rightBadgeColor: tab.app.errCol
            }

            // Transmitters Side-by-Side or Stacked depending on width
            GridLayout {
                Layout.fillWidth: true
                columns: tab.app.width >= 900 ? 2 : 1
                columnSpacing: 12
                rowSpacing: 12

                TransmitterCard {
                    app: tab.app
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
                    app: tab.app
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
                app: tab.app
                cardTitle: "USB-C Receiver (RX) & System Audio Summary"
                cardSubtitle: controller ? controller.rxSubtitle : "Receiver hardware & Linux PipeWire status"
                rightBadgeText: (controller && controller.rxConnected)
                                ? ("Protocol " + controller.protocolDialect.toUpperCase())
                                : "USB OFFLINE"
                rightBadgeColor: (controller && controller.rxConnected) ? tab.app.okCol : tab.app.errCol

                KeyValueRow { app: tab.app; keyText: "Receiver Hardware Model:"; valText: controller ? controller.rxHardwareModel : "—" }
                KeyValueRow { app: tab.app; keyText: "Receiver Serial Number:"; valText: controller ? controller.rxSerial : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Receiver Firmware / HW Rev:"; valText: controller ? controller.rxFirmwareAndRev : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Receiver Gain Step / Dial:"; valText: controller ? controller.rxGainSummary : "—" }
                KeyValueRow { app: tab.app; keyText: "Audio Channel Track Mode:"; valText: controller ? controller.settingStereo.toUpperCase() : "MONO" }
                KeyValueRow { app: tab.app; keyText: "Noise Cancellation:"; valText: controller ? controller.ncOverallSummary : "—" }
                KeyValueRow {
                    app: tab.app
                    keyText: "Clip Limiter & Safety (-6dB):"
                    valText: controller
                             ? ("Clip Limiter: " + controller.settingClipLimiter.toUpperCase() + " • Safety Track: " + controller.settingSafetyTrack.toUpperCase())
                             : "—"
                }
                KeyValueRow {
                    app: tab.app
                    keyText: "Power Management:"
                    valText: controller
                             ? ("RX 15m Auto-Off: " + controller.settingRxAutoOff.toUpperCase() + " • Camera Sync: " + controller.settingCameraPower.toUpperCase() + " • Plug-Free Speaker: " + controller.settingPlugFree.toUpperCase())
                             : "—"
                }
                KeyValueRow {
                    app: tab.app
                    keyText: "Linux PipeWire / ALSA Capture:"
                    valText: controller ? controller.linuxAudioSummary : "—"
                    valColor: (controller && controller.alsaMuted) ? tab.app.errCol : tab.app.fgColor
                }
            }
        }
    }
}
