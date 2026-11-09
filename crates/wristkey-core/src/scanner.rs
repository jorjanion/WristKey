use crate::continuity::{AppleContinuityFrame, APPLE_COMPANY_ID};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::{error, info, trace};
use std::time::Instant;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("Failed to initialize Windows Bluetooth watcher: {0}")]
    WinRtError(String),
    #[error("Scanner is already running")]
    AlreadyRunning,
    #[error("Platform not supported for BLE scanning")]
    UnsupportedPlatform,
}

/// An advertisement packet received from Bluetooth Low Energy.
#[derive(Debug, Clone)]
pub struct BleAdvertisementEvent {
    /// 48-bit Bluetooth MAC address.
    pub mac: u64,
    /// Received Signal Strength Indicator in dBm.
    pub rssi: i16,
    /// Parsed Apple Continuity data if present (Company ID 0x004C).
    pub apple_frame: Option<AppleContinuityFrame>,
    /// Instant when the packet was captured.
    pub timestamp: Instant,
}

/// Background BLE Scanner using Windows WinRT APIs.
pub struct BleScanner {
    event_sender: Option<Sender<BleAdvertisementEvent>>,
    #[cfg(windows)]
    watcher: Option<windows::Devices::Bluetooth::Advertisement::BluetoothLEAdvertisementWatcher>,
    is_running: bool,
}

impl Default for BleScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl BleScanner {
    pub fn new() -> Self {
        Self {
            event_sender: None,
            #[cfg(windows)]
            watcher: None,
            is_running: false,
        }
    }

    /// Starts scanning for BLE advertisements in active mode.
    /// Returns a crossbeam Receiver channel that emits events.
    pub fn start(&mut self) -> Result<Receiver<BleAdvertisementEvent>, ScannerError> {
        if self.is_running {
            return Err(ScannerError::AlreadyRunning);
        }

        let (sender, receiver) = unbounded::<BleAdvertisementEvent>();
        self.event_sender = Some(sender.clone());

        #[cfg(windows)]
        {
            use windows::Devices::Bluetooth::Advertisement::*;
            use windows::Foundation::TypedEventHandler;
            use windows::Storage::Streams::DataReader;

            let watcher = BluetoothLEAdvertisementWatcher::new()
                .map_err(|e| ScannerError::WinRtError(e.to_string()))?;

            // Active scanning prompts peripherals for scan response packets (more metadata)
            watcher
                .SetScanningMode(BluetoothLEScanningMode::Active)
                .map_err(|e| ScannerError::WinRtError(e.to_string()))?;

            let tx = sender;
            watcher
                .Received(&TypedEventHandler::new(
                    move |_sender, args: windows::core::Ref<BluetoothLEAdvertisementReceivedEventArgs>| {
                        if let Some(args) = args.as_ref() {
                            let mac = args.BluetoothAddress().unwrap_or(0);
                            let rssi = args.RawSignalStrengthInDBm().unwrap_or(-100);

                            let mut apple_frame: Option<AppleContinuityFrame> = None;

                            if let Ok(ad) = args.Advertisement() {
                                if let Ok(sections) = ad.ManufacturerData() {
                                    let count = sections.Size().unwrap_or(0);
                                    for i in 0..count {
                                        if let Ok(section) = sections.GetAt(i) {
                                            if let Ok(company_id) = section.CompanyId() {
                                                if company_id == APPLE_COMPANY_ID {
                                                    if let Ok(buffer) = section.Data() {
                                                        if let Ok(reader) =
                                                            DataReader::FromBuffer(&buffer)
                                                        {
                                                            let len = buffer.Length().unwrap_or(0)
                                                                as usize;
                                                            let mut raw = vec![0u8; len];
                                                            if reader.ReadBytes(&mut raw).is_ok() {
                                                                trace!(
                                                                    "Apple payload from {:012X} (len {}): {:02X?}",
                                                                    mac,
                                                                    len,
                                                                    raw
                                                                );
                                                                if let Some(parsed) =
                                                                    AppleContinuityFrame::parse(&raw)
                                                                {
                                                                    let replace = match &apple_frame {
                                                                        None => true,
                                                                        Some(current) => {
                                                                            !current.is_confirmed_apple_watch()
                                                                                && (parsed.is_confirmed_apple_watch()
                                                                                    || parsed.is_apple_watch())
                                                                        }
                                                                    };
                                                                    if replace {
                                                                        apple_frame = Some(parsed);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            let event = BleAdvertisementEvent {
                                mac,
                                rssi,
                                apple_frame,
                                timestamp: Instant::now(),
                            };

                            let _ = tx.send(event);
                        }
                        Ok(())
                    },
                ))
                .map_err(|e| ScannerError::WinRtError(e.to_string()))?;

            watcher
                .Start()
                .map_err(|e| ScannerError::WinRtError(e.to_string()))?;

            info!("Windows BLE AdvertisementWatcher started (Active Scanning mode).");
            self.watcher = Some(watcher);
            self.is_running = true;
        }

        #[cfg(not(windows))]
        {
            info!("[MOCK] BleScanner started on non-windows platform.");
            self.is_running = true;
        }

        Ok(receiver)
    }

    /// Stops the Bluetooth scanner.
    pub fn stop(&mut self) {
        if !self.is_running {
            return;
        }

        #[cfg(windows)]
        {
            if let Some(watcher) = self.watcher.take() {
                if let Err(e) = watcher.Stop() {
                    error!("Error stopping BluetoothLEAdvertisementWatcher: {}", e);
                } else {
                    info!("BluetoothLEAdvertisementWatcher stopped.");
                }
            }
        }

        self.event_sender = None;
        self.is_running = false;
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

impl Drop for BleScanner {
    fn drop(&mut self) {
        self.stop();
    }
}
