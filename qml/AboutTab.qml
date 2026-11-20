import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

Item {
    id: tab
    required property var app

    Flickable {
        id: aboutFlick
        objectName: "aboutFlick"
        anchors.fill: parent
        anchors.margins: 12
        contentWidth: width - 16
        contentHeight: aboutCol.implicitHeight + 16
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        QQC2.ScrollBar.vertical: HighContrastScrollBar { app: tab.app }

        ColumnLayout {
            id: aboutCol
            width: aboutFlick.width - 16
            spacing: 12

            InfoCard {
                app: tab.app
                objectName: "aboutHeaderCard"
                cardTitle: "KDE DJI Mic Mini Control"
                cardSubtitle: "Version " + tab.app.appVer + " • KDE Plasma 6 Kirigami + libusb-1.0 DUML v1/v2 + PipeWire/ALSA"
                rightBadgeText: "v" + tab.app.appVer
                rightBadgeColor: tab.app.accentCol

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    QQC2.Label {
                        Layout.fillWidth: true
                        text: "Real-time system tray battery gauge, live microphone VU meter, hardware settings controller, USB/HID/DUML protocol inspector, and automatic desktop disconnection popup monitor for the DJI Mic Mini (2ca3:4011) on Linux. Contains zero tracking and zero advertisements — forever."
                        color: tab.app.fgColor
                        font.pointSize: tab.app.bodyPt
                        wrapMode: Text.Wrap
                    }

                    KofiPillButton {
                        app: tab.app
                        objectName: "aboutKofiButton"
                        buttonText: "Buy Me a Coffee"
                    }
                }
            }

            InfoCard {
                app: tab.app
                objectName: "aboutLinksCard"
                cardTitle: "Author, Repository & Support Links"
                cardSubtitle: "Click any link below to open in your default web browser"

                ClickableLinkRow {
                    app: tab.app
                    keyText: "Author:"
                    linkText: "Russell Milliner (https://github.com/russellmilliner)"
                    linkUrl: "https://github.com/russellmilliner"
                }

                ClickableLinkRow {
                    app: tab.app
                    keyText: "GitHub Repository:"
                    linkText: "https://github.com/russellmilliner/dji-mic-mini-kde"
                    linkUrl: "https://github.com/russellmilliner/dji-mic-mini-kde"
                }

                ClickableLinkRow {
                    app: tab.app
                    keyText: "Buy Me a Coffee (Ko-fi):"
                    linkText: "https://ko-fi.com/russellmilliner"
                    linkUrl: "https://ko-fi.com/russellmilliner"
                }

                ClickableLinkRow {
                    app: tab.app
                    keyText: "License:"
                    linkText: "MIT License (https://opensource.org/licenses/MIT)"
                    linkUrl: "https://opensource.org/licenses/MIT"
                }

                ClickableLinkRow {
                    app: tab.app
                    keyText: "Protocol Attribution:"
                    linkText: "DJI DUML USB Interface 6 interoperability notes (piedol/lavdeck & caballeropedro/dji-mic)"
                    linkUrl: "https://github.com/piedol/lavdeck/blob/main/docs/PROTOCOL.md"
                }

                Flow {
                    Layout.fillWidth: true
                    Layout.minimumHeight: implicitHeight
                    Layout.preferredHeight: implicitHeight
                    spacing: 10

                    QQC2.Button {
                        icon.name: "globe"
                        text: "GitHub Profile"
                        font.pointSize: tab.app.smallPt
                        onClicked: Qt.openUrlExternally("https://github.com/russellmilliner")
                    }

                    QQC2.Button {
                        icon.name: "favorite"
                        text: "Support on Ko-fi (Buy Me a Coffee)"
                        font.pointSize: tab.app.smallPt
                        onClicked: Qt.openUrlExternally("https://ko-fi.com/russellmilliner")
                    }

                    QQC2.Button {
                        icon.name: "license"
                        text: "MIT License"
                        font.pointSize: tab.app.smallPt
                        onClicked: Qt.openUrlExternally("https://opensource.org/licenses/MIT")
                    }
                }
            }

            InfoCard {
                app: tab.app
                cardTitle: "Security & Architecture Highlights"
                cardSubtitle: "Hardened local IPC, closed DUML command allowlist, and non-root uaccess USB permissions"

                KeyValueRow {
                    app: tab.app
                    keyText: "Closed Command Allowlist:"
                    valText: "Strictly rejects corrupted CRC-16 frames and blocks firmware-update (0x55/0x35) and reboot (0x52) commands; only verified audio/power TLV settings and read-only queries are permitted."
                }
                KeyValueRow {
                    app: tab.app
                    keyText: "Authenticated Local IPC:"
                    valText: "Unix socket ($XDG_RUNTIME_DIR/dji-mic-tray.sock) uses 0600 permissions, read/write timeouts, bounded reads, and SO_PEERCRED UID verification on every connection."
                }
                KeyValueRow {
                    app: tab.app
                    keyText: "Non-Blocking Worker & Subprocess Safety:"
                    valText: "USB polling/writes and PipeWire probes run on a dedicated background worker thread; native D-Bus handles desktop alerts and all subprocesses enforce explicit wall-clock timeouts (shell=false)."
                }
            }
        }
    }
}
