import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

InfoCard {
    id: txCard
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
                     ? app.errCol
                     : (txBatteryPct >= 40 ? app.okCol : (txBatteryPct >= 20 ? app.warnCol : app.errCol))

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
                    color: txCard.app.mutedFg
                    font.pointSize: txCard.app.captionPt
                }
                QQC2.Label {
                    text: txCard.txConnected
                          ? (txCard.txBatteryPct + "% • Gauge " + txCard.txBatteryCode + "/7" + (txCard.txCharging ? " (Charging ⚡)" : ""))
                          : "Offline"
                    color: txCard.app.fgColor
                    font.pointSize: txCard.app.captionPt
                    font.bold: true
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: Math.round(10 * txCard.app.zoomScale)
                radius: 5
                color: Qt.alpha(txCard.app.fgColor, 0.12)
                border.width: 1
                border.color: Qt.alpha(txCard.app.fgColor, 0.22)

                Rectangle {
                    width: txCard.txConnected ? Math.max(6, parent.width * Math.min(1.0, txCard.txBatteryPct / 100.0)) : 0
                    height: parent.height
                    radius: 5
                    color: txCard.txBatteryPct >= 40 ? txCard.app.okCol : (txCard.txBatteryPct >= 20 ? txCard.app.warnCol : txCard.app.errCol)
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
                    color: txCard.app.mutedFg
                    font.pointSize: txCard.app.captionPt
                }
                QQC2.Label {
                    text: txCard.txConnected ? (txCard.txVuLevel + " / 255 (" + Math.round(Math.min(100, txCard.txVuLevel / 1.2)) + "%)") : "0 / 255"
                    color: txCard.app.fgColor
                    font.pointSize: txCard.app.captionPt
                    font.family: "monospace"
                    font.bold: true
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: Math.round(10 * txCard.app.zoomScale)
                radius: 5
                color: Qt.alpha(txCard.app.fgColor, 0.12)
                border.width: 1
                border.color: Qt.alpha(txCard.app.fgColor, 0.22)

                Rectangle {
                    readonly property real frac: txCard.txConnected ? Math.min(1.0, Math.max(0.0, txCard.txVuLevel / 120.0)) : 0.0
                    width: Math.round(parent.width * frac)
                    height: parent.height
                    radius: 5
                    color: frac > 0.85 ? txCard.app.errCol : (frac > 0.60 ? txCard.app.warnCol : txCard.app.okCol)
                }
            }
        }

        KeyValueRow { app: txCard.app; keyText: "Serial Number:"; valText: txCard.txConnected ? txCard.txSerial : "—"; monoValue: true }
        KeyValueRow { app: txCard.app; keyText: "Firmware Version:"; valText: txCard.txConnected ? txCard.txFirmware : "—"; monoValue: true }
        KeyValueRow { app: txCard.app; keyText: "Noise Cancellation:"; valText: txCard.txConnected ? txCard.txNcState : "—" }
        KeyValueRow { app: txCard.app; keyText: "Low-Cut / Voice Tone:"; valText: txCard.txConnected ? ("Low-Cut: " + txCard.txLowCut + " • Tone: " + txCard.txTone) : "—" }
        KeyValueRow { app: txCard.app; keyText: "LEDs & 15m Auto-Off:"; valText: txCard.txConnected ? ("Status LEDs: " + txCard.txLeds + " • Auto-Off: " + txCard.txAutoOff) : "—" }
        KeyValueRow { app: txCard.app; keyText: "Raw DUML Slot Bytes:"; valText: txCard.txConnected ? txCard.txRawHex : "00 40 00 00 00 00 00 00 00 (Absent Stub)"; monoValue: true }
    }
}
