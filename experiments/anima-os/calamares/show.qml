import QtQuick 2.15
Rectangle {
    color: "#2E3440"
    Column {
        anchors.centerIn: parent
        spacing: 24
        Image { source: "logo.png"; width: 160; height: 160; anchors.horizontalCenter: parent.horizontalCenter }
        Text { text: "Anima.ing"; color: "#ECEFF4"; font.pixelSize: 32; anchors.horizontalCenter: parent.horizontalCenter }
        Text { text: "Giving this computer a new purpose."; color: "#ECEFF4"; font.pixelSize: 18; anchors.horizontalCenter: parent.horizontalCenter }
    }
}
