use clap::Parser;
use colored::*;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;
use wristkey_core::{
    format_mac, is_elevated, lock_workstation, read_paired_bluetooth_keys, BleScanner,
    IrkResolver, ProximityState, RssiTracker, WatchPresenceManager, WatchSecurityStatus,
    WristKeyConfig, WristKeyEngine,
};

#[derive(Parser, Debug)]
#[command(
    name = "wristkey-cli",
    author = "Jorge",
    version = "0.2.0",
    about = "WristKey CLI: Apple Watch proximity BLE scanner, IRK pairing, and presence manager"
)]
struct Args {
    /// Scan for all nearby Apple BLE devices in active mode
    #[arg(short, long)]
    scan: bool,

    /// Filter by specific Apple Watch Identity Resolving Key (32-char hex string)
    #[arg(long)]
    irk: Option<String>,

    /// Inspect Windows Registry (BTHPORT Keys) for paired Bluetooth IRKs (requires Administrator/SYSTEM)
    #[arg(long)]
    list_keys: bool,

    /// Stream detailed Apple Continuity telemetry bitfields in real-time
    #[arg(long)]
    telemetry: bool,

    /// Automatically lock PC when device moves out of range or is removed from wrist
    #[arg(long)]
    auto_lock: bool,

    /// Signal threshold in dBm to trigger lock (e.g. -82)
    #[arg(long, default_value_t = -82.0)]
    lock_threshold: f64,

    /// Signal threshold in dBm to consider in proximity (e.g. -62)
    #[arg(long, default_value_t = -62.0)]
    unlock_threshold: f64,

    /// Run calibration mode with real-time ASCII signal meter
    #[arg(short, long)]
    calibrate: bool,

    /// Test Windows LockWorkStation() call immediately and exit
    #[arg(long)]
    test_lock: bool,

    /// Verify internal Bluetooth SIG AES-128 ah() test vector and exit
    #[arg(long)]
    verify_crypto: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    print_banner();

    if args.verify_crypto {
        return run_crypto_verification();
    }

    if args.test_lock {
        println!("{}", "Testing Windows user32::LockWorkStation()...".yellow());
        match lock_workstation() {
            Ok(_) => println!("{}", "Workstation locked successfully!".green().bold()),
            Err(e) => eprintln!("{} {}", "Lock failed:".red().bold(), e),
        }
        return Ok(());
    }

    if args.list_keys {
        return run_list_keys();
    }

    if args.telemetry {
        return run_telemetry(&args);
    }

    if args.calibrate {
        return run_calibration(&args);
    }

    // Default to scan mode if no specific action provided
    run_scan(&args)
}

fn print_banner() {
    println!("{}", "==========================================================".cyan());
    println!("{}", "       WristKey - Apple Watch Proximity Unlock/Lock       ".cyan().bold());
    println!("{}", "              Windows 11 / 10 BLE Engine (Phase 2)        ".cyan());
    println!("{}", "==========================================================".cyan());
}

fn run_list_keys() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "Querying Windows Registry for paired Bluetooth keys...".bold());
    let elevated = is_elevated();
    println!(
        "Process integrity: {}",
        if elevated {
            "Elevated (Administrator)".green().bold()
        } else {
            "Standard User (Non-Elevated)".yellow()
        }
    );

    match read_paired_bluetooth_keys() {
        Ok(keys) => {
            if keys.is_empty() {
                println!("{}", "No paired Bluetooth keys found in BTHPORT registry.".yellow());
            } else {
                println!("\nFound {} paired device key(s):", keys.len());
                for (idx, key) in keys.iter().enumerate() {
                    println!("\n[{}] Device MAC: {}", idx + 1, key.device_mac.cyan().bold());
                    println!("    Host Adapter: {}", key.host_adapter_mac);
                    if let Some(ref irk_hex) = key.irk_hex {
                        println!("    IRK:          {}", irk_hex.green().bold());
                    } else {
                        println!("    IRK:          (not a BLE privacy device or no IRK stored)");
                    }
                    if key.ltk.is_some() {
                        println!("    LTK:          [present]");
                    }
                    if key.link_key.is_some() {
                        println!("    Link Key:     [present]");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("\n{} {}", "Registry query notice:".yellow().bold(), e);
            if !elevated {
                println!(
                    "\nTip: To dump paired IRKs from Windows Registry directly, run this command from an elevated (Administrator) terminal: \n  {}",
                    "wristkey-cli --list-keys".cyan().bold()
                );
                println!("Alternatively, you can provide your IRK manually via the --irk <HEX> flag or in config/wristkey.default.toml.");
            }
        }
    }

    Ok(())
}

fn run_crypto_verification() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "\nVerifying Bluetooth SIG ah(k, r) cryptographic test vector...".bold());
    let irk: [u8; 16] = [
        0xec, 0x02, 0x34, 0xa3, 0x57, 0xc8, 0xad, 0x05,
        0x34, 0x10, 0x10, 0xa6, 0x0a, 0x39, 0x7d, 0x9b,
    ];
    let prand: [u8; 3] = [0x70, 0x81, 0x94];
    let expected_hash: [u8; 3] = [0x0d, 0xfb, 0xaa];

    let resolver = IrkResolver::new(&irk);
    let computed = resolver.compute_ah(&prand);

    println!("IRK:           ec0234a357c8ad05341010a60a397d9b");
    println!("Prand:         {:02x}{:02x}{:02x}", prand[0], prand[1], prand[2]);
    println!("Expected Hash: {:02x}{:02x}{:02x}", expected_hash[0], expected_hash[1], expected_hash[2]);
    println!("Computed Hash: {:02x}{:02x}{:02x}", computed[0], computed[1], computed[2]);

    if computed == expected_hash {
        println!("{}", "[PASS] Cryptographic resolution matches Bluetooth SIG standard 100%.".green().bold());
    } else {
        println!("{}", "[FAIL] Hash mismatch!".red().bold());
    }

    Ok(())
}

fn run_telemetry(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "--- Live Apple Continuity Telemetry Inspector ---".magenta().bold());
    println!("Decoding Nearby Info (0x10) flags in real time (On-Wrist, Locked, Wi-Fi, Auto-Unlock)...");

    let resolver = if let Some(ref irk_hex) = args.irk {
        println!("Filtering exclusively for IRK: {}", irk_hex.yellow());
        Some(IrkResolver::from_hex(irk_hex)?)
    } else {
        println!("{}", "Inspecting all Apple BLE advertisement packets...".yellow());
        None
    };

    let mut scanner = BleScanner::new();
    let rx = scanner.start()?;

    loop {
        while let Ok(event) = rx.try_recv() {
            if let Some(ref r) = resolver {
                if !r.matches_rpa(event.mac) {
                    continue;
                }
            }

            if let Some(ref frame) = event.apple_frame {
                let mac_str = format_mac(event.mac);
                println!("\n[Packet from {} | RSSI: {} dBm]", mac_str.cyan(), event.rssi);
                println!("  Protocol:     {}", frame.message_type_name.bold());
                if let Some(code) = frame.action_code {
                    println!("  Action:       0x{:02X} ({})", code, frame.action_name.as_deref().unwrap_or(""));
                }
                println!(
                    "  On Wrist:     {}",
                    if frame.is_on_wrist { "YES".green().bold() } else { "NO".red() }
                );
                println!(
                    "  Passcode:     {}",
                    if frame.is_locked { "LOCKED".red().bold() } else { "UNLOCKED".green().bold() }
                );
                println!(
                    "  Eligible:     {}",
                    if frame.is_on_wrist_and_unlocked {
                        "AUTHORIZED (On-Wrist & Unlocked)".green().bold()
                    } else {
                        "UNAUTHORIZED".red()
                    }
                );
                println!("  Auto-Unlock:  {}", if frame.auto_unlock_capable { "Yes" } else { "No" });
                println!("  Wi-Fi Active: {}", if frame.wifi_on { "Yes" } else { "No" });
                println!("  iCloud Prim:  {}", if frame.is_primary_icloud { "Yes" } else { "No" });
                if let Some(ref tag) = frame.auth_tag {
                    println!("  Auth Tag:     {:02X?}", tag);
                }
            }
        }
        sleep(Duration::from_millis(150));
    }
}

fn run_scan(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "Initializing BLE Scanner (Active Mode)...".bold());
    if let Some(ref irk) = args.irk {
        println!("Filtering exclusively for IRK: {}", irk.yellow());
    } else {
        println!("{}", "Listening for all Apple BLE devices (Nearby Info & Continuity)...".yellow());
    }

    if args.auto_lock {
        println!("{}", "AUTO-LOCK ENABLED: System will lock when device moves out of range or is removed from wrist!".red().bold());
    }

    let config = WristKeyConfig {
        target_irk_hex: args.irk.clone(),
        lock_threshold_dbm: args.lock_threshold,
        unlock_threshold_dbm: args.unlock_threshold,
        auto_lock_enabled: args.auto_lock,
        require_on_wrist: false,
    };

    let mut engine = WristKeyEngine::new(config)?;
    let mut scanner = BleScanner::new();
    let rx = scanner.start()?;

    println!("\n{:<19} {:<8} {:<18} {:<24}", "MAC Address", "RSSI", "Apple Type", "On-Wrist / Status");
    println!("{}", "-".repeat(74));

    loop {
        while let Ok(event) = rx.try_recv() {
            let matched = engine.handle_event(&event);

            if args.irk.is_some() && !matched {
                continue;
            }

            let mac_str = format_mac(event.mac);
            let rssi_str = format!("{} dBm", event.rssi);

            let (type_name, status_str) = if let Some(ref frame) = event.apple_frame {
                let status = if frame.is_on_wrist_and_unlocked {
                    "YES (On-Wrist & Unlocked)".green().bold().to_string()
                } else if frame.is_locked {
                    "NO (Watch is Locked)".red().to_string()
                } else if let Some(ref act) = frame.action_name {
                    format!("No ({})", act).yellow().to_string()
                } else {
                    "-".to_string()
                };
                (frame.message_type_name.clone(), status)
            } else {
                ("Unknown BLE".to_string(), "-".to_string())
            };

            println!(
                "{:<19} {:<8} {:<18} {:<24}",
                mac_str.cyan(),
                rssi_str,
                type_name,
                status_str
            );
        }

        engine.tick();
        sleep(Duration::from_millis(200));
    }
}

fn run_calibration(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "--- Real-Time Proximity Calibration ---".bold().magenta());
    println!("Sit at your desk or walk away to observe the signal meter.");
    println!("Lock Threshold: {} dBm | Unlock Threshold: {} dBm\n", args.lock_threshold, args.unlock_threshold);

    let mut scanner = BleScanner::new();
    let rx = scanner.start()?;

    let mut presence_mgr = if let Some(ref irk_hex) = args.irk {
        let resolver = IrkResolver::from_hex(irk_hex)?;
        Some(WatchPresenceManager::new(
            resolver,
            args.lock_threshold,
            args.unlock_threshold,
        ))
    } else {
        None
    };

    let mut fallback_tracker = RssiTracker::new(args.lock_threshold, args.unlock_threshold);

    loop {
        while let Ok(event) = rx.try_recv() {
            if let Some(ref mut mgr) = presence_mgr {
                if mgr.feed_advertisement(event.mac, event.rssi, event.apple_frame.clone()) {
                    let smoothed = mgr.rssi_tracker.smoothed_rssi.unwrap_or(event.rssi as f64);
                    let dist_str = mgr
                        .rssi_tracker
                        .estimated_distance_meters()
                        .map(|d| format!("~{:.1}m", d))
                        .unwrap_or_else(|| "?m".to_string());

                    let bar = render_meter(smoothed, -100.0, -40.0, 20);

                    let status_str = match mgr.security_status() {
                        WatchSecurityStatus::AuthorizedInProximity => "NEAR (Ready to Unlock)".green().bold(),
                        WatchSecurityStatus::AuthorizedFar => "FAR (Lock Zone)".red().bold(),
                        WatchSecurityStatus::DetectedUnauthorized { reason } => {
                            format!("UNAUTHORIZED ({})", reason).red()
                        }
                        WatchSecurityStatus::NotDetected => "SEARCHING".yellow(),
                    };

                    print!(
                        "\rRSSI: {:>5.1} dBm {} | Dist: {:>5} | {}",
                        smoothed, bar, dist_str, status_str
                    );
                    let _ = stdout().flush();
                }
            } else {
                // Discovery calibration without specific IRK
                if event.apple_frame.is_some() {
                    fallback_tracker.update(event.rssi);
                    let smoothed = fallback_tracker.smoothed_rssi.unwrap_or(event.rssi as f64);
                    let dist_str = fallback_tracker
                        .estimated_distance_meters()
                        .map(|d| format!("~{:.1}m", d))
                        .unwrap_or_else(|| "?m".to_string());

                    let bar = render_meter(smoothed, -100.0, -40.0, 20);
                    let state_str = match fallback_tracker.state {
                        ProximityState::Near => "NEAR (In Proximity)".green().bold(),
                        ProximityState::Far => "FAR (Out of Range)".red().bold(),
                        ProximityState::Lost => "LOST (Timeout)".red(),
                        ProximityState::Unknown => "ACQUIRING".yellow(),
                    };

                    print!(
                        "\rRSSI: {:>5.1} dBm {} | Dist: {:>5} | State: {:<20}",
                        smoothed, bar, dist_str, state_str
                    );
                    let _ = stdout().flush();
                }
            }
        }

        if let Some(ref mut mgr) = presence_mgr {
            mgr.tick();
        } else {
            fallback_tracker.tick();
        }

        sleep(Duration::from_millis(100));
    }
}

fn render_meter(val: f64, min: f64, max: f64, width: usize) -> String {
    let clamped = val.clamp(min, max);
    let fraction = (clamped - min) / (max - min);
    let filled = (fraction * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));
    if fraction > 0.6 {
        bar.green().to_string()
    } else if fraction > 0.3 {
        bar.yellow().to_string()
    } else {
        bar.red().to_string()
    }
}
