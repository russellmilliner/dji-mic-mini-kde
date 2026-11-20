import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

RowLayout {
    id: comboRow
    required property var app
    property string labelText: ""
    property var optionsModel: []
    property int currentOptionIndex: 0
    property bool controlEnabled: true
    signal activatedIndex(int index)

    Layout.fillWidth: true
    spacing: 12

    QQC2.Label {
        Layout.fillWidth: true
        text: comboRow.labelText
        color: comboRow.app.fgColor
        font.pointSize: comboRow.app.bodyPt
        wrapMode: Text.Wrap
    }

    QQC2.ComboBox {
        model: comboRow.optionsModel
        currentIndex: comboRow.currentOptionIndex
        enabled: comboRow.controlEnabled
        font.pointSize: comboRow.app.smallPt
        onActivated: function(idx) {
            comboRow.activatedIndex(idx)
        }
    }
}
