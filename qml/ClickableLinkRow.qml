import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

RowLayout {
    id: linkRow
    required property var app
    property string keyText: ""
    property string linkText: ""
    property string linkUrl: ""

    Layout.fillWidth: true
    spacing: 10

    QQC2.Label {
        Layout.preferredWidth: Math.min(210, linkRow.app.width * 0.27)
        text: linkRow.keyText
        color: linkRow.app.mutedFg
        font.pointSize: linkRow.app.smallPt
        font.bold: true
        wrapMode: Text.Wrap
    }

    QQC2.Label {
        Layout.fillWidth: true
        text: linkRow.linkText
        color: linkRow.app.linkCol
        font.pointSize: linkRow.app.smallPt
        font.underline: linkMouse.containsMouse
        wrapMode: Text.WrapAnywhere

        MouseArea {
            id: linkMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: Qt.openUrlExternally(linkRow.linkUrl)
        }
    }
}
