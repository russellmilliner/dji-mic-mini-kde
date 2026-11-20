import QtQuick
import QtQuick.Controls as QQC2

QQC2.ScrollBar {
    id: hcBar
    required property var app
    orientation: Qt.Vertical
    policy: size < 1.0 ? QQC2.ScrollBar.AlwaysOn : QQC2.ScrollBar.AsNeeded
    minimumSize: 0.14
    implicitWidth: 12
    padding: 2

    background: Rectangle {
        implicitWidth: 12
        radius: 6
        color: Qt.alpha(hcBar.app.fgColor, 0.14)
        border.width: 1
        border.color: Qt.alpha(hcBar.app.fgColor, 0.28)
    }

    contentItem: Rectangle {
        implicitWidth: 8
        radius: 4
        color: hcBar.pressed
               ? hcBar.app.accentCol
               : (hcBar.hovered
                  ? Qt.lighter(hcBar.app.accentCol, 1.15)
                  : Qt.alpha(hcBar.app.fgColor, 0.65))
    }
}
