import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

Rectangle {
    id: pill
    required property var app
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
            font.pointSize: pill.app.smallPt
        }

        QQC2.Label {
            text: pill.buttonText
            font.bold: true
            font.pointSize: pill.app.captionPt
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
