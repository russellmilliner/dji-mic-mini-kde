import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

RowLayout {
    id: kvRow
    required property var app
    property string keyText: ""
    property string valText: ""
    property bool monoValue: false
    property color valColor: app.fgColor

    Layout.fillWidth: true
    spacing: 10

    QQC2.Label {
        Layout.preferredWidth: Math.min(210, kvRow.app.width * 0.27)
        text: kvRow.keyText
        color: kvRow.app.mutedFg
        font.pointSize: kvRow.app.smallPt
        wrapMode: Text.Wrap
    }

    QQC2.Label {
        Layout.fillWidth: true
        text: kvRow.valText
        color: kvRow.valColor
        font.pointSize: kvRow.app.smallPt
        font.family: kvRow.monoValue ? "monospace" : Kirigami.Theme.defaultFont.family
        font.bold: !kvRow.monoValue
        wrapMode: Text.WrapAnywhere
    }
}
