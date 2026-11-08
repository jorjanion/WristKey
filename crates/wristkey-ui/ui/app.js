// WristKey Fluent UI - Application Logic

const tauriInvoke = window.__TAURI__?.core?.invoke || window.__TAURI__?.invoke || null;

// Application State
let currentStep = 1;
let isScanning = true;
let selectedMac = null;
let configuredIrk = "";
let unlockThreshold = -62;
let lockThreshold = -82;
let currentRssi = -65.0;

document.addEventListener("DOMContentLoaded", () => {
    initNavigation();
    initScannerControls();
    initCalibrationSliders();
    initVaultHandlers();
    initQuickLock();

    // Start simulated or live status updates
    startLiveUpdates();
});

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
    const deviceList = document.getElementById("device-list");

    btnToggle.addEventListener("click", () => {
        isScanning = !isScanning;
        if (isScanning) {
            btnToggle.textContent = "Scanning...";
            btnToggle.classList.replace("btn-secondary", "btn-primary");
            scanStatusText.textContent = "Listening for Apple Continuity advertisements";
            document.querySelector(".radar-sweep").style.display = "block";
        } else {
            btnToggle.textContent = "Resume Scan";
            btnToggle.classList.replace("btn-primary", "btn-secondary");
            scanStatusText.textContent = "Scan paused";
            document.querySelector(".radar-sweep").style.display = "none";
        }
    });

    // Populate mock / live devices
    setTimeout(() => {
        renderDiscoveredDevices([
            {
                mac: "E3:42:19:B1:0A:94",
                name: "Apple Watch (Nearby Info 0x10)",
                rssi: -64,
                onWrist: true,
                autoUnlock: true
            },
            {
                mac: "C8:90:7A:41:2F:10",
                name: "iPhone (AirDrop / Handoff)",
                rssi: -78,
                onWrist: false,
                autoUnlock: false
            }
        ]);
    }, 1200);
}

function renderDiscoveredDevices(devices) {
    const list = document.getElementById("device-list");
    list.innerHTML = "";

    devices.forEach((dev, idx) => {
        const item = document.createElement("div");
        item.className = `device-item ${idx === 0 ? "selected" : ""}`;
        if (idx === 0) selectedMac = dev.mac;

        item.innerHTML = `
            <div>
                <strong>${dev.name}</strong>
                <div style="font-size: 11.5px; color: var(--text-secondary);">${dev.mac}</div>
            </div>
            <div style="text-align: right;">
                <span style="color: ${dev.onWrist ? 'var(--success)' : 'var(--accent)'}; font-weight: 600;">
                    ${dev.rssi} dBm
                </span>
                <div style="font-size: 11px; color: ${dev.onWrist ? 'var(--success)' : 'var(--text-secondary)'};">
                    ${dev.onWrist ? '● On Wrist & Unlocked' : 'In Proximity'}
                </div>
            </div>
        `;

        item.addEventListener("click", () => {
            document.querySelectorAll(".device-item").forEach(el => el.classList.remove("selected"));
            item.classList.add("selected");
            selectedMac = dev.mac;
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
        btnImportReg.textContent = "Querying Registry (BTHPORT)...";
        if (tauriInvoke) {
            try {
                const keys = await tauriInvoke("query_registry_keys");
                displayRegistryKeys(keys);
            } catch (err) {
                regResults.innerHTML = `<span style="color: var(--danger)">${err}</span>`;
            }
        } else {
            // Simulated response in browser mode
            setTimeout(() => {
                displayRegistryKeys([
                    {
                        device_mac: "d8b377c8e9f2",
                        host_adapter_mac: "001a7dda710a",
                        irk_hex: "ec0234a357c8ad05341010a60a397d9b",
                        name: "Apple Watch Series 9"
                    }
                ]);
                btnImportReg.textContent = "⚡ Inspect Paired Keys from Registry";
            }, 500);
        }
    });
}

function displayRegistryKeys(keys) {
    if (!keys || keys.length === 0) {
        regResults.innerHTML = `<span style="color: var(--warning)">No paired BLE keys found or elevation required.</span>`;
        return;
    }

    regResults.innerHTML = keys.map(k => `
        <div style="background: rgba(255,255,255,0.06); padding: 10px; border-radius: 6px; margin-top: 8px;">
            <strong>${k.name || 'Apple Device'} (${k.device_mac})</strong>
            <div style="font-family: monospace; font-size: 11.5px; color: var(--accent); margin: 4px 0;">
                IRK: ${k.irk_hex || 'Not present'}
            </div>
            ${k.irk_hex ? `<button class="btn btn-primary" style="padding: 4px 10px; font-size: 11px;" onclick="selectIrk('${k.irk_hex}')">Use This IRK</button>` : ''}
        </div>
    `).join("");
}

window.selectIrk = function(hex) {
    inputIrk.value = hex;
    configuredIrk = hex;
    irkFeedback.innerHTML = `<span style="color: var(--success)">✓ IRK loaded from registry (${hex.slice(0, 10)}...)</span>`;
};

if (btnVerifyIrk) {
    btnVerifyIrk.addEventListener("click", async () => {
        const val = inputIrk.value.trim();
        if (val.length !== 32) {
            irkFeedback.innerHTML = `<span style="color: var(--danger)">IRK must be 32 hexadecimal characters (16 bytes).</span>`;
            return;
        }

        if (tauriInvoke) {
            try {
                const res = await tauriInvoke("verify_irk", { irk: val });
                irkFeedback.innerHTML = `<span style="color: var(--success)">✓ Valid IRK cryptographic key! AES-128 verified.</span>`;
                configuredIrk = val;
            } catch (err) {
                irkFeedback.innerHTML = `<span style="color: var(--danger)">Error: ${err}</span>`;
            }
        } else {
            irkFeedback.innerHTML = `<span style="color: var(--success)">✓ Valid IRK! AES-128 resolution confirmed.</span>`;
            configuredIrk = val;
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
            vaultFeedback.innerHTML = `<span style="color: var(--danger)">Please provide username and password/PIN.</span>`;
            return;
        }

        if (tauriInvoke) {
            try {
                await tauriInvoke("save_credentials", { domain, username, password });
                vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ Saved securely to Windows DPAPI vault!</span>`;
            } catch (err) {
                vaultFeedback.innerHTML = `<span style="color: var(--danger)">${err}</span>`;
            }
        } else {
            vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ Credentials protected with DPAPI (CryptProtectData).</span>`;
        }
    });

    btnTestVault.addEventListener("click", async () => {
        if (tauriInvoke) {
            try {
                const res = await tauriInvoke("test_vault");
                vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ Vault decrypted: ${res.username} (${res.domain})</span>`;
            } catch (err) {
                vaultFeedback.innerHTML = `<span style="color: var(--danger)">Vault error: ${err}</span>`;
            }
        } else {
            vaultFeedback.innerHTML = `<span style="color: var(--success)">✓ DPAPI Decryption successful: user verified.</span>`;
        }
    });
}

function initQuickLock() {
    const btnLock = document.getElementById("btn-quick-lock");
    btnLock.addEventListener("click", async () => {
        if (tauriInvoke) {
            await tauriInvoke("test_workstation_lock");
        } else {
            alert("[WristKey] user32::LockWorkStation() executed! Screen locked.");
        }
    });
}

// Live Signal Simulation & Updates
function startLiveUpdates() {
    setInterval(() => {
        if (!isScanning) return;

        // Slight jitter to simulate real RSSI
        const jitter = (Math.random() - 0.5) * 3.5;
        currentRssi = Math.round((currentRssi + jitter) * 10) / 10;
        currentRssi = Math.max(-95, Math.min(-45, currentRssi));

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
                elState.textContent = "NEAR (Ready to Unlock)";
                elState.className = "tag-near";
            } else if (currentRssi <= lockThreshold) {
                elState.textContent = "FAR (Auto-Lock Zone)";
                elState.className = "tag-far";
                elState.style.color = "var(--danger)";
            } else {
                elState.textContent = "INTERMEDIATE (Hysteresis Gap)";
                elState.className = "tag-mid";
                elState.style.color = "var(--warning)";
            }
        }

        // Update Dashboard Values
        const dashRssi = document.getElementById("dash-rssi");
        const dashDist = document.getElementById("dash-dist");
        if (dashRssi) dashRssi.textContent = `${currentRssi.toFixed(1)} dBm`;
        if (dashDist) dashDist.textContent = `~${dist.toFixed(1)} meters`;
    }, 800);
}
