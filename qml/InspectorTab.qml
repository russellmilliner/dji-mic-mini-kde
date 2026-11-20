import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

Item {
    id: tab
    required property var app

    Flickable {
        id: inspectorFlick
        objectName: "inspectorFlick"
        anchors.fill: parent
        anchors.margins: 12
        contentWidth: width - 16
        contentHeight: inspectorCol.implicitHeight + 16
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        QQC2.ScrollBar.vertical: HighContrastScrollBar { app: tab.app }

        ColumnLayout {
            id: inspectorCol
            width: inspectorFlick.width - 16
            spacing: 12

            InfoCard {
                app: tab.app
                cardTitle: "Linux USB Device & Sysfs Topology (VID:PID 2ca3:4011)"
                cardSubtitle: "All kernel USB device descriptor attributes exposed under /sys/bus/usb/devices"
                rightBadgeText: (controller && controller.usbPresent) ? "USB PRESENT" : "NOT PRESENT"
                rightBadgeColor: (controller && controller.usbPresent) ? tab.app.okCol : tab.app.errCol

                KeyValueRow { app: tab.app; keyText: "Vendor / Product ID:"; valText: controller ? controller.usbVidPid : "2ca3:4011"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Manufacturer / Product:"; valText: controller ? controller.usbManufacturerProduct : "—" }
                KeyValueRow { app: tab.app; keyText: "USB Descriptor Serial:"; valText: controller ? controller.usbDescriptorSerial : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "USB Bus / Device / Port:"; valText: controller ? controller.usbTopology : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Negotiated Speed / Power:"; valText: controller ? controller.usbSpeedAndPower : "—" }
                KeyValueRow { app: tab.app; keyText: "USBFS Device Node:"; valText: controller ? controller.usbFsNode : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Kernel Sysfs Path:"; valText: controller ? controller.usbSysfsPath : "—"; monoValue: true }
            }

            InfoCard {
                app: tab.app
                cardTitle: "USB HID & Service Interface (Interface 0) + Audio Stack (Interfaces 1 & 3)"
                cardSubtitle: "Consumer media key interface (Report 0x06), Vendor Service (Report 0x55), and ALSA/PipeWire nodes"

                KeyValueRow { app: tab.app; keyText: "HID Raw Device Node:"; valText: controller ? controller.hidNodeAndPhys : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "HID Report 0x55 Telemetry:"; valText: controller ? controller.hidServiceSummary : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "ALSA Sound Card:"; valText: controller ? controller.alsaCardDetails : "—" }
                KeyValueRow { app: tab.app; keyText: "PipeWire Source Node:"; valText: controller ? controller.pipewireNodeDetails : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Audio Format & State:"; valText: controller ? controller.pipewireFormatState : "—"; monoValue: true }
            }

            InfoCard {
                app: tab.app
                cardTitle: "DJI DUML Wire Protocol Inspector (Interface 6 — Bulk IN 0x86 / Bulk OUT 0x06)"
                cardSubtitle: "Live hexadecimal frame capture and protocol telemetry"
                rightBadgeText: controller ? ("Seq #" + controller.lastSequence + " (" + controller.heartbeatCount + " frames)") : "0 frames"
                rightBadgeColor: tab.app.accentCol

                KeyValueRow { app: tab.app; keyText: "Protocol Dialect:"; valText: controller ? controller.protocolDialect.toUpperCase() : "V1"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Last 0x5B/0x03 Heartbeat:"; valText: controller ? controller.lastHeartbeatHex : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Last 0x5B/0x05 Aux Frame:"; valText: controller ? controller.aux5b05Hex : "—"; monoValue: true }
                KeyValueRow { app: tab.app; keyText: "Last 0x04/0x74 Link Frame:"; valText: controller ? controller.aux0474Hex : "—"; monoValue: true }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10

                    QQC2.Button {
                        text: "Copy Full JSON Telemetry Snapshot"
                        icon.name: "edit-copy"
                        font.pointSize: tab.app.smallPt
                        onClicked: {
                            if (controller) controller.copyJsonSnapshotToClipboard()
                        }
                    }

                    QQC2.Label {
                        Layout.fillWidth: true
                        text: controller ? controller.lastActionMessage : ""
                        color: tab.app.okCol
                        font.pointSize: tab.app.captionPt
                        elide: Text.ElideRight
                    }
                }
            }
        }
    }
}
