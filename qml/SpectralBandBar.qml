import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

ColumnLayout {
    id: bandBar
    required property var app
    property string bandLabel: ""
    property string bandDb: "-55.0"

    readonly property real norm: Math.max(0.04, Math.min(1.0, (Number(bandDb) + 85.0) / 65.0))

    Layout.fillWidth: true
    spacing: 3

    RowLayout {
        Layout.fillWidth: true

        QQC2.Label {
            Layout.fillWidth: true
            text: bandBar.bandLabel
            color: bandBar.app.mutedFg
            font.pointSize: bandBar.app.captionPt
            elide: Text.ElideRight
        }

        QQC2.Label {
            text: bandBar.bandDb + " dBFS"
            color: bandBar.app.fgColor
            font.pointSize: bandBar.app.captionPt
            font.family: "monospace"
            font.bold: true
        }
    }

    Rectangle {
        Layout.fillWidth: true
        height: Math.round(8 * bandBar.app.zoomScale)
        radius: 4
        color: Qt.alpha(bandBar.app.fgColor, 0.12)
        border.width: 1
        border.color: Qt.alpha(bandBar.app.fgColor, 0.22)

        Rectangle {
            width: Math.round(parent.width * bandBar.norm)
            height: parent.height
            radius: 4
            color: bandBar.norm > 0.68 ? bandBar.app.errCol : (bandBar.norm > 0.48 ? bandBar.app.warnCol : bandBar.app.okCol)
        }
    }
}
