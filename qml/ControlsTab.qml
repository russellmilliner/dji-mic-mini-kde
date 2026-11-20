import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

Item {
    id: tab
    required property var app

    Flickable {
        id: controlsFlick
        objectName: "controlsFlick"
        anchors.fill: parent
        anchors.margins: 12
        contentWidth: width - 16
        contentHeight: controlsCol.implicitHeight + 16
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        QQC2.ScrollBar.vertical: HighContrastScrollBar { app: tab.app }

        ColumnLayout {
            id: controlsCol
            width: controlsFlick.width - 16
            spacing: 12

            InfoCard {
                app: tab.app
                cardTitle: "DJI Mic Mini Hardware Controls (Instant Apply over USB Interface 6)"
                cardSubtitle: "All changes are dispatched asynchronously via DUML 0x5B/0x01 and verified against the live receiver heartbeat."
                rightBadgeText: (controller && controller.rxConnected) ? "USB READY" : "RECEIVER OFFLINE"
                rightBadgeColor: (controller && controller.rxConnected) ? tab.app.okCol : tab.app.errCol

                SettingComboRow {
                    app: tab.app
                    labelText: "Noise Cancellation Strength (Basic vs Strong)"
                    optionsModel: ["basic", "strong"]
                    currentOptionIndex: (controller && controller.settingNcMode === "strong") ? 1 : 0
                    controlEnabled: controller ? controller.rxConnected : false
                    onActivatedIndex: function(idx) {
                        if (controller) controller.applyHardwareSetting("noise-cancel", idx === 1 ? "strong" : "basic")
                    }
                }

                SettingComboRow {
                    app: tab.app
                    labelText: "Audio Channel Routing (Mono Mixed vs Stereo Split TX1-L / TX2-R)"
                    optionsModel: ["mono", "stereo"]
                    currentOptionIndex: (controller && controller.settingStereo === "stereo") ? 1 : 0
                    controlEnabled: controller ? controller.rxConnected : false
                    onActivatedIndex: function(idx) {
                        if (controller) controller.applyHardwareSetting("stereo", idx === 1 ? "stereo" : "mono")
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Low-Cut High-Pass Filter (Attenuates rumble below 150 Hz on TX)"
                    checkedState: controller ? (controller.settingLowCut === "on") : false
                    controlEnabled: controller ? controller.rxConnected : false
                    onToggledByUser: function(c) {
                        if (controller) controller.applyHardwareSetting("low-cut", c ? "on" : "off")
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Safety Track (-6 dB Backup Channel in Mono Mode)"
                    checkedState: controller ? (controller.settingSafetyTrack === "on") : false
                    controlEnabled: controller ? controller.rxConnected : false
                    onToggledByUser: function(c) {
                        if (controller) controller.applyHardwareSetting("safety-track", c ? "on" : "off")
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Automatic Clipping Control / Peak Limiter"
                    checkedState: controller ? (controller.settingClipLimiter === "on") : false
                    controlEnabled: controller ? controller.rxConnected : false
                    onToggledByUser: function(c) {
                        if (controller) controller.applyHardwareSetting("clip-limiter", c ? "on" : "off")
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Transmitter Status Indicator LEDs (TX1 & TX2)"
                    checkedState: controller ? (controller.settingLeds === "on") : false
                    controlEnabled: controller ? controller.rxConnected : false
                    switchText: checkedState ? "LEDs On" : "LEDs Off"
                    onToggledByUser: function(c) {
                        if (controller) controller.applyHardwareSetting("mic-leds", c ? "on" : "off")
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Auto Power-Off after 15 Minutes of Inactivity (Receiver)"
                    checkedState: controller ? (controller.settingRxAutoOff === "on") : false
                    controlEnabled: controller ? controller.rxConnected : false
                    onToggledByUser: function(c) {
                        if (controller) controller.applyHardwareSetting("auto-off-15m", c ? "on" : "off")
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Synchronize Power On/Off with Host / Camera"
                    checkedState: controller ? (controller.settingCameraPower === "on") : false
                    controlEnabled: controller ? controller.rxConnected : false
                    onToggledByUser: function(c) {
                        if (controller) controller.applyHardwareSetting("camera-power", c ? "on" : "off")
                    }
                }

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Plug-Free External Speaker Playback Mode"
                    checkedState: controller ? (controller.settingPlugFree === "on") : false
                    controlEnabled: controller ? controller.rxConnected : false
                    onToggledByUser: function(c) {
                        if (controller) controller.applyHardwareSetting("plug-free", c ? "on" : "off")
                    }
                }
            }

            // Linux ALSA / PipeWire Capture Controls Card
            InfoCard {
                app: tab.app
                cardTitle: "Linux ALSA & PipeWire Capture Level Controls"
                cardSubtitle: controller ? controller.linuxAudioSummary : "ALSA / PipeWire source controls"
                rightBadgeText: (controller && controller.alsaMuted) ? "MUTED" : ("VOL " + (controller ? controller.alsaVolumePct : 100) + "%")
                rightBadgeColor: (controller && controller.alsaMuted) ? tab.app.errCol : tab.app.okCol

                SettingSwitchRow {
                    app: tab.app
                    labelText: "Microphone Capture Mute State"
                    checkedState: controller ? !controller.alsaMuted : true
                    switchText: checkedState ? "Unmuted (Live)" : "Muted"
                    onToggledByUser: function(c) {
                        if (controller) controller.setSystemMicMute(!c)
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12

                    QQC2.Label {
                        Layout.preferredWidth: Math.round(180 * tab.app.zoomScale)
                        text: "Capture Volume (%)"
                        color: tab.app.fgColor
                        font.pointSize: tab.app.bodyPt
                        wrapMode: Text.Wrap
                    }

                    QQC2.Slider {
                        id: volSlider
                        Layout.fillWidth: true
                        from: 0
                        to: 100
                        stepSize: 5
                        value: controller ? controller.alsaVolumePct : 100
                        onMoved: {
                            if (controller) controller.setSystemMicVolume(Math.round(value))
                        }
                    }

                    QQC2.Label {
                        Layout.preferredWidth: Math.round(64 * tab.app.zoomScale)
                        horizontalAlignment: Text.AlignRight
                        text: Math.round(volSlider.value) + "%"
                        color: tab.app.fgColor
                        font.pointSize: tab.app.smallPt
                        font.bold: true
                    }
                }
            }

            // AI Noise Cancellation & Ambient Room Noise Learning Card
            InfoCard {
                app: tab.app
                cardTitle: "AI Noise Cancellation & Ambient Room Noise Learning"
                cardSubtitle: controller
                              ? ("RNNoise Neural GRU + 4-Band Spectral & Tonal Hum Profiler • Auto-Calibrates TX/RX DSP • " + controller.noiseProfileSource)
                              : "RNNoise Neural GRU + 4-Band Spectral & Tonal Hum Profiler"
                rightBadgeText: {
                    if (!controller) return "AI READY"
                    if (controller.noiseLearnActive) return "LEARNING " + controller.noiseLearnProgressPct + "%"
                    if (controller.aiNcMode === "off") return "AI NC OFF"
                    if (controller.aiNcMode === "adaptive") return "AUTO-LEARN (#" + controller.adaptivePasses + ")"
                    return controller.noiseProfileLearned ? "PROFILE LEARNED" : "BASELINE AI"
                }
                rightBadgeColor: {
                    if (!controller) return tab.app.accentCol
                    if (controller.noiseLearnActive) return tab.app.warnCol
                    if (controller.aiNcMode === "off") return tab.app.errCol
                    return controller.noiseProfileLearned ? tab.app.okCol : tab.app.accentCol
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: tab.app.width >= 960 ? 2 : 1
                    columnSpacing: 12
                    rowSpacing: 8

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        QQC2.Label {
                            Layout.preferredWidth: Math.round(145 * tab.app.zoomScale)
                            text: "AI Denoise Mode:"
                            color: tab.app.mutedFg
                            font.pointSize: tab.app.smallPt
                            wrapMode: Text.Wrap
                        }

                        QQC2.ComboBox {
                            Layout.fillWidth: true
                            font.pointSize: tab.app.smallPt
                            model: [
                                "Off (Manual Hardware DSP Only)",
                                "Hardware DSP Only (Basic / Strong)",
                                "AI Learned Profile + Auto-Tune TX/RX DSP",
                                "Continuous Adaptive Auto-Learn + AI Denoise"
                            ]
                            currentIndex: {
                                var m = controller ? controller.aiNcMode : "learned_ai"
                                var keys = ["off", "hardware", "learned_ai", "adaptive"]
                                var idx = keys.indexOf(m)
                                return idx >= 0 ? idx : 2
                            }
                            onActivated: function(idx) {
                                var keys = ["off", "hardware", "learned_ai", "adaptive"]
                                if (controller) controller.setAiNcMode(keys[idx])
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        QQC2.Label {
                            Layout.preferredWidth: Math.round(145 * tab.app.zoomScale)
                            text: "Suppression Strength:"
                            color: tab.app.mutedFg
                            font.pointSize: tab.app.smallPt
                            wrapMode: Text.Wrap
                        }

                        QQC2.Slider {
                            Layout.fillWidth: true
                            from: 0
                            to: 100
                            stepSize: 5
                            value: controller ? controller.aiNcStrengthPct : 80
                            onMoved: {
                                if (controller) controller.setAiNcStrength(Math.round(value))
                            }
                        }

                        QQC2.Label {
                            Layout.preferredWidth: Math.round(48 * tab.app.zoomScale)
                            horizontalAlignment: Text.AlignRight
                            text: (controller ? controller.aiNcStrengthPct : 80) + "%"
                            color: tab.app.fgColor
                            font.pointSize: tab.app.smallPt
                            font.bold: true
                        }
                    }
                }

                QQC2.Switch {
                    Layout.fillWidth: true
                    text: controller && controller.aiNcVirtualMicActive
                          ? "PipeWire WebRTC AI Virtual Mic ('dji_ai_denoise_mic' — ACTIVE in PipeWire)"
                          : "Enable PipeWire WebRTC AI Virtual Microphone ('dji_ai_denoise_mic' for OBS / Zoom / Teams)"
                    font.pointSize: tab.app.smallPt
                    checked: controller ? controller.aiNcVirtualMic : false
                    onToggled: {
                        if (controller) controller.setAiNcVirtualMic(checked)
                    }
                }

                Flow {
                    Layout.fillWidth: true
                    Layout.minimumHeight: implicitHeight
                    Layout.preferredHeight: implicitHeight
                    spacing: 8

                    QQC2.Button {
                        text: (controller && controller.noiseLearnActive)
                              ? ("Sampling Room Acoustics (" + controller.noiseLearnProgressPct + "%)...")
                              : "🎙️ Learn Room Noise & Auto-Calibrate TX/RX (3s Sample)"
                        icon.name: "audio-input-microphone"
                        font.pointSize: tab.app.smallPt
                        highlighted: true
                        enabled: controller ? !controller.noiseLearnActive : false
                        onClicked: {
                            if (controller) controller.learnNoiseProfile()
                        }
                    }

                    QQC2.Button {
                        text: "Reset Learned Profile"
                        icon.name: "edit-reset"
                        font.pointSize: tab.app.smallPt
                        enabled: controller ? (!controller.noiseLearnActive && controller.noiseProfileLearned) : false
                        onClicked: {
                            if (controller) controller.resetNoiseProfile()
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    visible: controller ? (controller.noiseLearnActive || controller.noiseLearnProgressPct > 0) : false
                    height: Math.round(10 * tab.app.zoomScale)
                    radius: 5
                    color: Qt.alpha(tab.app.fgColor, 0.12)
                    border.width: 1
                    border.color: Qt.alpha(tab.app.fgColor, 0.24)

                    Rectangle {
                        width: Math.max(6, Math.round(parent.width * Math.min(1.0, (controller ? controller.noiseLearnProgressPct : 0) / 100.0)))
                        height: parent.height
                        radius: 5
                        color: (controller && controller.noiseLearnActive) ? tab.app.warnCol : tab.app.okCol
                    }
                }

                QQC2.Label {
                    Layout.fillWidth: true
                    text: controller ? controller.noiseLearnStatus : "Ready to sample ambient room acoustics."
                    color: (controller && controller.noiseLearnActive) ? tab.app.warnCol : tab.app.mutedFg
                    font.pointSize: tab.app.captionPt
                    font.bold: controller ? controller.noiseLearnActive : false
                    wrapMode: Text.Wrap
                }

                Kirigami.Separator { Layout.fillWidth: true }

                GridLayout {
                    Layout.fillWidth: true
                    columns: tab.app.width >= 900 ? 2 : 1
                    columnSpacing: 14
                    rowSpacing: 6

                    SpectralBandBar {
                        app: tab.app
                        bandLabel: "Sub / HVAC Rumble (20–140 Hz)"
                        bandDb: controller ? controller.bandLowRumbleStr : "-55.0"
                    }
                    SpectralBandBar {
                        app: tab.app
                        bandLabel: "PC Fan / Mains Hum (140–550 Hz)"
                        bandDb: controller ? controller.bandFanHumStr : "-51.0"
                    }
                    SpectralBandBar {
                        app: tab.app
                        bandLabel: "Mid Room / Reflections (550–2.8kHz)"
                        bandDb: controller ? controller.bandMidRoomStr : "-57.0"
                    }
                    SpectralBandBar {
                        app: tab.app
                        bandLabel: "High Preamp Hiss (2.8–12 kHz)"
                        bandDb: controller ? controller.bandHighHissStr : "-59.0"
                    }
                }

                KeyValueRow {
                    app: tab.app
                    keyText: "Detected Room Signature:"
                    valText: controller ? controller.noiseSignature : "—"
                }
                KeyValueRow {
                    app: tab.app
                    keyText: "Ambient vs RNNoise Floor:"
                    valText: controller ? controller.noiseFloorSummary : "—"
                    monoValue: true
                    valColor: tab.app.okCol
                }
                KeyValueRow {
                    app: tab.app
                    keyText: "Learned Filter & HW Rec:"
                    valText: controller ? controller.learnedFilterSummary : "—"
                    monoValue: true
                }
                KeyValueRow {
                    app: tab.app
                    keyText: "Last Learned / Pass Count:"
                    valText: controller
                             ? (controller.noiseProfileLearnedAt + " • Adaptive Passes: " + controller.adaptivePasses)
                             : "—"
                }
            }
        }
    }
}
