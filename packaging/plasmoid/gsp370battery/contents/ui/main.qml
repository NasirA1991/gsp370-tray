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
    property bool darkPanel: true
    property string linkState: "unknown"
    readonly property color labelColor: root.darkPanel ? "#f5f7fa" : "#1c1e22"

    Plasmoid.title: "GSP 370 Battery"
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground
    Plasmoid.status: percent >= 0 ? PlasmaCore.Types.ActiveStatus : PlasmaCore.Types.PassiveStatus

    toolTipMainText: "GSP 370"
    toolTipSubText: root.stateText()

    function stateText() {
        switch (root.linkState) {
        case "on":
            return root.percent >= 0 ? i18n("Battery: %1%", root.percent) : i18n("Connected");
        case "off":
            return i18n("Headset off");
        case "no-dongle":
            return i18n("Dongle not found");
        case "permission":
            return i18n("Serial permission denied");
        case "error":
            return i18n("Connection problem");
        default:
            return i18n("Starting…");
        }
    }

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
            color: root.labelColor
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
            text: i18n("GSP 370: %1", root.stateText())
            color: root.labelColor
        }
    }

    P5Support.DataSource {
        id: themeSource
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => {
            const output = String(data["stdout"] ?? "").trim();
            const channels = output.split(",").map(part => Number(part.trim()));
            if (channels.length >= 3 && channels.slice(0, 3).every(Number.isFinite)) {
                const luminance = 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
                root.darkPanel = luminance < 140;
            }
            themeSource.disconnectSource(source);
        }
    }

    P5Support.DataSource {
        id: batterySource
        engine: "executable"
        connectedSources: []
        onNewData: (source, data) => {
            let state = "unknown";
            let value = -1;
            String(data["stdout"] ?? "").split("\n").forEach(line => {
                const separator = line.indexOf("=");
                if (separator < 0) {
                    return;
                }
                const key = line.slice(0, separator).trim();
                const text = line.slice(separator + 1).trim();
                if (key === "state") {
                    state = text;
                } else if (key === "battery") {
                    const parsed = parseInt(text, 10);
                    if (Number.isFinite(parsed)) {
                        value = parsed;
                    }
                }
            });
            root.linkState = state;
            root.percent = state === "on" && value >= 0 && value <= 100 ? value : -1;
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

    Component.onCompleted: {
        themeSource.connectSource("kreadconfig6 --file kdeglobals --group Colors:Window --key BackgroundNormal");
    }

    function refresh() {
        if (root.busy) {
            return;
        }
        root.busy = true;
        batterySource.connectSource("sh -c 'for p in \"$HOME/.local/bin/gsp370-tray\" /usr/bin/gsp370-tray /usr/local/bin/gsp370-tray; do [ -x \"$p\" ] && exec \"$p\" --status; done; exit 1'");
    }
}
