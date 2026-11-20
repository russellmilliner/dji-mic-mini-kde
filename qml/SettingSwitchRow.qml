import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

RowLayout {
    id: switchRow
    required property var app
    property string labelText: ""
    property bool boldLabel: false
    property bool checkedState: false
    property bool controlEnabled: true
    property string switchText: ""
    signal toggledByUser(bool nextChecked)

    Layout.fillWidth: true
    spacing: 12

    QQC2.Label {
        Layout.fillWidth: true
        text: switchRow.labelText
        color: switchRow.app.fgColor
        font.pointSize: switchRow.app.bodyPt
        font.bold: switchRow.boldLabel
        wrapMode: Text.Wrap
    }

    QQC2.Switch {
        checked: switchRow.checkedState
        enabled: switchRow.controlEnabled
        text: switchRow.switchText.length > 0 ? switchRow.switchText : (checked ? "Enabled" : "Disabled")
        font.pointSize: switchRow.app.smallPt
        onToggled: switchRow.toggledByUser(checked)
    }
}
