#pragma once

#include <QObject>
#include <QString>
#include <QTimer>
#include <QMenu>
#include <QIcon>
#include <QPointer>
#include <QQuickItem>
#include <QPointF>
#include <QEvent>

class KStatusNotifierItem;

struct TxSlotCache {
    bool connected = false;
    int batteryPct = 0;
    int batteryCode = 0;
    bool charging = false;
    int vuLevel = 0;
    QString serial = QStringLiteral("—");
    QString firmware = QStringLiteral("—");
    QString product;
    QString tone = QStringLiteral("standard");
    QString ncSummary = QStringLiteral("—");
    QString rawFlags = QStringLiteral("—");
};

class DjiMicController : public QObject {
    Q_OBJECT

    Q_PROPERTY(QString appVersion READ appVersion CONSTANT)
    Q_PROPERTY(QString appIconUrl READ appIconUrl CONSTANT)

    Q_PROPERTY(bool windowVisible READ windowVisible NOTIFY windowChanged)
    Q_PROPERTY(bool quitting READ quitting NOTIFY windowChanged)
    Q_PROPERTY(int activeTabIndex READ activeTabIndex NOTIFY windowChanged)

    Q_PROPERTY(int uiZoomPercent READ uiZoomPercent NOTIFY configChanged)
    Q_PROPERTY(QString colorScheme READ colorScheme NOTIFY configChanged)
    Q_PROPERTY(bool startOnLogin READ startOnLogin NOTIFY configChanged)
    Q_PROPERTY(bool startMinimized READ startMinimized NOTIFY configChanged)
    Q_PROPERTY(bool alertOnRxDisconnect READ alertOnRxDisconnect NOTIFY configChanged)
    Q_PROPERTY(bool alertOnTxDisconnect READ alertOnTxDisconnect NOTIFY configChanged)
    Q_PROPERTY(bool alertOnLowBattery READ alertOnLowBattery NOTIFY configChanged)
    Q_PROPERTY(int lowBatteryThresholdPct READ lowBatteryThresholdPct NOTIFY configChanged)
    Q_PROPERTY(QString aiNcMode READ aiNcMode NOTIFY configChanged)
    Q_PROPERTY(int aiNcStrengthPct READ aiNcStrengthPct NOTIFY configChanged)
    Q_PROPERTY(bool aiNcVirtualMic READ aiNcVirtualMic NOTIFY configChanged)

    Q_PROPERTY(bool udevReady READ udevReady NOTIFY statusChanged)
    Q_PROPERTY(QString udevSummary READ udevSummary NOTIFY statusChanged)
    Q_PROPERTY(QString udevRuleFileStatus READ udevRuleFileStatus NOTIFY statusChanged)
    Q_PROPERTY(QString udevDeviceAccessStatus READ udevDeviceAccessStatus NOTIFY statusChanged)

    Q_PROPERTY(bool deviceInUse READ deviceInUse NOTIFY telemetryChanged)
    Q_PROPERTY(bool usbPresent READ usbPresent NOTIFY telemetryChanged)
    Q_PROPERTY(bool rxConnected READ rxConnected NOTIFY telemetryChanged)
    Q_PROPERTY(int txConnectedCount READ txConnectedCount NOTIFY telemetryChanged)
    Q_PROPERTY(QString overallStatusState READ overallStatusState NOTIFY telemetryChanged)
    Q_PROPERTY(QString overallStatusBadge READ overallStatusBadge NOTIFY telemetryChanged)
    Q_PROPERTY(QString headerSubtitle READ headerSubtitle NOTIFY telemetryChanged)
    Q_PROPERTY(QString protocolDialect READ protocolDialect NOTIFY telemetryChanged)
    Q_PROPERTY(int lastSequence READ lastSequence NOTIFY telemetryChanged)
    Q_PROPERTY(int heartbeatCount READ heartbeatCount NOTIFY telemetryChanged)
    Q_PROPERTY(QString rxHardwareModel READ rxHardwareModel NOTIFY telemetryChanged)
    Q_PROPERTY(QString rxSerial READ rxSerial NOTIFY telemetryChanged)
    Q_PROPERTY(QString rxFirmware READ rxFirmware NOTIFY telemetryChanged)
    Q_PROPERTY(QString rxFirmwareAndRev READ rxFirmwareAndRev NOTIFY telemetryChanged)
    Q_PROPERTY(QString rxGainSummary READ rxGainSummary NOTIFY telemetryChanged)
    Q_PROPERTY(QString rxSubtitle READ rxSubtitle NOTIFY telemetryChanged)
    Q_PROPERTY(QString statusBarSummary READ statusBarSummary NOTIFY telemetryChanged)

    Q_PROPERTY(bool tx1Connected READ tx1Connected NOTIFY telemetryChanged)
    Q_PROPERTY(int tx1BatteryPct READ tx1BatteryPct NOTIFY telemetryChanged)
    Q_PROPERTY(int tx1BatteryCode READ tx1BatteryCode NOTIFY telemetryChanged)
    Q_PROPERTY(bool tx1Charging READ tx1Charging NOTIFY telemetryChanged)
    Q_PROPERTY(int tx1VuLevel READ tx1VuLevel NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx1Serial READ tx1Serial NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx1Firmware READ tx1Firmware NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx1Product READ tx1Product NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx1Tone READ tx1Tone NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx1NcSummary READ tx1NcSummary NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx1RawFlags READ tx1RawFlags NOTIFY telemetryChanged)

    Q_PROPERTY(bool tx2Connected READ tx2Connected NOTIFY telemetryChanged)
    Q_PROPERTY(int tx2BatteryPct READ tx2BatteryPct NOTIFY telemetryChanged)
    Q_PROPERTY(int tx2BatteryCode READ tx2BatteryCode NOTIFY telemetryChanged)
    Q_PROPERTY(bool tx2Charging READ tx2Charging NOTIFY telemetryChanged)
    Q_PROPERTY(int tx2VuLevel READ tx2VuLevel NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx2Serial READ tx2Serial NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx2Firmware READ tx2Firmware NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx2Product READ tx2Product NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx2Tone READ tx2Tone NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx2NcSummary READ tx2NcSummary NOTIFY telemetryChanged)
    Q_PROPERTY(QString tx2RawFlags READ tx2RawFlags NOTIFY telemetryChanged)

    Q_PROPERTY(QString settingNcMode READ settingNcMode NOTIFY controlsChanged)
    Q_PROPERTY(QString settingNcPower READ settingNcPower NOTIFY controlsChanged)
    Q_PROPERTY(QString ncOverallSummary READ ncOverallSummary NOTIFY controlsChanged)
    Q_PROPERTY(QString settingStereo READ settingStereo NOTIFY controlsChanged)
    Q_PROPERTY(QString settingLowCut READ settingLowCut NOTIFY controlsChanged)
    Q_PROPERTY(QString settingSafetyTrack READ settingSafetyTrack NOTIFY controlsChanged)
    Q_PROPERTY(QString settingClipLimiter READ settingClipLimiter NOTIFY controlsChanged)
    Q_PROPERTY(QString settingLeds READ settingLeds NOTIFY controlsChanged)
    Q_PROPERTY(QString settingRxAutoOff READ settingRxAutoOff NOTIFY controlsChanged)
    Q_PROPERTY(QString settingTxAutoOff READ settingTxAutoOff NOTIFY controlsChanged)
    Q_PROPERTY(QString settingCameraPower READ settingCameraPower NOTIFY controlsChanged)
    Q_PROPERTY(QString settingPlugFree READ settingPlugFree NOTIFY controlsChanged)
    Q_PROPERTY(int alsaVolumePct READ alsaVolumePct NOTIFY controlsChanged)
    Q_PROPERTY(bool alsaMuted READ alsaMuted NOTIFY controlsChanged)
    Q_PROPERTY(QString linuxAudioSummary READ linuxAudioSummary NOTIFY controlsChanged)

    Q_PROPERTY(bool aiNcVirtualMicActive READ aiNcVirtualMicActive NOTIFY noiseChanged)
    Q_PROPERTY(bool noiseLearnActive READ noiseLearnActive NOTIFY noiseChanged)
    Q_PROPERTY(int noiseLearnProgressPct READ noiseLearnProgressPct NOTIFY noiseChanged)
    Q_PROPERTY(QString noiseLearnStatus READ noiseLearnStatus NOTIFY noiseChanged)
    Q_PROPERTY(bool noiseProfileLearned READ noiseProfileLearned NOTIFY noiseChanged)
    Q_PROPERTY(QString noiseProfileLearnedAt READ noiseProfileLearnedAt NOTIFY noiseChanged)
    Q_PROPERTY(QString noiseProfileSource READ noiseProfileSource NOTIFY noiseChanged)
    Q_PROPERTY(QString noiseSignature READ noiseSignature NOTIFY noiseChanged)
    Q_PROPERTY(QString noiseFloorSummary READ noiseFloorSummary NOTIFY noiseChanged)
    Q_PROPERTY(QString learnedFilterSummary READ learnedFilterSummary NOTIFY noiseChanged)
    Q_PROPERTY(QString bandLowRumbleStr READ bandLowRumbleStr NOTIFY noiseChanged)
    Q_PROPERTY(QString bandFanHumStr READ bandFanHumStr NOTIFY noiseChanged)
    Q_PROPERTY(QString bandMidRoomStr READ bandMidRoomStr NOTIFY noiseChanged)
    Q_PROPERTY(QString bandHighHissStr READ bandHighHissStr NOTIFY noiseChanged)
    Q_PROPERTY(int adaptivePasses READ adaptivePasses NOTIFY noiseChanged)

    Q_PROPERTY(QString usbVidPid READ usbVidPid NOTIFY statusChanged)
    Q_PROPERTY(QString usbManufacturerProduct READ usbManufacturerProduct NOTIFY statusChanged)
    Q_PROPERTY(QString usbDescriptorSerial READ usbDescriptorSerial NOTIFY statusChanged)
    Q_PROPERTY(QString usbTopology READ usbTopology NOTIFY statusChanged)
    Q_PROPERTY(QString usbSpeedAndPower READ usbSpeedAndPower NOTIFY statusChanged)
    Q_PROPERTY(QString usbFsNode READ usbFsNode NOTIFY statusChanged)
    Q_PROPERTY(QString usbSysfsPath READ usbSysfsPath NOTIFY statusChanged)
    Q_PROPERTY(QString hidNodeAndPhys READ hidNodeAndPhys NOTIFY statusChanged)
    Q_PROPERTY(QString hidServiceSummary READ hidServiceSummary NOTIFY statusChanged)
    Q_PROPERTY(QString alsaCardDetails READ alsaCardDetails NOTIFY statusChanged)
    Q_PROPERTY(QString pipewireNodeDetails READ pipewireNodeDetails NOTIFY statusChanged)
    Q_PROPERTY(QString pipewireFormatState READ pipewireFormatState NOTIFY statusChanged)
    Q_PROPERTY(QString lastHeartbeatHex READ lastHeartbeatHex NOTIFY statusChanged)
    Q_PROPERTY(QString aux5b05Hex READ aux5b05Hex NOTIFY statusChanged)
    Q_PROPERTY(QString aux0474Hex READ aux0474Hex NOTIFY statusChanged)
    Q_PROPERTY(QString lastActionMessage READ lastActionMessage NOTIFY statusChanged)
    Q_PROPERTY(QString eventLogText READ eventLogText NOTIFY statusChanged)

public:
    explicit DjiMicController(bool startMinimized, bool uiTestMode, const QString& appDir, QObject* parent = nullptr);
    ~DjiMicController() override;

    QString appVersion() const {
#ifdef APP_VERSION_STR
        return QStringLiteral(APP_VERSION_STR);
#else
        return QStringLiteral("0.2.5");
#endif
    }
    QString appIconUrl() const;
    bool deviceInUse() const { return m_deviceInUse; }
    bool windowVisible() const { return m_windowVisible; }
    bool quitting() const { return m_quitting; }
    int activeTabIndex() const { return m_activeTabIndex; }
    int uiZoomPercent() const { return m_uiZoomPercent; }
    QString colorScheme() const { return m_colorScheme; }
    bool startOnLogin() const { return m_startOnLogin; }
    bool startMinimized() const { return m_startMinimized; }
    bool alertOnRxDisconnect() const { return m_alertOnRxDisconnect; }
    bool alertOnTxDisconnect() const { return m_alertOnTxDisconnect; }
    bool alertOnLowBattery() const { return m_alertOnLowBattery; }
    int lowBatteryThresholdPct() const { return m_lowBatteryThresholdPct; }

    bool udevReady() const { return m_udevReady; }
    QString udevSummary() const { return m_udevSummary; }
    QString udevRuleFileStatus() const { return m_udevRuleFileStatus; }
    QString udevDeviceAccessStatus() const { return m_udevDeviceAccessStatus; }

    bool usbPresent() const { return m_usbPresent; }
    bool rxConnected() const { return m_rxConnected; }
    int txConnectedCount() const { return m_txConnectedCount; }
    QString overallStatusState() const { return m_overallStatusState; }
    QString overallStatusBadge() const { return m_overallStatusBadge; }
    QString headerSubtitle() const { return m_headerSubtitle; }
    QString protocolDialect() const { return m_protocolDialect; }
    int lastSequence() const { return m_lastSequence; }
    int heartbeatCount() const { return m_heartbeatCount; }
    QString rxHardwareModel() const { return m_rxHardwareModel; }
    QString rxSerial() const { return m_rxSerial; }
    QString rxFirmware() const { return m_rxFirmware; }
    QString rxFirmwareAndRev() const { return m_rxFirmwareAndRev; }
    QString rxGainSummary() const { return m_rxGainSummary; }
    QString rxSubtitle() const { return m_rxSubtitle; }

    bool tx1Connected() const { return m_tx1.connected; }
    int tx1BatteryPct() const { return m_tx1.batteryPct; }
    int tx1BatteryCode() const { return m_tx1.batteryCode; }
    bool tx1Charging() const { return m_tx1.charging; }
    int tx1VuLevel() const { return m_tx1.vuLevel; }
    QString tx1Serial() const { return m_tx1.serial; }
    QString tx1Firmware() const { return m_tx1.firmware; }
    QString tx1Product() const { return m_tx1.product; }
    QString tx1Tone() const { return m_tx1.tone; }
    QString tx1NcSummary() const { return m_tx1.ncSummary; }
    QString tx1RawFlags() const { return m_tx1.rawFlags; }

    bool tx2Connected() const { return m_tx2.connected; }
    int tx2BatteryPct() const { return m_tx2.batteryPct; }
    int tx2BatteryCode() const { return m_tx2.batteryCode; }
    bool tx2Charging() const { return m_tx2.charging; }
    int tx2VuLevel() const { return m_tx2.vuLevel; }
    QString tx2Serial() const { return m_tx2.serial; }
    QString tx2Firmware() const { return m_tx2.firmware; }
    QString tx2Product() const { return m_tx2.product; }
    QString tx2Tone() const { return m_tx2.tone; }
    QString tx2NcSummary() const { return m_tx2.ncSummary; }
    QString tx2RawFlags() const { return m_tx2.rawFlags; }

    QString settingNcMode() const { return m_settingNcMode; }
    QString settingNcPower() const { return m_settingNcPower; }
    QString ncOverallSummary() const { return m_ncOverallSummary; }
    QString settingStereo() const { return m_settingStereo; }
    QString settingLowCut() const { return m_settingLowCut; }
    QString settingSafetyTrack() const { return m_settingSafetyTrack; }
    QString settingClipLimiter() const { return m_settingClipLimiter; }
    QString settingLeds() const { return m_settingLeds; }
    QString settingRxAutoOff() const { return m_settingRxAutoOff; }
    QString settingTxAutoOff() const { return m_settingTxAutoOff; }
    QString settingCameraPower() const { return m_settingCameraPower; }
    QString settingPlugFree() const { return m_settingPlugFree; }

    int alsaVolumePct() const { return m_alsaVolumePct; }
    bool alsaMuted() const { return m_alsaMuted; }
    QString aiNcMode() const { return m_aiNcMode; }
    int aiNcStrengthPct() const { return m_aiNcStrengthPct; }
    bool aiNcVirtualMic() const { return m_aiNcVirtualMic; }
    bool aiNcVirtualMicActive() const { return m_aiNcVirtualMicActive; }
    bool noiseLearnActive() const { return m_noiseLearnActive; }
    int noiseLearnProgressPct() const { return m_noiseLearnProgressPct; }
    QString noiseLearnStatus() const { return m_noiseLearnStatus; }
    bool noiseProfileLearned() const { return m_noiseProfileLearned; }
    QString noiseProfileLearnedAt() const { return m_noiseProfileLearnedAt; }
    QString noiseProfileSource() const { return m_noiseProfileSource; }
    QString noiseSignature() const { return m_noiseSignature; }
    QString noiseFloorSummary() const { return m_noiseFloorSummary; }
    QString learnedFilterSummary() const { return m_learnedFilterSummary; }
    QString bandLowRumbleStr() const { return m_bandLowRumbleStr; }
    QString bandFanHumStr() const { return m_bandFanHumStr; }
    QString bandMidRoomStr() const { return m_bandMidRoomStr; }
    QString bandHighHissStr() const { return m_bandHighHissStr; }
    int adaptivePasses() const { return m_adaptivePasses; }
    QString linuxAudioSummary() const { return m_linuxAudioSummary; }
    QString usbVidPid() const { return m_usbVidPid; }
    QString usbManufacturerProduct() const { return m_usbManufacturerProduct; }
    QString usbDescriptorSerial() const { return m_usbDescriptorSerial; }
    QString usbTopology() const { return m_usbTopology; }
    QString usbSpeedAndPower() const { return m_usbSpeedAndPower; }
    QString usbFsNode() const { return m_usbFsNode; }
    QString usbSysfsPath() const { return m_usbSysfsPath; }
    QString hidNodeAndPhys() const { return m_hidNodeAndPhys; }
    QString hidServiceSummary() const { return m_hidServiceSummary; }
    QString alsaCardDetails() const { return m_alsaCardDetails; }
    QString pipewireNodeDetails() const { return m_pipewireNodeDetails; }
    QString pipewireFormatState() const { return m_pipewireFormatState; }
    QString lastHeartbeatHex() const { return m_lastHeartbeatHex; }
    QString aux5b05Hex() const { return m_aux5b05Hex; }
    QString aux0474Hex() const { return m_aux0474Hex; }
    QString lastActionMessage() const { return m_lastActionMessage; }
    QString eventLogText() const { return m_eventLogText; }
    QString statusBarSummary() const { return m_statusBarSummary; }

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
    Q_INVOKABLE void setStartOnLogin(bool enabled);
    Q_INVOKABLE void setStartMinimized(bool enabled);
    Q_INVOKABLE void setAlertOnRxDisconnect(bool enabled);
    Q_INVOKABLE void setAlertOnTxDisconnect(bool enabled);
    Q_INVOKABLE void setAlertOnLowBattery(bool enabled);
    Q_INVOKABLE void setLowBatteryThresholdPct(int val);
    Q_INVOKABLE void sendTestAlert();
    Q_INVOKABLE void openKdeShortcutsSettings();
    Q_INVOKABLE void openKdeNotificationsSettings();
    Q_INVOKABLE void setColorScheme(const QString& scheme);
    Q_INVOKABLE void setUiZoomPercent(int pct);
    Q_INVOKABLE void zoomIn();
    Q_INVOKABLE void zoomOut();
    Q_INVOKABLE void zoomReset();
    Q_INVOKABLE void quitApp();

    void setupTray();
    void updateTrayUi(bool force = false);
    void syncSnapshot(bool forceAll = false);
    void registerKdeGlobalShortcuts();
    void applyNativeColorScheme(const QString& scheme);
    void sendDesktopNotificationDBus(
        const QString& eventId,
        const QString& title,
        const QString& body,
        const QString& icon,
        int urgency
    );

signals:
    void windowChanged();
    void configChanged();
    void telemetryChanged();
    void controlsChanged();
    void noiseChanged();
    void statusChanged();

private slots:
    void onGlobalShortcutPressed(const QString& componentUnique, const QString& shortcutUnique, qlonglong timestamp);

private:
    bool m_uiTestMode = false;
    QString m_appDir;
    QPointer<KStatusNotifierItem> m_tray;
    QPointer<QMenu> m_trayMenu;
    QIcon m_cachedTrayIcon;
    QString m_lastTrayIconSig;
    QString m_lastTrayTip;
    QString m_lastTrayMenuSig;

    // Cached snapshot state
    bool m_windowVisible = true;
    bool m_quitting = false;
    int m_activeTabIndex = 0;

    int m_uiZoomPercent = 100;
    QString m_colorScheme = QStringLiteral("system");
    bool m_startOnLogin = true;
    bool m_startMinimized = true;
    bool m_alertOnRxDisconnect = true;
    bool m_alertOnTxDisconnect = true;
    bool m_alertOnLowBattery = true;
    int m_lowBatteryThresholdPct = 25;
    QString m_aiNcMode = QStringLiteral("learned_ai");
    int m_aiNcStrengthPct = 80;
    bool m_aiNcVirtualMic = false;

    bool m_deviceInUse = false;
    int m_minBatteryPct = -1;
    int m_maxVuLevel = 0;

    bool m_udevReady = false;
    QString m_udevSummary;
    QString m_udevRuleFileStatus;
    QString m_udevDeviceAccessStatus;

    bool m_usbPresent = false;
    bool m_rxConnected = false;
    int m_txConnectedCount = 0;
    QString m_overallStatusState = QStringLiteral("error");
    QString m_overallStatusBadge = QStringLiteral("CHECKING");
    QString m_headerSubtitle;
    QString m_protocolDialect = QStringLiteral("v1");
    int m_lastSequence = 0;
    int m_heartbeatCount = 0;
    QString m_rxHardwareModel = QStringLiteral("DJI MIC MINI RX");
    QString m_rxSerial = QStringLiteral("—");
    QString m_rxFirmware = QStringLiteral("—");
    QString m_rxFirmwareAndRev = QStringLiteral("—");
    QString m_rxGainSummary = QStringLiteral("0 dB");
    QString m_rxSubtitle;
    QString m_statusBarSummary;

    TxSlotCache m_tx1;
    TxSlotCache m_tx2;

    QString m_settingNcMode = QStringLiteral("basic");
    QString m_settingNcPower = QStringLiteral("off");
    QString m_ncOverallSummary = QStringLiteral("—");
    QString m_settingStereo = QStringLiteral("mono");
    QString m_settingLowCut = QStringLiteral("off");
    QString m_settingSafetyTrack = QStringLiteral("off");
    QString m_settingClipLimiter = QStringLiteral("on");
    QString m_settingLeds = QStringLiteral("on");
    QString m_settingRxAutoOff = QStringLiteral("on");
    QString m_settingTxAutoOff = QStringLiteral("on");
    QString m_settingCameraPower = QStringLiteral("off");
    QString m_settingPlugFree = QStringLiteral("on");
    int m_alsaVolumePct = 100;
    bool m_alsaMuted = false;
    QString m_linuxAudioSummary;

    bool m_aiNcVirtualMicActive = false;
    bool m_noiseLearnActive = false;
    int m_noiseLearnProgressPct = 0;
    QString m_noiseLearnStatus;
    bool m_noiseProfileLearned = false;
    QString m_noiseProfileLearnedAt;
    QString m_noiseProfileSource;
    QString m_noiseSignature;
    QString m_noiseFloorSummary;
    QString m_learnedFilterSummary;
    QString m_bandLowRumbleStr = QStringLiteral("-55.0");
    QString m_bandFanHumStr = QStringLiteral("-51.0");
    QString m_bandMidRoomStr = QStringLiteral("-57.0");
    QString m_bandHighHissStr = QStringLiteral("-59.0");
    int m_adaptivePasses = 0;

    QString m_usbVidPid = QStringLiteral("2ca3:4011");
    QString m_usbManufacturerProduct;
    QString m_usbDescriptorSerial = QStringLiteral("—");
    QString m_usbTopology = QStringLiteral("—");
    QString m_usbSpeedAndPower = QStringLiteral("—");
    QString m_usbFsNode = QStringLiteral("—");
    QString m_usbSysfsPath = QStringLiteral("—");
    QString m_hidNodeAndPhys = QStringLiteral("—");
    QString m_hidServiceSummary;
    QString m_alsaCardDetails = QStringLiteral("—");
    QString m_pipewireNodeDetails = QStringLiteral("—");
    QString m_pipewireFormatState = QStringLiteral("—");
    QString m_lastHeartbeatHex = QStringLiteral("—");
    QString m_aux5b05Hex = QStringLiteral("—");
    QString m_aux0474Hex = QStringLiteral("—");
    QString m_lastActionMessage;
    QString m_eventLogText;
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
    QPointer<QQuickItem> m_lockedFlickable;
    qint64 m_lastWheelMs = 0;
    bool m_scrollingActive = false;
    QTimer* m_hoverRestoreTimer = nullptr;
};
