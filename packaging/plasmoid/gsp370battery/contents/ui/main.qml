import QtQuick
import QtQuick.Layouts

import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import org.kde.plasma.plasma5support as P5Support

PlasmoidItem {
    id: root

    property int percent: -1
    property bool busy: false

    Plasmoid.title: "GSP 370 Battery"
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground
    Plasmoid.status: percent >= 0 ? PlasmaCore.Types.ActiveStatus : PlasmaCore.Types.PassiveStatus

    toolTipMainText: "GSP 370"
    toolTipSubText: percent >= 0 ? i18n("Battery: %1%", percent) : i18n("Headset off")

    compactRepresentation: Item {
        implicitWidth: label.visible ? label.implicitWidth : 0
        implicitHeight: label.visible ? label.implicitHeight : 0
        Layout.minimumWidth: implicitWidth
        Layout.preferredWidth: implicitWidth
        Layout.maximumWidth: implicitWidth
        Layout.minimumHeight: implicitHeight
        Layout.preferredHeight: implicitHeight
        Layout.maximumHeight: implicitHeight

        PlasmaComponents3.Label {
            id: label
            anchors.centerIn: parent
            visible: root.percent >= 0
            text: visible ? root.percent + "%" : ""
        }

        MouseArea {
            anchors.fill: parent
            enabled: label.visible
            onClicked: root.refresh()
        }
    }

    fullRepresentation: Item {
        implicitWidth: Kirigami.Units.gridUnit * 8
        implicitHeight: Kirigami.Units.gridUnit * 3
        PlasmaComponents3.Label {
            anchors.centerIn: parent
            text: root.percent >= 0 ? i18n("GSP 370: %1%", root.percent) : i18n("GSP 370: headset off")
        }
    }

    P5Support.DataSource {
        id: batterySource
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => {
            const output = String(data["stdout"] ?? "").trim();
            const value = parseInt(output, 10);
            if (Number.isFinite(value) && value >= 0 && value <= 100) {
                root.percent = value;
            } else {
                root.percent = -1;
            }
            root.busy = false;
            batterySource.disconnectSource(source);
        }
    }

    Timer {
        interval: 60000
        repeat: true
        running: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }

    function refresh() {
        if (root.busy) {
            return;
        }
        root.busy = true;
        batterySource.connectSource("gsp370-tray --battery");
    }
}
