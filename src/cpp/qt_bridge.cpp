#include "qt_bridge.h"

#include <QApplication>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
#include <QDesktopServices>
#include <QClipboard>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QIcon>
#include <QPainter>
#include <QPainterPath>
#include <QPen>
#include <QPixmap>
#include <QUrl>
#include <QKeyEvent>
#include <QWheelEvent>

#include <KStatusNotifierItem>

#include <cstdio>

extern "C" {
    void rust_free_str(char* ptr);
    void rust_register_callbacks(void (*on_state)(), void (*on_window)(), void (*on_config)());
    void rust_init_backend(bool start_minimized, bool ui_test_mode);
    void rust_poll_tick();
    bool rust_get_bool(const char* key);
    int rust_get_int(const char* key);
    char* rust_get_str(const char* key);

    void rust_set_window_visible(bool vis);
    void rust_toggle_window();
    void rust_set_active_tab_index(int idx);
    void rust_refresh_now();
    void rust_recheck_udev_status();
    void rust_notify_udev_copied();
    void rust_install_udev_rule_interactive();
    void rust_apply_hardware_setting(const char* setting, const char* value);
    void rust_set_system_mic_mute(bool muted);
    void rust_set_system_mic_volume(int pct);
    void rust_learn_noise_profile();
    void rust_reset_noise_profile();
    void rust_set_ai_nc_mode(const char* mode);
    void rust_set_ai_nc_strength(int pct);
    void rust_set_ai_nc_virtual_mic(bool enabled);
    void rust_notify_json_copied();
    void rust_set_alert_on_rx_disconnect(bool enabled);
    void rust_set_alert_on_tx_disconnect(bool enabled);
    void rust_set_alert_on_low_battery(bool enabled);
    void rust_set_low_battery_threshold_pct(int val);
    void rust_send_test_alert();
    void rust_set_color_scheme(const char* scheme);
    void rust_set_ui_zoom_percent(int pct);
    void rust_quit_app();
}

static DjiMicController* g_controller = nullptr;

static QString rStr(const char* key) {
    char* raw = rust_get_str(key);
    if (!raw) return QString();
    QString out = QString::fromUtf8(raw);
    rust_free_str(raw);
    return out;
}

static void onRustStateChanged() {
    if (!g_controller) return;
    QMetaObject::invokeMethod(g_controller, []() {
        if (!g_controller) return;
        g_controller->updateTrayUi(false);
        emit g_controller->stateChanged();
    }, Qt::QueuedConnection);
}

static void onRustWindowChanged() {
    if (!g_controller) return;
    QMetaObject::invokeMethod(g_controller, []() {
        if (!g_controller) return;
        emit g_controller->windowChanged();
        if (g_controller->windowVisible()) {
            for (QWindow* w : QGuiApplication::topLevelWindows()) {
                if (w->objectName() == QStringLiteral("mainWindow")) {
                    w->showNormal();
                    if (QGuiApplication::platformName() != QStringLiteral("offscreen")) {
                        w->raise();
                    }
                    w->requestActivate();
                }
            }
        }
    }, Qt::QueuedConnection);
}

static void onRustConfigChanged() {
    if (!g_controller) return;
    QMetaObject::invokeMethod(g_controller, []() {
        if (!g_controller) return;
        g_controller->updateTrayUi(true);
        emit g_controller->configChanged();
    }, Qt::QueuedConnection);
}

static QPixmap renderDjiMicPixmap(bool rxConnected, int txCount, int minBatteryPct, int maxVuLevel, bool muted, int size = 64) {
    const int sz = 64;
    QPixmap pix(sz, sz);
    pix.fill(Qt::transparent);

    QPainter p(&pix);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setRenderHint(QPainter::TextAntialiasing, true);

    QColor primaryCol;
    QString badgeText;
    if (!rxConnected) {
        primaryCol = QColor("#ef4444");
        badgeText = QStringLiteral("OFF");
    } else if (txCount == 0) {
        primaryCol = QColor("#f59e0b");
        badgeText = QStringLiteral("NO TX");
    } else if (muted) {
        primaryCol = QColor("#ef4444");
        badgeText = QStringLiteral("MUTE");
    } else {
        int pct = (minBatteryPct >= 0) ? minBatteryPct : 100;
        if (pct >= 40) {
            primaryCol = QColor("#22c55e");
        } else if (pct >= 20) {
            primaryCol = QColor("#f59e0b");
        } else {
            primaryCol = QColor("#ef4444");
        }
        badgeText = QStringLiteral("%1%").arg(pct);
    }

    if (rxConnected && txCount > 0 && !muted && maxVuLevel > 15) {
        int vuAlpha = qMin(180, int(40 + (maxVuLevel / 120.0) * 140));
        QColor glowCol = primaryCol;
        glowCol.setAlpha(vuAlpha);
        p.setPen(Qt::NoPen);
        p.setBrush(glowCol);
        p.drawEllipse(QRectF(12, 4, 32, 38));
    }

    p.setPen(QPen(QColor("#0f172a"), 2.4));
    p.setBrush(rxConnected ? QColor("#e2e8f0") : QColor("#64748b"));
    p.drawRoundedRect(QRectF(19, 7, 18, 28), 9, 9);

    p.setPen(QPen(QColor("#334155"), 1.6));
    for (int y : {14, 19, 24}) {
        p.drawLine(23, y, 33, y);
    }

    QPainterPath cradle;
    cradle.moveTo(14, 23);
    cradle.arcTo(QRectF(14, 11, 28, 28), 180, 180);
    p.setPen(QPen(primaryCol, 3.2, Qt::SolidLine, Qt::RoundCap));
    p.setBrush(Qt::NoBrush);
    p.drawPath(cradle);

    p.drawLine(28, 39, 28, 49);
    p.drawLine(19, 49, 37, 49);

    QColor rxLedCol = rxConnected ? QColor("#22c55e") : QColor("#ef4444");
    p.setPen(QPen(QColor("#0f172a"), 1.8));
    p.setBrush(rxLedCol);
    p.drawEllipse(QRectF(4, 5, 11, 11));

    QColor txLedCol = (rxConnected && txCount > 0) ? QColor("#22c55e") : (rxConnected ? QColor("#f59e0b") : QColor("#475569"));
    p.setBrush(txLedCol);
    p.drawEllipse(QRectF(47, 5, 12, 12));

    int bw = (badgeText.length() >= 4) ? 44 : 36;
    int bh = 20;
    int bx = sz - bw - 1;
    int by = sz - bh - 1;
    p.setPen(QPen(QColor("#0f172a"), 2.0));
    p.setBrush(primaryCol);
    p.drawRoundedRect(QRectF(bx, by, bw, bh), 6, 6);

    QFont font = p.font();
    font.setPixelSize(badgeText.length() <= 4 ? 11 : 10);
    font.setBold(true);
    p.setFont(font);
    p.setPen(QColor("#0f172a"));
    p.drawText(QRect(bx, by, bw, bh), Qt::AlignCenter, badgeText);

    p.end();
    if (size != sz) {
        return pix.scaled(size, size, Qt::KeepAspectRatio, Qt::SmoothTransformation);
    }
    return pix;
}

static QIcon renderDjiMicTrayIcon(bool rxConnected, int txCount, int minBatteryPct, int maxVuLevel, bool muted) {
    QIcon icon;
    icon.addPixmap(renderDjiMicPixmap(rxConnected, txCount, minBatteryPct, maxVuLevel, muted, 64));
    icon.addPixmap(renderDjiMicPixmap(rxConnected, txCount, minBatteryPct, maxVuLevel, muted, 256));
    return icon;
}

static void ensureTaskManagerIconFiles(const QString& appDir) {
    if (QFile::exists(QStringLiteral("/.flatpak-info"))) return;
    QPixmap pix64 = renderDjiMicPixmap(true, 1, 100, 45, false, 64);
    QPixmap pix256 = renderDjiMicPixmap(true, 1, 100, 45, false, 256);

    QString home = QDir::homePath();
    QString assetDir = appDir + QStringLiteral("/assets");
    QString hi256 = home + QStringLiteral("/.local/share/icons/hicolor/256x256/apps/dji-mic-tray.png");
    QString hi64 = home + QStringLiteral("/.local/share/icons/hicolor/64x64/apps/dji-mic-tray.png");

    QDir().mkpath(QFileInfo(hi256).absolutePath());
    QDir().mkpath(QFileInfo(hi64).absolutePath());

    if (QDir(assetDir).exists()) {
        pix256.save(assetDir + QStringLiteral("/dji-mic-tray.png"), "PNG");
    }
    pix256.save(hi256, "PNG");
    pix64.save(hi64, "PNG");
}

DjiMicController::DjiMicController(bool startMinimized, bool uiTestMode, const QString& appDir, QObject* parent)
    : QObject(parent), m_uiTestMode(uiTestMode), m_appDir(appDir) {
    g_controller = this;
    rust_register_callbacks(onRustStateChanged, onRustWindowChanged, onRustConfigChanged);
    rust_init_backend(startMinimized, uiTestMode);

    if (!m_uiTestMode) {
        ensureTaskManagerIconFiles(m_appDir);
        setupTray();
        m_pollTimer = new QTimer(this);
        m_pollTimer->setInterval(250);
        connect(m_pollTimer, &QTimer::timeout, this, []() {
            rust_poll_tick();
        });
        m_pollTimer->start();
    }
}

QString DjiMicController::appIconUrl() const {
    QString p = m_appDir + QStringLiteral("/assets/dji-mic-tray.png");
    if (QFile::exists(p)) {
        return QUrl::fromLocalFile(p).toString();
    }
    return QStringLiteral("audio-input-microphone");
}

bool DjiMicController::deviceInUse() const { return rust_get_bool("deviceInUse"); }
bool DjiMicController::windowVisible() const { return rust_get_bool("windowVisible"); }
bool DjiMicController::quitting() const { return rust_get_bool("quitting"); }
int DjiMicController::activeTabIndex() const { return rust_get_int("activeTabIndex"); }
int DjiMicController::uiZoomPercent() const { return rust_get_int("uiZoomPercent"); }
QString DjiMicController::colorScheme() const { return rStr("colorScheme"); }
bool DjiMicController::alertOnRxDisconnect() const { return rust_get_bool("alertOnRxDisconnect"); }
bool DjiMicController::alertOnTxDisconnect() const { return rust_get_bool("alertOnTxDisconnect"); }
bool DjiMicController::alertOnLowBattery() const { return rust_get_bool("alertOnLowBattery"); }
int DjiMicController::lowBatteryThresholdPct() const { return rust_get_int("lowBatteryThresholdPct"); }

bool DjiMicController::udevReady() const { return rust_get_bool("udevReady"); }
QString DjiMicController::udevSummary() const { return rStr("udevSummary"); }
QString DjiMicController::udevRuleFileStatus() const { return rStr("udevRuleFileStatus"); }
QString DjiMicController::udevDeviceAccessStatus() const { return rStr("udevDeviceAccessStatus"); }

bool DjiMicController::usbPresent() const { return rust_get_bool("usbPresent"); }
bool DjiMicController::rxConnected() const { return rust_get_bool("rxConnected"); }
int DjiMicController::txConnectedCount() const { return rust_get_int("txConnectedCount"); }
QString DjiMicController::overallStatusState() const { return rStr("overallStatusState"); }
QString DjiMicController::overallStatusBadge() const { return rStr("overallStatusBadge"); }
QString DjiMicController::headerSubtitle() const { return rStr("headerSubtitle"); }
QString DjiMicController::protocolDialect() const { return rStr("protocolDialect"); }
int DjiMicController::lastSequence() const { return rust_get_int("lastSequence"); }
int DjiMicController::heartbeatCount() const { return rust_get_int("heartbeatCount"); }
QString DjiMicController::rxHardwareModel() const { return rStr("rxHardwareModel"); }
QString DjiMicController::rxSerial() const { return rStr("rxSerial"); }
QString DjiMicController::rxFirmware() const { return rStr("rxFirmware"); }
QString DjiMicController::rxFirmwareAndRev() const { return rStr("rxFirmwareAndRev"); }
QString DjiMicController::rxGainSummary() const { return rStr("rxGainSummary"); }
QString DjiMicController::rxSubtitle() const { return rStr("rxSubtitle"); }

bool DjiMicController::tx1Connected() const { return rust_get_bool("tx1Connected"); }
int DjiMicController::tx1BatteryPct() const { return rust_get_int("tx1BatteryPct"); }
int DjiMicController::tx1BatteryCode() const { return rust_get_int("tx1BatteryCode"); }
bool DjiMicController::tx1Charging() const { return rust_get_bool("tx1Charging"); }
int DjiMicController::tx1VuLevel() const { return rust_get_int("tx1VuLevel"); }
QString DjiMicController::tx1Serial() const { return rStr("tx1Serial"); }
QString DjiMicController::tx1Firmware() const { return rStr("tx1Firmware"); }
QString DjiMicController::tx1Product() const { return rStr("tx1Product"); }
QString DjiMicController::tx1Tone() const { return rStr("tx1Tone"); }
QString DjiMicController::tx1NcSummary() const { return rStr("tx1NcSummary"); }
QString DjiMicController::tx1RawFlags() const { return rStr("tx1RawFlags"); }

bool DjiMicController::tx2Connected() const { return rust_get_bool("tx2Connected"); }
int DjiMicController::tx2BatteryPct() const { return rust_get_int("tx2BatteryPct"); }
int DjiMicController::tx2BatteryCode() const { return rust_get_int("tx2BatteryCode"); }
bool DjiMicController::tx2Charging() const { return rust_get_bool("tx2Charging"); }
int DjiMicController::tx2VuLevel() const { return rust_get_int("tx2VuLevel"); }
QString DjiMicController::tx2Serial() const { return rStr("tx2Serial"); }
QString DjiMicController::tx2Firmware() const { return rStr("tx2Firmware"); }
QString DjiMicController::tx2Product() const { return rStr("tx2Product"); }
QString DjiMicController::tx2Tone() const { return rStr("tx2Tone"); }
QString DjiMicController::tx2NcSummary() const { return rStr("tx2NcSummary"); }
QString DjiMicController::tx2RawFlags() const { return rStr("tx2RawFlags"); }

QString DjiMicController::settingNcMode() const { return rStr("settingNcMode"); }
QString DjiMicController::settingNcPower() const { return rStr("settingNcPower"); }
QString DjiMicController::ncOverallSummary() const { return rStr("ncOverallSummary"); }
QString DjiMicController::settingStereo() const { return rStr("settingStereo"); }
QString DjiMicController::settingLowCut() const { return rStr("settingLowCut"); }
QString DjiMicController::settingSafetyTrack() const { return rStr("settingSafetyTrack"); }
QString DjiMicController::settingClipLimiter() const { return rStr("settingClipLimiter"); }
QString DjiMicController::settingLeds() const { return rStr("settingLeds"); }
QString DjiMicController::settingRxAutoOff() const { return rStr("settingRxAutoOff"); }
QString DjiMicController::settingTxAutoOff() const { return rStr("settingTxAutoOff"); }
QString DjiMicController::settingCameraPower() const { return rStr("settingCameraPower"); }
QString DjiMicController::settingPlugFree() const { return rStr("settingPlugFree"); }

int DjiMicController::alsaVolumePct() const { return rust_get_int("alsaVolumePct"); }
bool DjiMicController::alsaMuted() const { return rust_get_bool("alsaMuted"); }
QString DjiMicController::aiNcMode() const { return rStr("aiNcMode"); }
int DjiMicController::aiNcStrengthPct() const { return rust_get_int("aiNcStrengthPct"); }
bool DjiMicController::aiNcVirtualMic() const { return rust_get_bool("aiNcVirtualMic"); }
bool DjiMicController::aiNcVirtualMicActive() const { return rust_get_bool("aiNcVirtualMicActive"); }
bool DjiMicController::noiseLearnActive() const { return rust_get_bool("noiseLearnActive"); }
int DjiMicController::noiseLearnProgressPct() const { return rust_get_int("noiseLearnProgressPct"); }
QString DjiMicController::noiseLearnStatus() const { return rStr("noiseLearnStatus"); }
bool DjiMicController::noiseProfileLearned() const { return rust_get_bool("noiseProfileLearned"); }
QString DjiMicController::noiseProfileLearnedAt() const { return rStr("noiseProfileLearnedAt"); }
QString DjiMicController::noiseProfileSource() const { return rStr("noiseProfileSource"); }
QString DjiMicController::noiseSignature() const { return rStr("noiseSignature"); }
QString DjiMicController::noiseFloorSummary() const { return rStr("noiseFloorSummary"); }
QString DjiMicController::learnedFilterSummary() const { return rStr("learnedFilterSummary"); }
QString DjiMicController::bandLowRumbleStr() const { return rStr("bandLowRumbleStr"); }
QString DjiMicController::bandFanHumStr() const { return rStr("bandFanHumStr"); }
QString DjiMicController::bandMidRoomStr() const { return rStr("bandMidRoomStr"); }
QString DjiMicController::bandHighHissStr() const { return rStr("bandHighHissStr"); }
int DjiMicController::adaptivePasses() const { return rust_get_int("adaptivePasses"); }
QString DjiMicController::linuxAudioSummary() const { return rStr("linuxAudioSummary"); }
QString DjiMicController::usbVidPid() const { return rStr("usbVidPid"); }
QString DjiMicController::usbManufacturerProduct() const { return rStr("usbManufacturerProduct"); }
QString DjiMicController::usbDescriptorSerial() const { return rStr("usbDescriptorSerial"); }
QString DjiMicController::usbTopology() const { return rStr("usbTopology"); }
QString DjiMicController::usbSpeedAndPower() const { return rStr("usbSpeedAndPower"); }
QString DjiMicController::usbFsNode() const { return rStr("usbFsNode"); }
QString DjiMicController::usbSysfsPath() const { return rStr("usbSysfsPath"); }
QString DjiMicController::hidNodeAndPhys() const { return rStr("hidNodeAndPhys"); }
QString DjiMicController::hidServiceSummary() const { return rStr("hidServiceSummary"); }
QString DjiMicController::alsaCardDetails() const { return rStr("alsaCardDetails"); }
QString DjiMicController::pipewireNodeDetails() const { return rStr("pipewireNodeDetails"); }
QString DjiMicController::pipewireFormatState() const { return rStr("pipewireFormatState"); }
QString DjiMicController::lastHeartbeatHex() const { return rStr("lastHeartbeatHex"); }
QString DjiMicController::aux5b05Hex() const { return rStr("aux5b05Hex"); }
QString DjiMicController::aux0474Hex() const { return rStr("aux0474Hex"); }
QString DjiMicController::lastActionMessage() const { return rStr("lastActionMessage"); }
QString DjiMicController::eventLogText() const { return rStr("eventLogText"); }
QString DjiMicController::statusBarSummary() const { return rStr("statusBarSummary"); }

void DjiMicController::setWindowVisible(bool vis) { rust_set_window_visible(vis); }
void DjiMicController::toggleWindow() { rust_toggle_window(); }
void DjiMicController::openAboutTab() {
    rust_set_active_tab_index(4);
    rust_set_window_visible(true);
}
void DjiMicController::setActiveTabIndex(int idx) { rust_set_active_tab_index(idx); }
void DjiMicController::refreshNow() { rust_refresh_now(); }
void DjiMicController::recheckUdevStatus() { rust_recheck_udev_status(); }
void DjiMicController::copyUdevCommandToClipboard() {
    if (QClipboard* cb = QGuiApplication::clipboard()) {
        cb->setText(rStr("udevInstallOneliner"));
        rust_notify_udev_copied();
    }
}
void DjiMicController::installUdevRuleInteractive() {
    copyUdevCommandToClipboard();
    rust_install_udev_rule_interactive();
}
void DjiMicController::applyHardwareSetting(const QString& setting, const QString& value) {
    rust_apply_hardware_setting(setting.toUtf8().constData(), value.toUtf8().constData());
}
void DjiMicController::setSystemMicMute(bool muted) { rust_set_system_mic_mute(muted); }
void DjiMicController::setSystemMicVolume(int pct) { rust_set_system_mic_volume(pct); }
void DjiMicController::learnNoiseProfile() { rust_learn_noise_profile(); }
void DjiMicController::resetNoiseProfile() { rust_reset_noise_profile(); }
void DjiMicController::setAiNcMode(const QString& mode) { rust_set_ai_nc_mode(mode.toUtf8().constData()); }
void DjiMicController::setAiNcStrength(int pct) { rust_set_ai_nc_strength(pct); }
void DjiMicController::setAiNcVirtualMic(bool enabled) { rust_set_ai_nc_virtual_mic(enabled); }
void DjiMicController::copyJsonSnapshotToClipboard() {
    if (QClipboard* cb = QGuiApplication::clipboard()) {
        cb->setText(rStr("fullSnapshotJson"));
        rust_notify_json_copied();
    }
}
void DjiMicController::setAlertOnRxDisconnect(bool enabled) { rust_set_alert_on_rx_disconnect(enabled); }
void DjiMicController::setAlertOnTxDisconnect(bool enabled) { rust_set_alert_on_tx_disconnect(enabled); }
void DjiMicController::setAlertOnLowBattery(bool enabled) { rust_set_alert_on_low_battery(enabled); }
void DjiMicController::setLowBatteryThresholdPct(int val) { rust_set_low_battery_threshold_pct(val); }
void DjiMicController::sendTestAlert() { rust_send_test_alert(); }
void DjiMicController::setColorScheme(const QString& scheme) { rust_set_color_scheme(scheme.toUtf8().constData()); }
void DjiMicController::setUiZoomPercent(int pct) { rust_set_ui_zoom_percent(pct); }
void DjiMicController::zoomIn() { setUiZoomPercent(uiZoomPercent() + 10); }
void DjiMicController::zoomOut() { setUiZoomPercent(uiZoomPercent() - 10); }
void DjiMicController::zoomReset() { setUiZoomPercent(100); }
void DjiMicController::quitApp() {
    if (m_pollTimer) m_pollTimer->stop();
    rust_quit_app();
    QApplication::quit();
}

void DjiMicController::setupTray() {
    m_tray = new KStatusNotifierItem(QStringLiteral("dji-mic-tray"), this);
    m_tray->setCategory(KStatusNotifierItem::Hardware);
    m_tray->setTitle(QStringLiteral("KDE DJI Mic Mini Control"));
    m_tray->setStandardActionsEnabled(false);
    m_trayMenu = new QMenu();
    m_tray->setContextMenu(m_trayMenu);
    connect(m_tray, &KStatusNotifierItem::activateRequested, this, [this](bool, const QPoint&) {
        toggleWindow();
    });
    connect(m_tray, &KStatusNotifierItem::secondaryActivateRequested, this, [this](const QPoint&) {
        setSystemMicMute(!alsaMuted());
    });
    updateTrayUi(true);
}

void DjiMicController::updateTrayUi(bool force) {
    if (!m_tray || !m_trayMenu) return;

    KStatusNotifierItem::ItemStatus targetStatus =
        deviceInUse() ? KStatusNotifierItem::Active : KStatusNotifierItem::Passive;
    if (m_tray->status() != targetStatus) {
        m_tray->setStatus(targetStatus);
    }

    bool rxOk = rxConnected();
    int txCnt = txConnectedCount();
    int minBat = rust_get_int("minBatteryPct");
    int maxVu = rust_get_int("maxVuLevel");
    bool vuActive = (rxOk && txCnt > 0 && !alsaMuted() && maxVu > 40);
    bool muted = alsaMuted();

    QString iconSig = QStringLiteral("%1|%2|%3|%4")
                          .arg(rxOk)
                          .arg(txCnt)
                          .arg(minBat)
                          .arg(muted);

    QIcon icon = renderDjiMicTrayIcon(rxOk, txCnt, minBat, vuActive ? 60 : 0, muted);
    if (force || iconSig != m_lastTrayIconSig) {
        m_lastTrayIconSig = iconSig;
        m_tray->setIconByPixmap(icon);
        if (auto* app = qobject_cast<QApplication*>(QApplication::instance())) {
            app->setWindowIcon(icon);
        }
        for (QWindow* w : QGuiApplication::topLevelWindows()) {
            w->setIcon(icon);
        }
    }

    QString tipTitle = QStringLiteral("KDE DJI Mic Mini Control — %1").arg(overallStatusBadge());
    QStringList tipLines;
    tipLines << QStringLiteral("Receiver RX: %1 (FW %2, S/N %3, Gain %4)")
                    .arg(rxHardwareModel(), rxFirmware(), rxSerial(), rxGainSummary());
    if (tx1Connected()) {
        tipLines << QStringLiteral("TX 1: Connected • Battery %1% (Code %2/7) • S/N %3")
                        .arg(tx1BatteryPct()).arg(tx1BatteryCode()).arg(tx1Serial());
    } else {
        tipLines << QStringLiteral("TX 1: Disconnected / Off");
    }
    if (tx2Connected()) {
        tipLines << QStringLiteral("TX 2: Connected • Battery %1% (Code %2/7) • S/N %3")
                        .arg(tx2BatteryPct()).arg(tx2BatteryCode()).arg(tx2Serial());
    } else {
        tipLines << QStringLiteral("TX 2: Disconnected / Off");
    }
    tipLines << QStringLiteral("Settings: %1 • NC %2 • Low-Cut %3 • LEDs %4")
                    .arg(settingStereo().toUpper(), ncOverallSummary(), settingLowCut().toUpper(), settingLeds().toUpper());
    tipLines << QStringLiteral("Linux Audio: %1").arg(linuxAudioSummary());
    QString tipSub = tipLines.join('\n');
    QString newTip = tipTitle + QStringLiteral("\n") + tipSub;
    if (force || newTip != m_lastTrayTip) {
        m_lastTrayTip = newTip;
        m_tray->setToolTip(icon, tipTitle, tipSub);
    }

    QString menuSig = QStringLiteral("%1|%2|%3|%4|%5|%6|%7|%8|%9|%10|%11|%12|%13|%14|%15|%16|%17|%18")
                          .arg(rxOk)
                          .arg(txCnt)
                          .arg(minBat)
                          .arg(muted)
                          .arg(udevReady())
                          .arg(settingNcMode(), settingStereo(), settingLowCut(), settingSafetyTrack())
                          .arg(settingClipLimiter(), settingLeds(), settingRxAutoOff(), settingCameraPower(), settingPlugFree())
                          .arg(alsaVolumePct())
                          .arg(alertOnRxDisconnect())
                          .arg(alertOnTxDisconnect())
                          .arg(alertOnLowBattery());
    if (!force && menuSig == m_lastTrayMenuSig) return;
    m_lastTrayMenuSig = menuSig;

    m_trayMenu->clear();
    QAction* hdr = m_trayMenu->addAction(QStringLiteral("🎙️ KDE DJI Mic Mini Control — %1").arg(overallStatusBadge()));
    hdr->setEnabled(false);

    QAction* openAct = m_trayMenu->addAction(QStringLiteral("Open KDE DJI Mic Mini Control..."));
    connect(openAct, &QAction::triggered, this, [this]() { setWindowVisible(true); });
    m_trayMenu->addSeparator();

    QAction* rxAct = m_trayMenu->addAction(
        QStringLiteral("RX: %1 | FW %2 | S/N %3 | Gain %4").arg(rxHardwareModel(), rxFirmware(), rxSerial(), rxGainSummary()));
    rxAct->setEnabled(false);

    QAction* tx1Act = m_trayMenu->addAction(
        tx1Connected()
            ? QStringLiteral("TX 1: Connected | Battery %1% (%2/7) | S/N %3 | FW %4")
                  .arg(tx1BatteryPct()).arg(tx1BatteryCode()).arg(tx1Serial(), tx1Firmware())
            : QStringLiteral("TX 1: Disconnected / Powered Off"));
    tx1Act->setEnabled(false);

    QAction* tx2Act = m_trayMenu->addAction(
        tx2Connected()
            ? QStringLiteral("TX 2: Connected | Battery %1% (%2/7) | S/N %3 | FW %4")
                  .arg(tx2BatteryPct()).arg(tx2BatteryCode()).arg(tx2Serial(), tx2Firmware())
            : QStringLiteral("TX 2: Disconnected / Powered Off"));
    tx2Act->setEnabled(false);

    QAction* usbAct = m_trayMenu->addAction(
        QStringLiteral("USB: %1 | %2 | udev: %3")
            .arg(usbTopology(), usbSpeedAndPower(), udevReady() ? QStringLiteral("OK") : QStringLiteral("Needs Setup")));
    usbAct->setEnabled(false);

    QAction* audAct = m_trayMenu->addAction(QStringLiteral("Audio: %1").arg(linuxAudioSummary()));
    audAct->setEnabled(false);

    m_trayMenu->addSeparator();

    QMenu* hwMenu = m_trayMenu->addMenu(QStringLiteral("Hardware Settings (Instant Apply)"));
    hwMenu->setEnabled(rxOk);

    QMenu* ncMenu = hwMenu->addMenu(QStringLiteral("Noise Cancellation Strength (%1)").arg(settingNcMode().toUpper()));
    for (const QString& mode : {QStringLiteral("basic"), QStringLiteral("strong")}) {
        QAction* act = ncMenu->addAction(mode == QStringLiteral("basic") ? QStringLiteral("Basic") : QStringLiteral("Strong"));
        act->setCheckable(true);
        act->setChecked(settingNcMode() == mode);
        connect(act, &QAction::triggered, this, [this, mode]() {
            applyHardwareSetting(QStringLiteral("noise-cancel"), mode);
        });
    }

    QMenu* chMenu = hwMenu->addMenu(QStringLiteral("Audio Channels (%1)").arg(settingStereo().toUpper()));
    for (const QString& mode : {QStringLiteral("mono"), QStringLiteral("stereo")}) {
        QAction* act = chMenu->addAction(mode == QStringLiteral("mono") ? QStringLiteral("Mono") : QStringLiteral("Stereo"));
        act->setCheckable(true);
        act->setChecked(settingStereo() == mode);
        connect(act, &QAction::triggered, this, [this, mode]() {
            applyHardwareSetting(QStringLiteral("stereo"), mode);
        });
    }

    struct ToggleItem {
        const char* key;
        const char* label;
        QString curVal;
    };
    const ToggleItem toggles[] = {
        {"low-cut", "Low-Cut High-Pass Filter", settingLowCut()},
        {"safety-track", "Safety Track (-6 dB)", settingSafetyTrack()},
        {"clip-limiter", "Clip Limiter", settingClipLimiter()},
        {"mic-leds", "Transmitter Status LEDs", settingLeds()},
        {"auto-off-15m", "RX Auto Power-Off (15 min)", settingRxAutoOff()},
        {"camera-power", "Power On/Off with Camera", settingCameraPower()},
        {"plug-free", "Plug-Free External Speaker", settingPlugFree()},
    };
    for (const auto& t : toggles) {
        QAction* act = hwMenu->addAction(QStringLiteral("%1 (%2)").arg(QString::fromUtf8(t.label), t.curVal.toUpper()));
        act->setCheckable(true);
        act->setChecked(t.curVal == QStringLiteral("on"));
        QString k = QString::fromUtf8(t.key);
        QString nextVal = (t.curVal == QStringLiteral("on")) ? QStringLiteral("off") : QStringLiteral("on");
        connect(act, &QAction::triggered, this, [this, k, nextVal]() {
            applyHardwareSetting(k, nextVal);
        });
    }

    QAction* muteAct = m_trayMenu->addAction(muted ? QStringLiteral("Unmute System Mic Capture")
                                                   : QStringLiteral("Mute System Mic Capture"));
    connect(muteAct, &QAction::triggered, this, [this, muted]() { setSystemMicMute(!muted); });

    QMenu* alertMenu = m_trayMenu->addMenu(QStringLiteral("Desktop Popup Alerts & udev"));
    QAction* aRx = alertMenu->addAction(QStringLiteral("Alert when USB-C Receiver Disconnects"));
    aRx->setCheckable(true);
    aRx->setChecked(alertOnRxDisconnect());
    connect(aRx, &QAction::triggered, this, &DjiMicController::setAlertOnRxDisconnect);

    QAction* aTx = alertMenu->addAction(QStringLiteral("Alert when Wireless TX Mic Disconnects"));
    aTx->setCheckable(true);
    aTx->setChecked(alertOnTxDisconnect());
    connect(aTx, &QAction::triggered, this, &DjiMicController::setAlertOnTxDisconnect);

    QAction* aBat = alertMenu->addAction(QStringLiteral("Alert on Low TX Battery (<= %1%)").arg(lowBatteryThresholdPct()));
    aBat->setCheckable(true);
    aBat->setChecked(alertOnLowBattery());
    connect(aBat, &QAction::triggered, this, &DjiMicController::setAlertOnLowBattery);

    alertMenu->addSeparator();
    QAction* udevAct = alertMenu->addAction(
        QStringLiteral("USB udev Rule: %1").arg(udevReady() ? QStringLiteral("Verified") : QStringLiteral("Install / Repair udev Rule...")));
    connect(udevAct, &QAction::triggered, this, &DjiMicController::installUdevRuleInteractive);

    QAction* testAct = alertMenu->addAction(QStringLiteral("Send Test Desktop Alert Popup Now"));
    connect(testAct, &QAction::triggered, this, &DjiMicController::sendTestAlert);

    m_trayMenu->addSeparator();
    QAction* aboutAct = m_trayMenu->addAction(QStringLiteral("About KDE DJI Mic Mini Control..."));
    connect(aboutAct, &QAction::triggered, this, &DjiMicController::openAboutTab);

    QAction* kofiAct = m_trayMenu->addAction(QStringLiteral("☕ Buy Me a Coffee (Ko-fi — Russell Milliner)"));
    connect(kofiAct, &QAction::triggered, this, []() {
        QDesktopServices::openUrl(QUrl(QStringLiteral("https://ko-fi.com/russellmilliner")));
    });

    m_trayMenu->addSeparator();
    QAction* quitAct = m_trayMenu->addAction(QStringLiteral("Quit KDE DJI Mic Mini Control"));
    connect(quitAct, &QAction::triggered, this, &DjiMicController::quitApp);
}

PloopyAndZoomEventFilter::PloopyAndZoomEventFilter(DjiMicController* controller)
    : QObject(controller), m_controller(controller) {
    m_hoverRestoreTimer = new QTimer(this);
    m_hoverRestoreTimer->setSingleShot(true);
    m_hoverRestoreTimer->setInterval(400);
    connect(m_hoverRestoreTimer, &QTimer::timeout, this, [this]() {
        m_scrollingActive = false;
        m_lockedFlickable = nullptr;
    });
}

bool PloopyAndZoomEventFilter::isScrollableFlickable(QQuickItem* item) const {
    if (!item || !item->metaObject()) return false;
    QString cls = QString::fromLatin1(item->metaObject()->className());
    if (!cls.contains("Flickable") && !cls.contains("ListView") && !cls.contains("GridView") && !cls.contains("ScrollView")) {
        return false;
    }
    double ch = item->property("contentHeight").toDouble();
    double h = item->height();
    return ch > h + 1.0;
}

QQuickItem* PloopyAndZoomEventFilter::findDeepestFlickable(QQuickItem* rootItem, const QPointF& scenePos) const {
    if (!rootItem || !rootItem->isVisible() || rootItem->opacity() <= 0.0) return nullptr;
    QQuickItem* best = nullptr;
    QPointF pt = rootItem->mapFromScene(scenePos);
    if (pt.x() >= 0.0 && pt.x() <= rootItem->width() && pt.y() >= 0.0 && pt.y() <= rootItem->height()) {
        if (isScrollableFlickable(rootItem)) {
            best = rootItem;
        }
        const auto children = rootItem->childItems();
        for (QQuickItem* child : children) {
            if (QQuickItem* sub = findDeepestFlickable(child, scenePos)) {
                best = sub;
            }
        }
    }
    return best;
}

bool PloopyAndZoomEventFilter::eventFilter(QObject* obj, QEvent* event) {
    QEvent::Type etype = event->type();
    if (m_scrollingActive && (etype == QEvent::HoverEnter || etype == QEvent::HoverMove || etype == QEvent::ToolTip)) {
        return true;
    }
    if (etype == QEvent::MouseMove && m_scrollingActive) {
        m_scrollingActive = false;
        m_lockedFlickable = nullptr;
    }
    auto* win = qobject_cast<QQuickWindow*>(obj);
    if (win && etype == QEvent::KeyPress) {
        auto* ke = static_cast<QKeyEvent*>(event);
        if (ke->modifiers() & Qt::ControlModifier) {
            int k = ke->key();
            QString txt = ke->text();
            if (k == Qt::Key_Plus || k == Qt::Key_Equal || txt == "+" || txt == "=") {
                m_controller->zoomIn();
                return true;
            }
            if (k == Qt::Key_Minus || k == Qt::Key_Underscore || txt == "-" || txt == "_") {
                m_controller->zoomOut();
                return true;
            }
            if (k == Qt::Key_0 || k == Qt::Key_ParenRight || txt == "0" || txt == ")") {
                m_controller->zoomReset();
                return true;
            }
        }
    }
    if (win && etype == QEvent::Wheel) {
        auto* we = static_cast<QWheelEvent*>(event);
        QPoint angle = we->angleDelta();
        QPoint pixel = we->pixelDelta();
        if (we->modifiers() & Qt::ControlModifier) {
            int dy = !angle.isNull() ? angle.y() : pixel.y();
            if (dy > 0) m_controller->zoomIn();
            else if (dy < 0) m_controller->zoomOut();
            return true;
        }
        double dyPx = 0.0;
        if (!angle.isNull() && angle.y() != 0) {
            dyPx = angle.y() * 0.50;
        } else if (!pixel.isNull() && pixel.y() != 0) {
            dyPx = double(pixel.y());
        }
        if (qAbs(dyPx) > 0.01) {
            qint64 nowMs = QDateTime::currentMSecsSinceEpoch();
            QPointF scenePos = we->position();
            QQuickItem* flick = nullptr;
            if (m_lockedFlickable && (nowMs - m_lastWheelMs) <= 350 && m_lockedFlickable->isVisible()) {
                flick = m_lockedFlickable;
            }
            if (!flick) {
                flick = findDeepestFlickable(win->contentItem(), scenePos);
                m_lockedFlickable = flick;
            }
            m_lastWheelMs = nowMs;
            if (flick) {
                if (!m_scrollingActive) {
                    m_scrollingActive = true;
                    QEvent leaveEv(QEvent::HoverLeave);
                    QCoreApplication::sendEvent(win, &leaveEv);
                }
                m_hoverRestoreTimer->start();
                double curY = flick->property("contentY").toDouble();
                double origY = flick->property("originY").toDouble();
                double topM = flick->property("topMargin").toDouble();
                double botM = flick->property("bottomMargin").toDouble();
                double ch = flick->property("contentHeight").toDouble();
                double h = flick->height();
                double minY = origY - topM;
                double maxY = qMax(minY, origY + ch + botM - h);
                double newY = qBound(minY, curY - dyPx, maxY);
                flick->setProperty("contentY", newY);
                we->accept();
                return true;
            }
        }
    }
    return QObject::eventFilter(obj, event);
}

static void checkTree(QQuickItem* item, const QString& ctx, bool checkTabBarFit, QStringList& violations) {
    if (!item || !item->isVisible() || item->opacity() <= 0.0 || item->width() <= 0.0 || item->height() <= 0.0) {
        return;
    }
    QString cls = item->metaObject() ? QString::fromLatin1(item->metaObject()->className()) : QString();
    if (checkTabBarFit && item->objectName() == QStringLiteral("mainTabBar")) {
        QObject* ci = item->property("contentItem").value<QObject*>();
        double cw = ci ? ci->property("contentWidth").toDouble() : item->implicitWidth();
        double tw = item->width();
        if (cw > tw + 2.0) {
            violations.append(QStringLiteral("%1: TabBar overflow — tabs require %2px but TabBar width is %3px")
                                  .arg(ctx).arg(cw, 0, 'f', 1).arg(tw, 0, 'f', 1));
        }
    }
    if (cls.contains("Text") || cls.contains("Label") || cls.contains("Heading")) {
        bool truncated = item->property("truncated").toBool();
        double cw = item->property("contentWidth").toDouble();
        double ch = item->property("contentHeight").toDouble();
        double w = item->width();
        double h = item->height();
        QString txt = item->property("text").toString().left(40);
        if (!truncated && cw > w + 4.0) {
            violations.append(QStringLiteral("%1: Horizontal text overflow (%2 > %3) on '%4'")
                                  .arg(ctx).arg(cw, 0, 'f', 1).arg(w, 0, 'f', 1).arg(txt));
        }
        if (!truncated && ch > h + 6.0) {
            violations.append(QStringLiteral("%1: Vertical text overflow (%2 > %3) on '%4'")
                                  .arg(ctx).arg(ch, 0, 'f', 1).arg(h, 0, 'f', 1).arg(txt));
        }
    }
    const auto children = item->childItems();
    for (QQuickItem* c : children) {
        checkTree(c, ctx, checkTabBarFit, violations);
    }
}

extern "C" int cpp_run_dji_mic_tray(int argc, char** argv, bool startMinimized, bool uiTestMode, const char* appDirCStr) {
    qputenv("QT_QUICK_CONTROLS_STYLE", "org.kde.desktop");
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("KDE DJI Mic Mini Control"));
    app.setApplicationDisplayName(QStringLiteral("KDE DJI Mic Mini Control"));
    app.setOrganizationName(QStringLiteral("millinr"));
    if (QFile::exists(QStringLiteral("/.flatpak-info"))) {
        QGuiApplication::setDesktopFileName(QStringLiteral("io.github.russellmilliner.DjiMicMiniControl"));
    } else {
        QGuiApplication::setDesktopFileName(QStringLiteral("dji-mic-tray"));
    }
    app.setQuitOnLastWindowClosed(false);

    QString appDir = QString::fromUtf8(appDirCStr);
    ensureTaskManagerIconFiles(appDir);
    QIcon initialIcon = renderDjiMicTrayIcon(true, 1, 100, 0, false);
    app.setWindowIcon(initialIcon);

    DjiMicController controller(startMinimized, uiTestMode, appDir);

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty(QStringLiteral("controller"), &controller);
    QString qmlPath = appDir + QStringLiteral("/qml/Main.qml");
    engine.load(QUrl::fromLocalFile(qmlPath));
    if (engine.rootObjects().isEmpty()) {
        fprintf(stderr, "Failed to load QML from %s\n", qmlPath.toUtf8().constData());
        return 1;
    }

    auto* win = qobject_cast<QQuickWindow*>(engine.rootObjects().first());
    if (!win) return 1;
    win->setIcon(initialIcon);

    PloopyAndZoomEventFilter evFilter(&controller);
    app.installEventFilter(&evFilter);

    if (uiTestMode) {
        QStringList violations;
        app.processEvents();
        if (!win->title().contains(controller.appVersion())) {
            violations.append(QStringLiteral("Window title ('%1') does not contain appVersion ('%2')")
                                  .arg(win->title(), controller.appVersion()));
        }
        auto* tbQuick = win->findChild<QQuickItem*>(QStringLiteral("mainTabBar"));
        auto* boxQuick = win->findChild<QQuickItem*>(QStringLiteral("tabContentBox"));
        if (!tbQuick || !boxQuick) {
            violations.append(QStringLiteral("Missing mainTabBar or attached tabContentBox"));
        } else {
            double gap = qAbs((tbQuick->y() + tbQuick->height()) - boxQuick->y());
            if (gap > 2.0) {
                violations.append(QStringLiteral("mainTabBar is not attached directly to tabContentBox (gap=%1px)")
                                      .arg(gap, 0, 'f', 1));
            }
        }
        QObject* tbItem = win->findChild<QObject*>(QStringLiteral("mainTabBar"));
        if (tbItem) {
            QObject* ci = tbItem->property("contentItem").value<QObject*>();
            double cw = ci ? ci->property("contentWidth").toDouble() : tbItem->property("implicitWidth").toDouble();
            double tw = tbItem->property("width").toDouble();
            if (cw > tw + 2.0) {
                violations.append(QStringLiteral("Default window width (%1px) is too narrow for all 5 tabs (%2px > %3px)")
                                      .arg(win->width()).arg(cw, 0, 'f', 1).arg(tw, 0, 'f', 1));
            }
        }

        const QPair<int, int> sizes[] = {{760, 540}, {1220, 780}, {1360, 860}};
        const QString schemes[] = {QStringLiteral("dark"), QStringLiteral("light")};
        const int zooms[] = {80, 100, 130};
        for (const auto& sz : sizes) {
            win->setWidth(sz.first);
            win->setHeight(sz.second);
            for (const QString& scheme : schemes) {
                controller.setColorScheme(scheme);
                for (int zoom : zooms) {
                    controller.setUiZoomPercent(zoom);
                    for (int tabIdx = 0; tabIdx < 5; ++tabIdx) {
                        controller.setActiveTabIndex(tabIdx);
                        app.processEvents();
                        if (QQuickItem* content = win->contentItem()) {
                            QString ctx = QStringLiteral("%1x%2 scheme=%3 zoom=%4% tab=%5")
                                              .arg(sz.first).arg(sz.second).arg(scheme).arg(zoom).arg(tabIdx);
                            checkTree(content, ctx, (sz.first >= 1220 && zoom <= 100), violations);
                        }
                    }
                }
            }
        }

        if (!violations.isEmpty()) {
            fprintf(stderr, "UI_TEST_FAILED:\n%s\n", violations.mid(0, 25).join('\n').toUtf8().constData());
            return 1;
        }
        printf("UI_TEST_OK: 0 layout, TabBar, or text overflow violations across all window sizes (760x540, 1220x780, 1360x860), Dark/Light themes, zoom levels (80%%, 100%%, 130%%), and all 5 tabs (including About & udev).\n");
        return 0;
    }

    return app.exec();
}
