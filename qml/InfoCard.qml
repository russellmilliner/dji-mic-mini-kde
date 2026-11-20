import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

Rectangle {
    id: card
    required property var app
    default property alias cardContent: innerCol.data
    property string cardTitle: ""
    property string cardSubtitle: ""
    property string rightBadgeText: ""
    property color rightBadgeColor: app.accentCol

    Layout.fillWidth: true
    radius: 8
    color: card.app.cardBg
    border.width: 1
    border.color: card.app.borderCol
    implicitHeight: innerCol.implicitHeight + 24

    ColumnLayout {
        id: innerCol
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            visible: card.cardTitle.length > 0
            spacing: 8

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Kirigami.Heading {
                    Layout.fillWidth: true
                    level: 3
                    text: card.cardTitle
                    color: card.app.fgColor
                    font.pointSize: card.app.subheadPt
                    font.bold: true
                    wrapMode: Text.Wrap
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    visible: card.cardSubtitle.length > 0
                    text: card.cardSubtitle
                    color: card.app.mutedFg
                    font.pointSize: card.app.captionPt
                    wrapMode: Text.Wrap
                }
            }

            StatusBadge {
                app: card.app
                visible: card.rightBadgeText.length > 0
                badgeText: card.rightBadgeText
                badgeColor: card.rightBadgeColor
            }
        }

        Kirigami.Separator {
            Layout.fillWidth: true
            visible: card.cardTitle.length > 0
        }
    }
}
