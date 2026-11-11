mod bridge;
mod protocol;
mod router;
#[cfg(windows)]
mod windows;
use std::{io, net::TcpListener, sync::mpsc};

fn usage() -> ! {
    eprintln!("Usage: keyless host | keyless client <HOST-IP>");
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
    let mut stream = bridge::connect(&address)?;
    println!("Connected to {address}.");
    loop {
        windows::inject(bridge::receive(&mut stream)?);
    }
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

fn main() -> io::Result<()> {
    let a: Vec<_> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("host") => host(),
        Some("client") => match a.get(2) {
            Some(ip) => client(ip),
            None => usage(),
        },
        _ => usage(),
    }
}
