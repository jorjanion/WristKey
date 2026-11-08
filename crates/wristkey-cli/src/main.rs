use clap::Parser;
use colored::*;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;
use wristkey_core::{
    format_mac, lock_workstation, BleScanner, IrkResolver, ProximityState, RssiTracker,
    WristKeyConfig, WristKeyEngine,
};

#[derive(Parser, Debug)]
#[command(
    name = "wristkey-cli",
    author = "Jorge",
    version = "0.1.0",
    about = "WristKey CLI: Proximity BLE scanner, distance calibrator, and auto-lock tester"
)]
struct Args {
    /// Scan for all nearby Apple BLE devices in active mode
    #[arg(short, long)]
    scan: bool,

    /// Filter by specific Apple Watch Identity Resolving Key (32-char hex string)
    #[arg(long)]
    irk: Option<String>,

    /// Automatically lock PC when device moves out of range or signal is lost
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

    if args.calibrate {
        return run_calibration(&args);
    }

    // Default to scan mode if no specific action provided
    run_scan(&args)
}

fn print_banner() {
    println!("{}", "==========================================================".cyan());
    println!("{}", "       WristKey - Apple Watch Proximity Unlock/Lock       ".cyan().bold());
    println!("{}", "              Windows 11 / 10 BLE Engine                  ".cyan());
    println!("{}", "==========================================================".cyan());
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

fn run_scan(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "Initializing BLE Scanner (Active Mode)...".bold());
    if let Some(ref irk) = args.irk {
        println!("Filtering exclusively for IRK: {}", irk.yellow());
    } else {
        println!("{}", "Listening for all Apple BLE devices (Nearby Info & Continuity)...".yellow());
    }

    if args.auto_lock {
        println!("{}", "AUTO-LOCK ENABLED: System will lock when signal drops below threshold!".red().bold());
    }

    let config = WristKeyConfig {
        target_irk_hex: args.irk.clone(),
        lock_threshold_dbm: args.lock_threshold,
        unlock_threshold_dbm: args.unlock_threshold,
        auto_lock_enabled: args.auto_lock,
        require_on_wrist: false, // in general scan, report status without discarding
    };

    let mut engine = WristKeyEngine::new(config)?;
    let mut scanner = BleScanner::new();
    let rx = scanner.start()?;

    println!("\n{:<19} {:<8} {:<18} {:<24}", "MAC Address", "RSSI", "Apple Type", "On-Wrist / Status");
    println!("{}", "-".repeat(74));

    loop {
        // Drain events
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

    let mut tracker = RssiTracker::new(args.lock_threshold, args.unlock_threshold);
    let mut scanner = BleScanner::new();
    let rx = scanner.start()?;

    let resolver = if let Some(ref irk_hex) = args.irk {
        Some(IrkResolver::from_hex(irk_hex)?)
    } else {
        None
    };

    loop {
        while let Ok(event) = rx.try_recv() {
            let matches = if let Some(ref r) = resolver {
                r.matches_rpa(event.mac)
            } else {
                // Focus on Apple Nearby Info frames in calibration
                event.apple_frame.as_ref().map(|f| f.message_type == 0x10).unwrap_or(false)
            };

            if matches {
                tracker.update(event.rssi);

                let smoothed = tracker.smoothed_rssi.unwrap_or(event.rssi as f64);
                let dist_str = tracker
                    .estimated_distance_meters()
                    .map(|d| format!("~{:.1}m", d))
                    .unwrap_or_else(|| "?m".to_string());

                let bar = render_meter(smoothed, -100.0, -40.0, 20);

                let state_str = match tracker.state {
                    ProximityState::Near => "NEAR (Unlock Range)".green().bold(),
                    ProximityState::Far => "FAR (Lock Range)".red().bold(),
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

        tracker.tick();
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
