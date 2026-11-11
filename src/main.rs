mod bridge;
mod protocol;
mod router;
#[cfg(windows)]
mod windows;
use std::{io, net::TcpListener, path::PathBuf, sync::mpsc};

fn usage() -> ! {
    eprintln!("Usage: keyless host | keyless client <HOST-IP> | keyless send <AI-IP> <FILE>");
    std::process::exit(2)
}
#[cfg(windows)]
fn host() -> io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", bridge::DEFAULT_PORT))?;
    println!("Keyless host waiting on {}…", bridge::DEFAULT_PORT);
    let (mut stream, peer) = listener.accept()?;
    bridge::accept(&mut stream)?;
    println!("Connected to {peer}. Right edge hands off; Ctrl+Alt+Space toggles.");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for event in rx {
            if bridge::send(&mut stream, event).is_err() {
                break;
            }
        }
    });
    windows::capture(tx)
}
#[cfg(windows)]
fn client(ip: &str) -> io::Result<()> {
    let address = if ip.contains(':') {
        ip.to_string()
    } else {
        format!("{ip}:{}", bridge::DEFAULT_PORT)
    };
    let download_dir = std::env::current_dir()?.join("Keyless Files");
    std::thread::spawn(move || {
        let _ = bridge::receive_files(&download_dir);
    });
    let mut stream = bridge::connect(&address)?;
    println!("Connected to {address}.");
    loop {
        windows::inject(bridge::receive(&mut stream)?);
    }
}
#[cfg(windows)]
fn send_file(ip: &str, file: &str) -> io::Result<()> {
    let address = if ip.contains(':') {
        ip.to_owned()
    } else {
        format!("{ip}:{}", bridge::FILE_PORT)
    };
    bridge::send_file(&address, &PathBuf::from(file))?;
    println!("Sent {file} to {ip}.");
    Ok(())
}
#[cfg(not(windows))]
fn host() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows required",
    ))
}
#[cfg(not(windows))]
fn client(_: &str) -> io::Result<()> {
    host()
}
#[cfg(not(windows))]
fn send_file(_: &str, _: &str) -> io::Result<()> {
    host()
}

fn main() -> io::Result<()> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() == 1 || matches!(a.get(1).map(String::as_str), Some("setup" | "setup-check")) {
        let folder = std::env::temp_dir().join(format!("keyless-setup-{}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        let script = folder.join("setup.ps1");
        // Windows PowerShell 5.1 treats UTF-8 without a BOM as an ANSI file.
        // The setup includes Unicode labels, so preserve its encoding explicitly.
        let script_text = format!("\u{feff}{}", include_str!("../setup.ps1"));
        std::fs::write(&script, script_text)?;
        let mut command = std::process::Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-STA", "-File"])
            .arg(&script)
            .arg("-SourceExe")
            .arg(std::env::current_exe()?);
        if a.get(1).map(String::as_str) == Some("setup-check") {
            command.arg("-SmokeTest");
        }
        let result = command.output();
        let _ = std::fs::remove_file(&script);
        let _ = std::fs::remove_dir(&folder);
        match result {
            Ok(output) if output.status.success() => {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            }
            result => {
                let detail = match result {
                    Ok(output) => format!(
                        "{}\n{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ),
                    Err(error) => error.to_string(),
                };
                let log = std::env::temp_dir().join("keyless-startup-error.txt");
                let _ = std::fs::write(&log, &detail);
                let message = format!(
                    "Keyless setup could not start.\n\n{detail}\n\nDetails saved to {}",
                    log.display()
                );
                eprintln!("{message}");
                #[cfg(windows)]
                windows::show_error(&message);
                return Err(io::Error::other("setup failed; see startup error log"));
            }
        }
        return Ok(());
    }
    match a.get(1).map(String::as_str) {
        Some("host") => host(),
        Some("client") => match a.get(2) {
            Some(ip) => client(ip),
            None => usage(),
        },
        Some("send") => match (a.get(2), a.get(3)) {
            (Some(ip), Some(file)) => send_file(ip, file),
            _ => usage(),
        },
        _ => usage(),
    }
}
