// WristKey Fluent UI - Application Logic

async function invoke(cmd, args = {}) {
    if (window.__TAURI__?.core?.invoke) {
        return await window.__TAURI__.core.invoke(cmd, args);
    }
    if (window.__TAURI__?.invoke) {
        return await window.__TAURI__.invoke(cmd, args);
    }
    console.warn("Tauri invoke API not detected, running in web mode");
    throw new Error("Tauri API not available");
}

// Application State
let currentStep = 1;
let isScanning = true;
let selectedMac = localStorage.getItem("wristkey_selected_mac") || null;
let unlockThreshold = parseInt(localStorage.getItem("wristkey_unlock_thresh")) || -58;
let lockThreshold = parseInt(localStorage.getItem("wristkey_lock_thresh")) || -74;
let currentRssi = -58.0;
let scanInterval = null;
let isWorkstationLocked = false;
let isSetupCompleted = localStorage.getItem("wristkey_setup_completed") === "true";
let autoLockEnabled = localStorage.getItem("wristkey_auto_lock") !== "false";
let autoUnlockEnabled = localStorage.getItem("wristkey_auto_unlock") !== "false";
let requireWristEnabled = localStorage.getItem("wristkey_require_wrist") !== "false";

async function syncEngineSettings() {
    try {
        await invoke("update_engine_settings", {
            newSettings: {
                auto_lock_enabled: autoLockEnabled,
                auto_unlock_enabled: autoUnlockEnabled,
                require_on_wrist: requireWristEnabled,
                lock_threshold_dbm: lockThreshold,
                unlock_threshold_dbm: unlockThreshold,
                tracked_mac: selectedMac
            }
        });
    } catch (_) {}
}

async function loadEngineSettings() {
    try {
        const s = await invoke("get_engine_settings");
        if (s) {
            autoLockEnabled = s.auto_lock_enabled;
            autoUnlockEnabled = s.auto_unlock_enabled;
            requireWristEnabled = s.require_on_wrist;
            lockThreshold = s.lock_threshold_dbm;
            unlockThreshold = s.unlock_threshold_dbm;
            if (s.tracked_mac) {
                selectedMac = s.tracked_mac;
                localStorage.setItem("wristkey_selected_mac", selectedMac);
            }
            localStorage.setItem("wristkey_auto_lock", autoLockEnabled);
            localStorage.setItem("wristkey_auto_unlock", autoUnlockEnabled);
            localStorage.setItem("wristkey_require_wrist", requireWristEnabled);
            localStorage.setItem("wristkey_lock_thresh", lockThreshold);
            localStorage.setItem("wristkey_unlock_thresh", unlockThreshold);

            const chkAutoLock = document.getElementById("chk-auto-lock");
            const chkAutoUnlock = document.getElementById("chk-auto-unlock");
            const chkRequireWrist = document.getElementById("chk-require-wrist");
            if (chkAutoLock) chkAutoLock.checked = autoLockEnabled;
            if (chkAutoUnlock) chkAutoUnlock.checked = autoUnlockEnabled;
            if (chkRequireWrist) chkRequireWrist.checked = requireWristEnabled;

            const sliderUnlock = document.getElementById("slider-unlock");
            const sliderLock = document.getElementById("slider-lock");
            const valUnlock = document.getElementById("val-unlock-dbm");
            const valLock = document.getElementById("val-lock-dbm");
            if (sliderUnlock) sliderUnlock.value = unlockThreshold;
            if (valUnlock) valUnlock.textContent = `${unlockThreshold} dBm`;
            if (sliderLock) sliderLock.value = lockThreshold;
            if (valLock) valLock.textContent = `${lockThreshold} dBm`;
            updateThresholdMarkers();
        }
    } catch (_) {}
}

document.addEventListener("DOMContentLoaded", () => {
    initNavigation();
    initScannerControls();
    initCalibrationSliders();
    initVaultHandlers();
    initDashboardHandlers();
    initQuickLock();
    loadEngineSettings();

    // If configuration was previously completed, jump directly to the live Dashboard!
    if (isSetupCompleted) {
        goToStep(4);
    }

    // Start live device polling and status updates immediately (instant startup)
    startLiveScannerPolling();
    startLiveUpdates();

    // Check for updates silently in the background 3 seconds after launch
    setTimeout(checkForUpdates, 3000);
});

// Auto-Updater (Silently checks GitHub releases via native fetch; zero process spawns, no CMD window, instant startup)
async function checkForUpdates() {
    try {
        const response = await fetch("https://api.github.com/repos/jorjanion/WristKey/releases/latest", {
            headers: { "Accept": "application/vnd.github.v3+json" }
        });
        if (response.ok) {
            const release = await response.json();
            const currentVersion = "0.4.16";
            const tag = release.tag_name || "";
            const latestClean = tag.replace(/^v/, "");

            if (latestClean && latestClean !== currentVersion) {
                let downloadUrl = release.html_url || "";
                if (Array.isArray(release.assets)) {
                    for (const asset of release.assets) {
                        if (asset.name && asset.name.endsWith(".exe")) {
                            downloadUrl = asset.browser_download_url;
                            break;
                        }
                    }
                }

                showUpdateBanner({
                    current_version: currentVersion,
                    latest_version: tag,
                    download_url: downloadUrl,
                    release_notes: release.body || ""
                });
                return;
            }
        }
    } catch (_) {
        // Fallback to background invoke completely silently
        try {
            const update = await invoke("check_for_updates");
            if (update) showUpdateBanner(update);
        } catch (_) {}
    }
}

function showUpdateBanner(update) {
    const banner = document.getElementById("update-banner");
    const versionTag = document.getElementById("update-version-tag");
    const btnUpdate = document.getElementById("btn-update-now");
    const btnDismiss = document.getElementById("btn-dismiss-update");

    if (!banner || !versionTag) return;

    versionTag.textContent = update.latest_version;
    banner.style.display = "flex";

    btnUpdate.onclick = async () => {
        btnUpdate.disabled = true;
        btnUpdate.textContent = "Descargando e instalando...";
        try {
            await invoke("download_and_install_update", { downloadUrl: update.download_url });
        } catch (err) {
            alert("Error al descargar actualización: " + err);
            btnUpdate.disabled = false;
            btnUpdate.textContent = "Reintentar";
        }
    };

    btnDismiss.onclick = () => {
        banner.style.display = "none";
    };
}

// Convert raw dBm into everyday human-readable proximity
function getHumanSignal(rssi) {
    if (rssi >= -52) {
        return {
            distance: "Al lado del PC (< 0.5 m)",
            quality: "Señal óptima",
            colorClass: "proximity-very-near",
            bars: "📶 Fuerte"
        };
    } else if (rssi >= -65) {
        return {
            distance: "En el escritorio (~1 m)",
            quality: "Señal buena",
            colorClass: "proximity-near",
            bars: "📶 Buena"
        };
    } else if (rssi >= -76) {
        return {
            distance: "En la habitación (~2-3 m)",
            quality: "Señal media",
            colorClass: "proximity-mid",
            bars: "📶 Media"
        };
    } else {
        return {
            distance: "Lejos (> 4 m)",
            quality: "Señal débil",
            colorClass: "proximity-far",
            bars: "📶 Lejos"
        };
    }
}

// Step Navigation (Streamlined 4-Step Wizard)
function initNavigation() {
    const navItems = document.querySelectorAll(".nav-item");
    const nextButtons = document.querySelectorAll(".btn-next");
    const prevButtons = document.querySelectorAll(".btn-prev");

    navItems.forEach(btn => {
        btn.addEventListener("click", () => {
            const step = parseInt(btn.dataset.step);
            goToStep(step);
        });
    });

    nextButtons.forEach(btn => {
        btn.addEventListener("click", () => {
            const next = parseInt(btn.dataset.next);
            goToStep(next);
        });
    });

    prevButtons.forEach(btn => {
        btn.addEventListener("click", () => {
            const prev = parseInt(btn.dataset.prev);
            goToStep(prev);
        });
    });
}

function goToStep(step) {
    currentStep = step;
    document.querySelectorAll(".wizard-step").forEach(el => el.classList.remove("active"));
    document.querySelectorAll(".nav-item").forEach(el => el.classList.remove("active"));

    const targetStep = document.getElementById(`step-${step}`);
    const targetNav = document.querySelector(`.nav-item[data-step="${step}"]`);

    if (targetStep) targetStep.classList.add("active");
    if (targetNav) targetNav.classList.add("active");
}

// Step 1: Scanner & Discovery
function initScannerControls() {
    const btnToggle = document.getElementById("btn-toggle-scan");
    const scanStatusText = document.getElementById("scan-status-text");

    btnToggle.addEventListener("click", () => {
        isScanning = !isScanning;
        if (isScanning) {
            btnToggle.textContent = "Buscando...";
            btnToggle.classList.replace("btn-secondary", "btn-primary");
            scanStatusText.textContent = "Escuchando emisiones Bluetooth de tu Apple Watch en muñeca";
            document.querySelector(".radar-sweep").style.display = "block";
        } else {
            btnToggle.textContent = "Reanudar búsqueda";
            btnToggle.classList.replace("btn-primary", "btn-secondary");
            scanStatusText.textContent = "Búsqueda en pausa";
            document.querySelector(".radar-sweep").style.display = "none";
        }
    });
}

function startLiveScannerPolling() {
    if (scanInterval) clearInterval(scanInterval);

    scanInterval = setInterval(async () => {
        if (!isScanning) return;
        try {
            const devices = await invoke("get_discovered_devices");
            renderDiscoveredDevices(devices);
        } catch (_) {
            // Scanner starting or web fallback
        }
    }, 700);
}

function renderDiscoveredDevices(devices) {
    const list = document.getElementById("device-list");
    if (!list) return;

    if (!devices || devices.length === 0) {
        list.innerHTML = `
            <div class="empty-state">
                <div style="font-size: 14px; font-weight: 500; margin-bottom: 6px;">Buscando tu Apple Watch en tu muñeca...</div>
                <div style="font-size: 12px; color: var(--text-secondary); max-width: 440px; margin: 0 auto; line-height: 1.6;">
                    Ponte el reloj en la muñeca y desbloquéalo (o toca la pantalla). WristKey filtra automáticamente cualquier otro teléfono o dispositivo de la casa.
                </div>
            </div>
        `;
        return;
    }

    // Sort: highest RSSI first
    devices.sort((a, b) => b.rssi - a.rssi);

    // Auto-select the strongest watch (user's watch at desk) if none selected
    if (!selectedMac || !devices.some(d => d.mac === selectedMac)) {
        selectedMac = devices[0].mac;
        currentRssi = devices[0].rssi;
    }

    // Remove any items that are no longer active
    const activeMacs = new Set(devices.map(d => d.mac));
    list.querySelectorAll(".device-item").forEach(el => {
        const mac = el.getAttribute("data-mac");
        if (mac && !activeMacs.has(mac)) {
            el.remove();
        }
    });

    const empty = list.querySelector(".empty-state");
    if (empty) empty.remove();

    devices.forEach((dev, idx) => {
        const signal = getHumanSignal(dev.rssi);
        const isClosestWatch = idx === 0;

        let item = list.querySelector(`.device-item[data-mac="${dev.mac}"]`);
        if (!item) {
            item = document.createElement("div");
            item.setAttribute("data-mac", dev.mac);
            item.addEventListener("click", () => {
                document.querySelectorAll(".device-item").forEach(el => el.classList.remove("selected"));
                item.classList.add("selected");
                selectedMac = dev.mac;
                currentRssi = dev.rssi;
                localStorage.setItem("wristkey_selected_mac", selectedMac);
                syncEngineSettings();
            });
        }

        const isSelected = selectedMac === dev.mac;
        item.className = `device-item ${isSelected ? "selected" : ""} ${isClosestWatch ? "is-closest" : ""}`;

        const closestBadge = isClosestWatch ? '<span class="badge-closest">⭐ TU APPLE WATCH</span>' : '';

        item.innerHTML = `
            <div>
                <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
                    <span class="badge-watch">⌚ ${dev.name}</span>
                    ${closestBadge}
                </div>
                <div style="font-size: 11.5px; color: var(--text-secondary); font-family: monospace;">${dev.mac} · Detectado en muñeca</div>
            </div>
            <div style="text-align: right;">
                <div class="proximity-tag ${signal.colorClass}">
                    <span>${signal.distance}</span>
                </div>
                <span class="signal-dbm-pill">${signal.quality} · ${dev.rssi} dBm</span>
            </div>
        `;

        list.appendChild(item);
    });
}

// Step 2: Distance Calibration Sliders & Thresholds
function initCalibrationSliders() {
    const sliderUnlock = document.getElementById("slider-unlock");
    const sliderLock = document.getElementById("slider-lock");
    const valUnlock = document.getElementById("val-unlock-dbm");
    const valLock = document.getElementById("val-lock-dbm");

    if (sliderUnlock) {
        sliderUnlock.value = unlockThreshold;
        if (valUnlock) valUnlock.textContent = `${unlockThreshold} dBm`;
        sliderUnlock.addEventListener("input", (e) => {
            unlockThreshold = parseInt(e.target.value);
            localStorage.setItem("wristkey_unlock_thresh", unlockThreshold);
            if (valUnlock) valUnlock.textContent = `${unlockThreshold} dBm`;
            updateThresholdMarkers();
        });
        sliderUnlock.addEventListener("change", () => {
            syncEngineSettings();
        });
    }

    if (sliderLock) {
        sliderLock.value = lockThreshold;
        if (valLock) valLock.textContent = `${lockThreshold} dBm`;
        sliderLock.addEventListener("input", (e) => {
            lockThreshold = parseInt(e.target.value);
            localStorage.setItem("wristkey_lock_thresh", lockThreshold);
            if (valLock) valLock.textContent = `${lockThreshold} dBm`;
            updateThresholdMarkers();
        });
        sliderLock.addEventListener("change", () => {
            syncEngineSettings();
        });
    }

    updateThresholdMarkers();
}

function updateThresholdMarkers() {
    const minRssi = -82;
    const maxRssi = -42;
    const unlockPct = Math.min(100, Math.max(0, ((unlockThreshold - minRssi) / (maxRssi - minRssi)) * 100));
    const lockPct = Math.min(100, Math.max(0, ((lockThreshold - minRssi) / (maxRssi - minRssi)) * 100));

    const markerUnlock = document.getElementById("marker-unlock");
    const markerLock = document.getElementById("marker-lock");

    if (markerUnlock) {
        markerUnlock.style.left = `${unlockPct}%`;
        const lbl = markerUnlock.querySelector(".marker-label");
        if (lbl) lbl.textContent = `🔓 Desbloqueo (${unlockThreshold} dBm)`;
    }
    if (markerLock) {
        markerLock.style.left = `${lockPct}%`;
        const lbl = markerLock.querySelector(".marker-label");
        if (lbl) lbl.textContent = `🔒 Bloqueo (${lockThreshold} dBm)`;
    }
}

// Step 3: Vault Handlers (DPAPI Credential Storage)
function initVaultHandlers() {
    const btnSaveVault = document.getElementById("btn-save-vault");
    const btnTestVault = document.getElementById("btn-test-vault");
    const vaultFeedback = document.getElementById("vault-feedback");

    if (btnSaveVault) {
        btnSaveVault.addEventListener("click", async () => {
            const domain = document.getElementById("vault-domain").value.trim() || ".";
            const username = document.getElementById("vault-username").value.trim();
            const password = document.getElementById("vault-password").value;

            if (!username || !password) {
                vaultFeedback.innerHTML = `<span style="color: var(--danger)">Introduce usuario y contraseña/PIN de Windows.</span>`;
                return;
            }

            try {
                await invoke("save_credentials", { domain, username, password });
                localStorage.setItem("wristkey_setup_completed", "true");
                isSetupCompleted = true;
                vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ Credenciales guardadas y cifradas con DPAPI en Windows.</span>`;
            } catch (err) {
                vaultFeedback.innerHTML = `<span style="color: var(--danger)">${err}</span>`;
            }
        });
    }

    if (btnTestVault) {
        btnTestVault.addEventListener("click", async () => {
            try {
                const res = await invoke("test_vault");
                localStorage.setItem("wristkey_setup_completed", "true");
                isSetupCompleted = true;
                vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ Bóveda verificada correctamente: ${res.username} (${res.domain})</span>`;
            } catch (err) {
                vaultFeedback.innerHTML = `<span style="color: var(--danger)">Error al leer bóveda: ${err}</span>`;
            }
        });
    }

    const btnFinish = document.querySelector('.btn-next[data-next="4"]');
    if (btnFinish) {
        btnFinish.addEventListener("click", () => {
            localStorage.setItem("wristkey_setup_completed", "true");
            isSetupCompleted = true;
        });
    }
}

// Step 4: Dashboard Controls & Persistent Settings
function initDashboardHandlers() {
    const chkAutoLock = document.getElementById("chk-auto-lock");
    const chkAutoUnlock = document.getElementById("chk-auto-unlock");
    const chkRequireWrist = document.getElementById("chk-require-wrist");
    const chkAutostart = document.getElementById("chk-autostart");

    const btnLockNow = document.getElementById("btn-dash-lock-now");
    const btnCalibrate = document.getElementById("btn-dash-calibrate");
    const btnRescan = document.getElementById("btn-dash-rescan");
    const btnReconfig = document.getElementById("btn-reconfig");

    if (chkAutoLock) {
        chkAutoLock.checked = autoLockEnabled;
        chkAutoLock.addEventListener("change", (e) => {
            autoLockEnabled = e.target.checked;
            localStorage.setItem("wristkey_auto_lock", autoLockEnabled);
            syncEngineSettings();
        });
    }

    if (chkAutoUnlock) {
        chkAutoUnlock.checked = autoUnlockEnabled;
        chkAutoUnlock.addEventListener("change", (e) => {
            autoUnlockEnabled = e.target.checked;
            localStorage.setItem("wristkey_auto_unlock", autoUnlockEnabled);
            syncEngineSettings();
        });
    }

    if (chkRequireWrist) {
        chkRequireWrist.checked = requireWristEnabled;
        chkRequireWrist.addEventListener("change", (e) => {
            requireWristEnabled = e.target.checked;
            localStorage.setItem("wristkey_require_wrist", requireWristEnabled);
            syncEngineSettings();
        });
    }

    if (chkAutostart) {
        invoke("get_autostart_enabled")
            .then((enabled) => { chkAutostart.checked = enabled; })
            .catch(() => {});

        chkAutostart.addEventListener("change", async (e) => {
            try {
                await invoke("set_autostart_enabled", { enabled: e.target.checked });
            } catch (err) {
                console.error("Autostart error:", err);
            }
        });
    }

    if (btnLockNow) {
        btnLockNow.addEventListener("click", async () => {
            try {
                await invoke("test_workstation_lock");
            } catch (err) {
                console.error("Lock error:", err);
            }
        });
    }

    if (btnCalibrate) {
        btnCalibrate.addEventListener("click", () => goToStep(2));
    }

    if (btnRescan) {
        btnRescan.addEventListener("click", () => goToStep(1));
    }

    if (btnReconfig) {
        btnReconfig.addEventListener("click", () => goToStep(2));
    }
}

function initQuickLock() {
    const btnLock = document.getElementById("btn-quick-lock");
    if (btnLock) {
        btnLock.addEventListener("click", async () => {
            try {
                await invoke("test_workstation_lock");
            } catch (err) {
                console.error("Lock error:", err);
            }
        });
    }
}

// Step 2 & 4: Live Signal Updates (Steady, Smooth & Highly Accurate)
function startLiveUpdates() {
    setInterval(async () => {
        if (!isScanning) return;

        try {
            const reading = await invoke("get_live_proximity", { mac: selectedMac });
            if (reading) {
                currentRssi = reading.rssi;

                // Update Tracked Device Header
                const elTrackedName = document.getElementById("cal-tracked-name");
                const elTrackedMac = document.getElementById("cal-tracked-mac");
                if (elTrackedName && reading.device_name) elTrackedName.textContent = reading.device_name;
                if (elTrackedMac && reading.current_mac) elTrackedMac.textContent = reading.current_mac;

                // Update Calibration Meter
                const elRssi = document.getElementById("cal-rssi-value");
                const elBar = document.getElementById("cal-meter-bar");
                const elDist = document.getElementById("cal-dist-est");
                const elState = document.getElementById("cal-state-tag");

                if (elRssi) elRssi.textContent = reading.rssi.toFixed(1);
                if (elDist) elDist.textContent = `~${reading.distance_meters.toFixed(1)}m`;

                // Dynamic Percentage: maps -82 dBm (0%) to -42 dBm (100%)
                const minRssi = -82;
                const maxRssi = -42;
                const pct = Math.min(100, Math.max(0, ((reading.rssi - minRssi) / (maxRssi - minRssi)) * 100));
                if (elBar) elBar.style.width = `${pct}%`;

                if (elState) {
                    if (reading.is_lost || reading.rssi <= lockThreshold) {
                        elState.textContent = "LEJOS (Zona de auto-bloqueo)";
                        elState.className = "tag-far";
                        elState.style.color = "var(--danger)";
                    } else if (reading.rssi >= unlockThreshold) {
                        elState.textContent = "CERCA (Escritorio / Desbloqueado)";
                        elState.className = "tag-near";
                        elState.style.color = "var(--success)";
                    } else {
                        elState.textContent = "ZONA MEDIA (En la habitación / Sin cambios)";
                        elState.className = "tag-mid";
                        elState.style.color = "var(--warning)";
                    }
                }

                // Update Dashboard Live Proximity View
                const dashName = document.getElementById("dash-device-name");
                const dashMac = document.getElementById("dash-mac-pill");
                const dashRssiLive = document.getElementById("dash-rssi-live");
                const dashDistMeters = document.getElementById("dash-dist-meters");
                const dashMeterBar = document.getElementById("dash-meter-bar");
                const dashZonePill = document.getElementById("dash-zone-pill");
                const dashLiveStatus = document.getElementById("dash-live-status-text");
                const dashLockVal = document.getElementById("dash-lock-val");
                const dashUnlockVal = document.getElementById("dash-unlock-val");
                const dashLblLock = document.getElementById("dash-lbl-lock-thresh");
                const dashLblUnlock = document.getElementById("dash-lbl-unlock-thresh");
                const dashWrist = document.getElementById("dash-wrist-status");

                if (dashName && reading.device_name) dashName.textContent = reading.device_name;
                if (dashMac && reading.current_mac) dashMac.textContent = reading.current_mac;
                if (dashRssiLive) dashRssiLive.textContent = reading.rssi.toFixed(1);
                if (dashDistMeters) dashDistMeters.textContent = `~${reading.distance_meters.toFixed(1)} m`;
                if (dashMeterBar) dashMeterBar.style.width = `${pct}%`;

                if (dashLockVal) dashLockVal.textContent = `${lockThreshold} dBm`;
                if (dashUnlockVal) dashUnlockVal.textContent = `${unlockThreshold} dBm`;
                if (dashLblLock) dashLblLock.textContent = `${lockThreshold}`;
                if (dashLblUnlock) dashLblUnlock.textContent = `${unlockThreshold}`;

                const humanSig = getHumanSignal(reading.rssi);
                if (dashZonePill) {
                    dashZonePill.className = `proximity-tag ${humanSig.colorClass}`;
                    dashZonePill.innerHTML = `<span>${humanSig.distance}</span>`;
                }

                if (dashWrist) {
                    if (reading.is_confirmed_watch && reading.on_wrist && reading.is_unlocked && !reading.is_lost) {
                        dashWrist.textContent = "En muñeca (Desbloqueado)";
                        dashWrist.className = "dash-val green";
                        dashWrist.style.color = "";
                    } else if (reading.is_confirmed_watch && (!reading.on_wrist || !reading.is_unlocked)) {
                        dashWrist.textContent = "Fuera de muñeca / Bloqueado";
                        dashWrist.className = "dash-val";
                        dashWrist.style.color = "var(--danger)";
                    } else {
                        dashWrist.textContent = "Buscando reloj...";
                        dashWrist.className = "dash-val";
                        dashWrist.style.color = "";
                    }
                }

                if (dashLiveStatus) {
                    if (reading.is_lost || reading.rssi <= lockThreshold) {
                        dashLiveStatus.textContent = "LEJOS (Zona de auto-bloqueo)";
                        dashLiveStatus.style.color = "var(--danger)";
                        if (dashMeterBar) dashMeterBar.style.background = "var(--danger)";
                        isWorkstationLocked = true;
                    } else if (reading.rssi >= unlockThreshold) {
                        dashLiveStatus.textContent = "CERCA (En tu escritorio / Desbloqueado)";
                        dashLiveStatus.style.color = "var(--success)";
                        if (dashMeterBar) dashMeterBar.style.background = "";
                        isWorkstationLocked = false;
                    } else {
                        dashLiveStatus.textContent = "ZONA MEDIA (En la habitación)";
                        dashLiveStatus.style.color = "var(--warning)";
                        if (dashMeterBar) dashMeterBar.style.background = "";
                    }
                }
            }
        } catch (_) {}
    }, 400);
}
