#pragma once

#include <QObject>
#include <QString>
#include <QTimer>
#include <QMenu>
#include <QQuickItem>
#include <QPointF>
#include <QEvent>

class KStatusNotifierItem;

class DjiMicController : public QObject {
    Q_OBJECT

    Q_PROPERTY(QString appVersion READ appVersion CONSTANT)
    Q_PROPERTY(QString appIconUrl READ appIconUrl CONSTANT)
    Q_PROPERTY(bool windowVisible READ windowVisible NOTIFY windowChanged)
    Q_PROPERTY(bool quitting READ quitting NOTIFY windowChanged)
    Q_PROPERTY(int activeTabIndex READ activeTabIndex NOTIFY windowChanged)
    Q_PROPERTY(int uiZoomPercent READ uiZoomPercent NOTIFY configChanged)
    Q_PROPERTY(QString colorScheme READ colorScheme NOTIFY configChanged)
    Q_PROPERTY(bool alertOnRxDisconnect READ alertOnRxDisconnect NOTIFY configChanged)
    Q_PROPERTY(bool alertOnTxDisconnect READ alertOnTxDisconnect NOTIFY configChanged)
    Q_PROPERTY(bool alertOnLowBattery READ alertOnLowBattery NOTIFY configChanged)
    Q_PROPERTY(int lowBatteryThresholdPct READ lowBatteryThresholdPct NOTIFY configChanged)

    Q_PROPERTY(bool udevReady READ udevReady NOTIFY stateChanged)
    Q_PROPERTY(QString udevSummary READ udevSummary NOTIFY stateChanged)
    Q_PROPERTY(QString udevRuleFileStatus READ udevRuleFileStatus NOTIFY stateChanged)
    Q_PROPERTY(QString udevDeviceAccessStatus READ udevDeviceAccessStatus NOTIFY stateChanged)

    Q_PROPERTY(bool usbPresent READ usbPresent NOTIFY stateChanged)
    Q_PROPERTY(bool rxConnected READ rxConnected NOTIFY stateChanged)
    Q_PROPERTY(int txConnectedCount READ txConnectedCount NOTIFY stateChanged)
    Q_PROPERTY(QString overallStatusState READ overallStatusState NOTIFY stateChanged)
    Q_PROPERTY(QString overallStatusBadge READ overallStatusBadge NOTIFY stateChanged)
    Q_PROPERTY(QString headerSubtitle READ headerSubtitle NOTIFY stateChanged)
    Q_PROPERTY(QString protocolDialect READ protocolDialect NOTIFY stateChanged)
    Q_PROPERTY(int lastSequence READ lastSequence NOTIFY stateChanged)
    Q_PROPERTY(int heartbeatCount READ heartbeatCount NOTIFY stateChanged)
    Q_PROPERTY(QString rxHardwareModel READ rxHardwareModel NOTIFY stateChanged)
    Q_PROPERTY(QString rxSerial READ rxSerial NOTIFY stateChanged)
    Q_PROPERTY(QString rxFirmware READ rxFirmware NOTIFY stateChanged)
    Q_PROPERTY(QString rxFirmwareAndRev READ rxFirmwareAndRev NOTIFY stateChanged)
    Q_PROPERTY(QString rxGainSummary READ rxGainSummary NOTIFY stateChanged)
    Q_PROPERTY(QString rxSubtitle READ rxSubtitle NOTIFY stateChanged)

    Q_PROPERTY(bool tx1Connected READ tx1Connected NOTIFY stateChanged)
    Q_PROPERTY(int tx1BatteryPct READ tx1BatteryPct NOTIFY stateChanged)
    Q_PROPERTY(int tx1BatteryCode READ tx1BatteryCode NOTIFY stateChanged)
    Q_PROPERTY(bool tx1Charging READ tx1Charging NOTIFY stateChanged)
    Q_PROPERTY(int tx1VuLevel READ tx1VuLevel NOTIFY stateChanged)
    Q_PROPERTY(QString tx1Serial READ tx1Serial NOTIFY stateChanged)
    Q_PROPERTY(QString tx1Firmware READ tx1Firmware NOTIFY stateChanged)
    Q_PROPERTY(QString tx1Product READ tx1Product NOTIFY stateChanged)
    Q_PROPERTY(QString tx1Tone READ tx1Tone NOTIFY stateChanged)
    Q_PROPERTY(QString tx1NcSummary READ tx1NcSummary NOTIFY stateChanged)
    Q_PROPERTY(QString tx1RawFlags READ tx1RawFlags NOTIFY stateChanged)

    Q_PROPERTY(bool tx2Connected READ tx2Connected NOTIFY stateChanged)
    Q_PROPERTY(int tx2BatteryPct READ tx2BatteryPct NOTIFY stateChanged)
    Q_PROPERTY(int tx2BatteryCode READ tx2BatteryCode NOTIFY stateChanged)
    Q_PROPERTY(bool tx2Charging READ tx2Charging NOTIFY stateChanged)
    Q_PROPERTY(int tx2VuLevel READ tx2VuLevel NOTIFY stateChanged)
    Q_PROPERTY(QString tx2Serial READ tx2Serial NOTIFY stateChanged)
    Q_PROPERTY(QString tx2Firmware READ tx2Firmware NOTIFY stateChanged)
    Q_PROPERTY(QString tx2Product READ tx2Product NOTIFY stateChanged)
    Q_PROPERTY(QString tx2Tone READ tx2Tone NOTIFY stateChanged)
    Q_PROPERTY(QString tx2NcSummary READ tx2NcSummary NOTIFY stateChanged)
    Q_PROPERTY(QString tx2RawFlags READ tx2RawFlags NOTIFY stateChanged)

    Q_PROPERTY(QString settingNcMode READ settingNcMode NOTIFY stateChanged)
    Q_PROPERTY(QString settingNcPower READ settingNcPower NOTIFY stateChanged)
    Q_PROPERTY(QString ncOverallSummary READ ncOverallSummary NOTIFY stateChanged)
    Q_PROPERTY(QString settingStereo READ settingStereo NOTIFY stateChanged)
    Q_PROPERTY(QString settingLowCut READ settingLowCut NOTIFY stateChanged)
    Q_PROPERTY(QString settingSafetyTrack READ settingSafetyTrack NOTIFY stateChanged)
    Q_PROPERTY(QString settingClipLimiter READ settingClipLimiter NOTIFY stateChanged)
    Q_PROPERTY(QString settingLeds READ settingLeds NOTIFY stateChanged)
    Q_PROPERTY(QString settingRxAutoOff READ settingRxAutoOff NOTIFY stateChanged)
    Q_PROPERTY(QString settingTxAutoOff READ settingTxAutoOff NOTIFY stateChanged)
    Q_PROPERTY(QString settingCameraPower READ settingCameraPower NOTIFY stateChanged)
    Q_PROPERTY(QString settingPlugFree READ settingPlugFree NOTIFY stateChanged)

    Q_PROPERTY(int alsaVolumePct READ alsaVolumePct NOTIFY stateChanged)
    Q_PROPERTY(bool alsaMuted READ alsaMuted NOTIFY stateChanged)
    Q_PROPERTY(QString aiNcMode READ aiNcMode NOTIFY stateChanged)
    Q_PROPERTY(int aiNcStrengthPct READ aiNcStrengthPct NOTIFY stateChanged)
    Q_PROPERTY(bool aiNcVirtualMic READ aiNcVirtualMic NOTIFY stateChanged)
    Q_PROPERTY(bool aiNcVirtualMicActive READ aiNcVirtualMicActive NOTIFY stateChanged)
    Q_PROPERTY(bool noiseLearnActive READ noiseLearnActive NOTIFY stateChanged)
    Q_PROPERTY(int noiseLearnProgressPct READ noiseLearnProgressPct NOTIFY stateChanged)
    Q_PROPERTY(QString noiseLearnStatus READ noiseLearnStatus NOTIFY stateChanged)
    Q_PROPERTY(bool noiseProfileLearned READ noiseProfileLearned NOTIFY stateChanged)
    Q_PROPERTY(QString noiseProfileLearnedAt READ noiseProfileLearnedAt NOTIFY stateChanged)
    Q_PROPERTY(QString noiseProfileSource READ noiseProfileSource NOTIFY stateChanged)
    Q_PROPERTY(QString noiseSignature READ noiseSignature NOTIFY stateChanged)
    Q_PROPERTY(QString noiseFloorSummary READ noiseFloorSummary NOTIFY stateChanged)
    Q_PROPERTY(QString learnedFilterSummary READ learnedFilterSummary NOTIFY stateChanged)
    Q_PROPERTY(QString bandLowRumbleStr READ bandLowRumbleStr NOTIFY stateChanged)
    Q_PROPERTY(QString bandFanHumStr READ bandFanHumStr NOTIFY stateChanged)
    Q_PROPERTY(QString bandMidRoomStr READ bandMidRoomStr NOTIFY stateChanged)
    Q_PROPERTY(QString bandHighHissStr READ bandHighHissStr NOTIFY stateChanged)
    Q_PROPERTY(int adaptivePasses READ adaptivePasses NOTIFY stateChanged)
    Q_PROPERTY(QString linuxAudioSummary READ linuxAudioSummary NOTIFY stateChanged)
    Q_PROPERTY(QString usbVidPid READ usbVidPid NOTIFY stateChanged)
    Q_PROPERTY(QString usbManufacturerProduct READ usbManufacturerProduct NOTIFY stateChanged)
    Q_PROPERTY(QString usbDescriptorSerial READ usbDescriptorSerial NOTIFY stateChanged)
    Q_PROPERTY(QString usbTopology READ usbTopology NOTIFY stateChanged)
    Q_PROPERTY(QString usbSpeedAndPower READ usbSpeedAndPower NOTIFY stateChanged)
    Q_PROPERTY(QString usbFsNode READ usbFsNode NOTIFY stateChanged)
    Q_PROPERTY(QString usbSysfsPath READ usbSysfsPath NOTIFY stateChanged)
    Q_PROPERTY(QString hidNodeAndPhys READ hidNodeAndPhys NOTIFY stateChanged)
    Q_PROPERTY(QString hidServiceSummary READ hidServiceSummary NOTIFY stateChanged)
    Q_PROPERTY(QString alsaCardDetails READ alsaCardDetails NOTIFY stateChanged)
    Q_PROPERTY(QString pipewireNodeDetails READ pipewireNodeDetails NOTIFY stateChanged)
    Q_PROPERTY(QString pipewireFormatState READ pipewireFormatState NOTIFY stateChanged)
    Q_PROPERTY(QString lastHeartbeatHex READ lastHeartbeatHex NOTIFY stateChanged)
    Q_PROPERTY(QString aux5b05Hex READ aux5b05Hex NOTIFY stateChanged)
    Q_PROPERTY(QString aux0474Hex READ aux0474Hex NOTIFY stateChanged)
    Q_PROPERTY(QString lastActionMessage READ lastActionMessage NOTIFY stateChanged)
    Q_PROPERTY(QString eventLogText READ eventLogText NOTIFY stateChanged)
    Q_PROPERTY(QString statusBarSummary READ statusBarSummary NOTIFY stateChanged)

public:
    explicit DjiMicController(bool startMinimized, bool uiTestMode, const QString& appDir, QObject* parent = nullptr);
    ~DjiMicController() override = default;

    QString appVersion() const {
#ifdef APP_VERSION_STR
        return QStringLiteral(APP_VERSION_STR);
#else
        return QStringLiteral("0.2.1");
#endif
    }
    QString appIconUrl() const;
    bool deviceInUse() const;
    bool windowVisible() const;
    bool quitting() const;
    int activeTabIndex() const;
    int uiZoomPercent() const;
    QString colorScheme() const;
    bool alertOnRxDisconnect() const;
    bool alertOnTxDisconnect() const;
    bool alertOnLowBattery() const;
    int lowBatteryThresholdPct() const;

    bool udevReady() const;
    QString udevSummary() const;
    QString udevRuleFileStatus() const;
    QString udevDeviceAccessStatus() const;

    bool usbPresent() const;
    bool rxConnected() const;
    int txConnectedCount() const;
    QString overallStatusState() const;
    QString overallStatusBadge() const;
    QString headerSubtitle() const;
    QString protocolDialect() const;
    int lastSequence() const;
    int heartbeatCount() const;
    QString rxHardwareModel() const;
    QString rxSerial() const;
    QString rxFirmware() const;
    QString rxFirmwareAndRev() const;
    QString rxGainSummary() const;
    QString rxSubtitle() const;

    bool tx1Connected() const;
    int tx1BatteryPct() const;
    int tx1BatteryCode() const;
    bool tx1Charging() const;
    int tx1VuLevel() const;
    QString tx1Serial() const;
    QString tx1Firmware() const;
    QString tx1Product() const;
    QString tx1Tone() const;
    QString tx1NcSummary() const;
    QString tx1RawFlags() const;

    bool tx2Connected() const;
    int tx2BatteryPct() const;
    int tx2BatteryCode() const;
    bool tx2Charging() const;
    int tx2VuLevel() const;
    QString tx2Serial() const;
    QString tx2Firmware() const;
    QString tx2Product() const;
    QString tx2Tone() const;
    QString tx2NcSummary() const;
    QString tx2RawFlags() const;

    QString settingNcMode() const;
    QString settingNcPower() const;
    QString ncOverallSummary() const;
    QString settingStereo() const;
    QString settingLowCut() const;
    QString settingSafetyTrack() const;
    QString settingClipLimiter() const;
    QString settingLeds() const;
    QString settingRxAutoOff() const;
    QString settingTxAutoOff() const;
    QString settingCameraPower() const;
    QString settingPlugFree() const;

    int alsaVolumePct() const;
    bool alsaMuted() const;
    QString aiNcMode() const;
    int aiNcStrengthPct() const;
    bool aiNcVirtualMic() const;
    bool aiNcVirtualMicActive() const;
    bool noiseLearnActive() const;
    int noiseLearnProgressPct() const;
    QString noiseLearnStatus() const;
    bool noiseProfileLearned() const;
    QString noiseProfileLearnedAt() const;
    QString noiseProfileSource() const;
    QString noiseSignature() const;
    QString noiseFloorSummary() const;
    QString learnedFilterSummary() const;
    QString bandLowRumbleStr() const;
    QString bandFanHumStr() const;
    QString bandMidRoomStr() const;
    QString bandHighHissStr() const;
    int adaptivePasses() const;
    QString linuxAudioSummary() const;
    QString usbVidPid() const;
    QString usbManufacturerProduct() const;
    QString usbDescriptorSerial() const;
    QString usbTopology() const;
    QString usbSpeedAndPower() const;
    QString usbFsNode() const;
    QString usbSysfsPath() const;
    QString hidNodeAndPhys() const;
    QString hidServiceSummary() const;
    QString alsaCardDetails() const;
    QString pipewireNodeDetails() const;
    QString pipewireFormatState() const;
    QString lastHeartbeatHex() const;
    QString aux5b05Hex() const;
    QString aux0474Hex() const;
    QString lastActionMessage() const;
    QString eventLogText() const;
    QString statusBarSummary() const;

    Q_INVOKABLE void setWindowVisible(bool vis);
    Q_INVOKABLE void toggleWindow();
    Q_INVOKABLE void openAboutTab();
    Q_INVOKABLE void setActiveTabIndex(int idx);
    Q_INVOKABLE void refreshNow();
    Q_INVOKABLE void recheckUdevStatus();
    Q_INVOKABLE void copyUdevCommandToClipboard();
    Q_INVOKABLE void installUdevRuleInteractive();
    Q_INVOKABLE void applyHardwareSetting(const QString& setting, const QString& value);
    Q_INVOKABLE void setSystemMicMute(bool muted);
    Q_INVOKABLE void setSystemMicVolume(int pct);
    Q_INVOKABLE void learnNoiseProfile();
    Q_INVOKABLE void resetNoiseProfile();
    Q_INVOKABLE void setAiNcMode(const QString& mode);
    Q_INVOKABLE void setAiNcStrength(int pct);
    Q_INVOKABLE void setAiNcVirtualMic(bool enabled);
    Q_INVOKABLE void copyJsonSnapshotToClipboard();
    Q_INVOKABLE void setAlertOnRxDisconnect(bool enabled);
    Q_INVOKABLE void setAlertOnTxDisconnect(bool enabled);
    Q_INVOKABLE void setAlertOnLowBattery(bool enabled);
    Q_INVOKABLE void setLowBatteryThresholdPct(int val);
    Q_INVOKABLE void sendTestAlert();
    Q_INVOKABLE void setColorScheme(const QString& scheme);
    Q_INVOKABLE void setUiZoomPercent(int pct);
    Q_INVOKABLE void zoomIn();
    Q_INVOKABLE void zoomOut();
    Q_INVOKABLE void zoomReset();
    Q_INVOKABLE void quitApp();

    void setupTray();
    void updateTrayUi(bool force = false);

signals:
    void stateChanged();
    void windowChanged();
    void configChanged();

private:
    bool m_uiTestMode = false;
    QString m_appDir;
    KStatusNotifierItem* m_tray = nullptr;
    QMenu* m_trayMenu = nullptr;
    QTimer* m_pollTimer = nullptr;
    QString m_lastTrayIconSig;
    QString m_lastTrayTip;
    QString m_lastTrayMenuSig;
};

class PloopyAndZoomEventFilter : public QObject {
    Q_OBJECT
public:
    explicit PloopyAndZoomEventFilter(DjiMicController* controller);
protected:
    bool eventFilter(QObject* obj, QEvent* event) override;
private:
    bool isScrollableFlickable(QQuickItem* item) const;
    QQuickItem* findDeepestFlickable(QQuickItem* rootItem, const QPointF& scenePos) const;
    DjiMicController* m_controller = nullptr;
    QQuickItem* m_lockedFlickable = nullptr;
    qint64 m_lastWheelMs = 0;
    bool m_scrollingActive = false;
    QTimer* m_hoverRestoreTimer = nullptr;
};
