use crate::protocol::InputEvent;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

pub const DEFAULT_PORT: u16 = 33_991;
pub const FILE_PORT: u16 = 33_992;
const MAGIC: [u8; 4] = *b"KLS1";
const FILE_MAGIC: [u8; 4] = *b"KLF1";

pub fn connect(address: &str) -> io::Result<TcpStream> {
    let mut stream = TcpStream::connect(address)?;
    stream.set_nodelay(true)?;
    stream.write_all(&MAGIC)?;
    Ok(stream)
}
pub fn accept(stream: &mut TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut magic = [0; 4];
    stream.read_exact(&mut magic)?;
    if magic == MAGIC {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a Keyless peer",
        ))
    }
}
pub fn send(stream: &mut TcpStream, event: InputEvent) -> io::Result<()> {
    stream.write_all(&event.encode())
}
pub fn receive(stream: &mut TcpStream) -> io::Result<InputEvent> {
    let mut bytes = [0; 5];
    stream.read_exact(&mut bytes)?;
    InputEvent::decode(bytes)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unknown input packet"))
}

/// Sends one file over the trusted local network. The filename is stripped to
/// its basename at both ends, so a sender can never choose an arbitrary path.
pub fn send_file(address: &str, path: &Path) -> io::Result<()> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "file needs a valid name"))?;
    let name = filename.as_bytes();
    if name.len() > u16::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "filename too long",
        ));
    }
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut stream = TcpStream::connect(address)?;
    stream.set_nodelay(true)?;
    stream.write_all(&FILE_MAGIC)?;
    stream.write_all(&(name.len() as u16).to_le_bytes())?;
    stream.write_all(name)?;
    stream.write_all(&size.to_le_bytes())?;
    io::copy(&mut file, &mut stream)?;
    Ok(())
}

/// Runs forever on the AI computer and stores received files in `directory`.
pub fn receive_files(directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let listener = TcpListener::bind(("0.0.0.0", FILE_PORT))?;
    for incoming in listener.incoming() {
        let mut stream = match incoming {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let _ = receive_file(&mut stream, directory);
    }
    Ok(())
}

fn receive_file(stream: &mut TcpStream, directory: &Path) -> io::Result<()> {
    let mut magic = [0; 4];
    stream.read_exact(&mut magic)?;
    if magic != FILE_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a Keyless file transfer",
        ));
    }
    let mut length = [0; 2];
    stream.read_exact(&mut length)?;
    let mut raw_name = vec![0; u16::from_le_bytes(length) as usize];
    stream.read_exact(&mut raw_name)?;
    let name = std::str::from_utf8(&raw_name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid filename"))?;
    let safe_name = Path::new(name)
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid filename"))?;
    let mut raw_size = [0; 8];
    stream.read_exact(&mut raw_size)?;
    let mut remaining = u64::from_le_bytes(raw_size);
    let target = directory.join(safe_name);
    let temporary = directory.join(format!("{}.keyless-part", safe_name.to_string_lossy()));
    let mut output = File::create(&temporary)?;
    let mut buffer = [0; 64 * 1024];
    while remaining > 0 {
        let wanted = remaining.min(buffer.len() as u64) as usize;
        let received = stream.read(&mut buffer[..wanted])?;
        if received == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "file transfer interrupted",
            ));
        }
        output.write_all(&buffer[..received])?;
        remaining -= received as u64;
    }
    drop(output);
    fs::rename(temporary, target)?;
    Ok(())
}
