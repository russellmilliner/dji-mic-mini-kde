import QtQuick
import QtQuick.Controls as QQC2

Rectangle {
    id: badge
    required property var app
    property string badgeText: ""
    property color badgeColor: app.accentCol

    radius: 6
    color: Qt.alpha(badgeColor, 0.16)
    border.width: 1
    border.color: Qt.alpha(badgeColor, 0.55)
    implicitWidth: Math.min(240, badgeLbl.implicitWidth + 16)
    implicitHeight: badgeLbl.implicitHeight + 8

    QQC2.Label {
        id: badgeLbl
        anchors.centerIn: parent
        width: parent.width - 12
        horizontalAlignment: Text.AlignHCenter
        text: badge.badgeText
        color: badge.badgeColor
        font.pointSize: badge.app.captionPt
        font.bold: true
        elide: Text.ElideRight
    }
}
