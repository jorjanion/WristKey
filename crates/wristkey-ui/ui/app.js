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
let selectedMac = null;
let configuredIrk = "";
let unlockThreshold = -62;
let lockThreshold = -82;
let currentRssi = -65.0;
let scanInterval = null;

document.addEventListener("DOMContentLoaded", () => {
    initNavigation();
    initScannerControls();
    initCalibrationSliders();
    initVaultHandlers();
    initQuickLock();

    // Check for updates on startup
    checkForUpdates();

    // Start live device polling and status updates
    startLiveScannerPolling();
    startLiveUpdates();
});

// Auto-Updater
async function checkForUpdates() {
    try {
        const update = await invoke("check_for_updates");
        if (update) {
            showUpdateBanner(update);
        }
    } catch (e) {
        console.log("Update check info:", e);
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
    if (rssi >= -50) {
        return {
            distance: "Al lado del PC (< 0.5 m)",
            quality: "Señal excelente",
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
    } else if (rssi >= -78) {
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

// Step Navigation
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

// Scanner & Discovery
function initScannerControls() {
    const btnToggle = document.getElementById("btn-toggle-scan");
    const scanStatusText = document.getElementById("scan-status-text");

    btnToggle.addEventListener("click", () => {
        isScanning = !isScanning;
        if (isScanning) {
            btnToggle.textContent = "Buscando...";
            btnToggle.classList.replace("btn-secondary", "btn-primary");
            scanStatusText.textContent = "Escuchando emisiones Bluetooth de dispositivos Apple";
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
    }, 800);
}

function renderDiscoveredDevices(devices) {
    const list = document.getElementById("device-list");
    if (!list) return;

    if (!devices || devices.length === 0) {
        list.innerHTML = `
            <div class="empty-state">
                <div style="font-size: 14px; font-weight: 500; margin-bottom: 6px;">Buscando tu Apple Watch...</div>
                <div style="font-size: 12px; color: var(--text-secondary); max-width: 440px; margin: 0 auto; line-height: 1.6;">
                    Coloca tu reloj cerca del ordenador o en tu muñeca. Toca la pantalla para que emita señal Bluetooth.
                </div>
            </div>
        `;
        return;
    }

    list.innerHTML = "";

    devices.forEach((dev, idx) => {
        const item = document.createElement("div");
        const isSelected = selectedMac === dev.mac || (!selectedMac && idx === 0);
        if (isSelected && !selectedMac) {
            selectedMac = dev.mac;
            currentRssi = dev.rssi;
        }

        const signal = getHumanSignal(dev.rssi);
        const isClosest = idx === 0 && dev.rssi >= -65;

        item.className = `device-item ${isSelected ? "selected" : ""} ${isClosest ? "is-closest" : ""}`;

        item.innerHTML = `
            <div>
                <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 3px;">
                    <span class="badge-watch">⌚ ${dev.name}</span>
                    ${isClosest ? '<span class="badge-closest">⭐ MÁS CERCANO (Al lado de tu PC)</span>' : ''}
                </div>
                <div style="font-size: 11.5px; color: var(--text-secondary); font-family: monospace;">${dev.mac}</div>
            </div>
            <div style="text-align: right;">
                <div class="proximity-tag ${signal.colorClass}">
                    <span>${signal.distance}</span>
                </div>
                <span class="signal-dbm-pill">${signal.quality} · ${dev.rssi} dBm</span>
            </div>
        `;

        item.addEventListener("click", () => {
            document.querySelectorAll(".device-item").forEach(el => el.classList.remove("selected"));
            item.classList.add("selected");
            selectedMac = dev.mac;
            currentRssi = dev.rssi;
        });

        list.appendChild(item);
    });
}

// Step 2: IRK Pairing
const btnImportReg = document.getElementById("btn-import-registry");
const regResults = document.getElementById("registry-results");
const inputIrk = document.getElementById("input-irk");
const btnVerifyIrk = document.getElementById("btn-verify-irk");
const irkFeedback = document.getElementById("irk-verification-feedback");

if (btnImportReg) {
    btnImportReg.addEventListener("click", async () => {
        btnImportReg.textContent = "Consultando registro (BTHPORT)...";
        try {
            const keys = await invoke("query_registry_keys");
            displayRegistryKeys(keys);
        } catch (err) {
            regResults.innerHTML = `
                <div style="background: rgba(255, 75, 75, 0.1); border: 1px solid rgba(255, 75, 75, 0.3); padding: 12px; border-radius: 6px; margin-top: 10px;">
                    <strong style="color: var(--danger); font-size: 12.5px;">Acceso restringido al registro</strong>
                    <div style="font-size: 11.5px; color: var(--text-secondary); margin-top: 4px;">
                        Windows requiere permisos de Administrador para leer claves Bluetooth emparejadas. Ejecuta WristKey como Administrador o introduce la clave IRK manualmente abajo.
                    </div>
                </div>
            `;
        } finally {
            btnImportReg.textContent = "⚡ Inspeccionar claves emparejadas";
        }
    });
}

function displayRegistryKeys(keys) {
    if (!keys || keys.length === 0) {
        regResults.innerHTML = `
            <div style="background: rgba(255, 185, 0, 0.1); border: 1px solid rgba(255, 185, 0, 0.3); padding: 12px; border-radius: 6px; margin-top: 10px;">
                <strong style="color: var(--warning); font-size: 12.5px;">No se encontraron claves de Apple Watch emparejadas en Windows.</strong>
                <div style="font-size: 11.5px; color: var(--text-secondary); margin-top: 4px;">
                    Si no has vinculado el reloj en Configuración de Windows > Bluetooth, puedes introducir la clave IRK manualmente en la Opción B.
                </div>
            </div>
        `;
        return;
    }

    regResults.innerHTML = keys.map(k => `
        <div style="background: rgba(255,255,255,0.06); padding: 10px; border-radius: 6px; margin-top: 8px;">
            <strong>${k.name || 'Dispositivo Apple'} (${k.device_mac})</strong>
            <div style="font-family: monospace; font-size: 11.5px; color: var(--accent); margin: 4px 0;">
                IRK: ${k.irk_hex || 'No disponible'}
            </div>
            ${k.irk_hex ? `<button class="btn btn-primary" style="padding: 4px 10px; font-size: 11px;" onclick="selectIrk('${k.irk_hex}')">Usar esta IRK</button>` : ''}
        </div>
    `).join("");
}

window.selectIrk = function(hex) {
    inputIrk.value = hex;
    configuredIrk = hex;
    irkFeedback.innerHTML = `<span style="color: var(--success)">✓ Clave IRK cargada (${hex.slice(0, 10)}...)</span>`;
};

if (btnVerifyIrk) {
    btnVerifyIrk.addEventListener("click", async () => {
        const val = inputIrk.value.trim();
        if (val.length !== 32) {
            irkFeedback.innerHTML = `<span style="color: var(--danger)">La clave IRK debe tener 32 caracteres hexadecimales (16 bytes).</span>`;
            return;
        }

        try {
            await invoke("verify_irk", { irk: val });
            irkFeedback.innerHTML = `<span style="color: var(--success)">✓ Clave IRK válida (Criptografía AES-128 verificada).</span>`;
            configuredIrk = val;
        } catch (err) {
            irkFeedback.innerHTML = `<span style="color: var(--danger)">Error: ${err}</span>`;
        }
    });
}

// Step 3: Distance Calibration
function initCalibrationSliders() {
    const sliderUnlock = document.getElementById("slider-unlock");
    const sliderLock = document.getElementById("slider-lock");
    const valUnlock = document.getElementById("val-unlock-dbm");
    const valLock = document.getElementById("val-lock-dbm");

    sliderUnlock.addEventListener("input", (e) => {
        unlockThreshold = parseInt(e.target.value);
        valUnlock.textContent = `${unlockThreshold} dBm`;
    });

    sliderLock.addEventListener("input", (e) => {
        lockThreshold = parseInt(e.target.value);
        valLock.textContent = `${lockThreshold} dBm`;
    });
}

// Step 4: Vault Handlers
function initVaultHandlers() {
    const btnSaveVault = document.getElementById("btn-save-vault");
    const btnTestVault = document.getElementById("btn-test-vault");
    const vaultFeedback = document.getElementById("vault-feedback");

    btnSaveVault.addEventListener("click", async () => {
        const domain = document.getElementById("vault-domain").value.trim() || ".";
        const username = document.getElementById("vault-username").value.trim();
        const password = document.getElementById("vault-password").value;

        if (!username || !password) {
            vaultFeedback.innerHTML = `<span style="color: var(--danger)">Introduce usuario y contraseña/PIN.</span>`;
            return;
        }

        try {
            await invoke("save_credentials", { domain, username, password });
            vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ Guardado seguro en la bóveda DPAPI de Windows.</span>`;
        } catch (err) {
            vaultFeedback.innerHTML = `<span style="color: var(--danger)">${err}</span>`;
        }
    });

    btnTestVault.addEventListener("click", async () => {
        try {
            const res = await invoke("test_vault");
            vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ Bóveda verificada: ${res.username} (${res.domain})</span>`;
        } catch (err) {
            vaultFeedback.innerHTML = `<span style="color: var(--danger)">Error en la bóveda: ${err}</span>`;
        }
    });
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

// Live Signal Updates
function startLiveUpdates() {
    setInterval(async () => {
        if (!isScanning) return;

        if (selectedMac) {
            try {
                const liveRssi = await invoke("get_device_rssi", { mac: selectedMac });
                if (liveRssi !== null && liveRssi !== undefined) {
                    currentRssi = liveRssi;
                }
            } catch (_) {}
        }

        // Update Calibration Meter
        const elRssi = document.getElementById("cal-rssi-value");
        const elBar = document.getElementById("cal-meter-bar");
        const elDist = document.getElementById("cal-dist-est");
        const elState = document.getElementById("cal-state-tag");

        if (elRssi) elRssi.textContent = currentRssi.toFixed(1);

        // Convert RSSI (-100 to -40) to percentage
        const pct = Math.min(100, Math.max(0, ((currentRssi - (-100)) / 60) * 100));
        if (elBar) elBar.style.width = `${pct}%`;

        // Calculate distance
        const ratio = (-59 - currentRssi) / (10 * 2.5);
        const dist = Math.pow(10, ratio);
        if (elDist) elDist.textContent = `~${dist.toFixed(1)}m`;

        if (elState) {
            if (currentRssi >= unlockThreshold) {
                elState.textContent = "CERCA (Listo para desbloquear)";
                elState.className = "tag-near";
            } else if (currentRssi <= lockThreshold) {
                elState.textContent = "LEJOS (Zona de auto-bloqueo)";
                elState.className = "tag-far";
                elState.style.color = "var(--danger)";
            } else {
                elState.textContent = "ZONA MEDIA (Histéresis)";
                elState.className = "tag-mid";
                elState.style.color = "var(--warning)";
            }
        }

        // Update Dashboard Values
        const dashRssi = document.getElementById("dash-rssi");
        const dashDist = document.getElementById("dash-dist");
        if (dashRssi) dashRssi.textContent = `${currentRssi.toFixed(1)} dBm`;
        if (dashDist) dashDist.textContent = `~${dist.toFixed(1)} m`;
    }, 800);
}
