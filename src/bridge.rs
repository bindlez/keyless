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
    let size = fs::metadata(path)?.len();
    let checksum = crc32_file(path)?;
    let mut file = File::open(path)?;
    let mut stream = TcpStream::connect(address)?;
    stream.set_nodelay(true)?;
    stream.write_all(&FILE_MAGIC)?;
    stream.write_all(&(name.len() as u16).to_le_bytes())?;
    stream.write_all(name)?;
    stream.write_all(&size.to_le_bytes())?;
    stream.write_all(&checksum.to_le_bytes())?;
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
    let mut raw_checksum = [0; 4];
    stream.read_exact(&mut raw_checksum)?;
    let expected_checksum = u32::from_le_bytes(raw_checksum);
    let target = directory.join(safe_name);
    let temporary = directory.join(format!("{}.keyless-part", safe_name.to_string_lossy()));
    let mut output = File::create(&temporary)?;
    let mut crc = Crc32::new();
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
        crc.update(&buffer[..received]);
        remaining -= received as u64;
    }
    drop(output);
    if crc.finish() != expected_checksum {
        let _ = fs::remove_file(&temporary);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file checksum did not match",
        ));
    }
    fs::rename(temporary, available_path(target))?;
    Ok(())
}

fn available_path(mut target: std::path::PathBuf) -> std::path::PathBuf {
    if !target.exists() {
        return target;
    }
    let parent = target.parent().unwrap_or_else(|| Path::new(".")).to_owned();
    let stem = target
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let extension = target
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();
    for index in 1..10_000 {
        target = parent.join(format!("{stem} ({index}){extension}"));
        if !target.exists() {
            return target;
        }
    }
    target
}

fn crc32_file(path: &Path) -> io::Result<u32> {
    let mut file = File::open(path)?;
    let mut buffer = [0; 64 * 1024];
    let mut crc = Crc32::new();
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            return Ok(crc.finish());
        }
        crc.update(&buffer[..read]);
    }
}

struct Crc32(u32);
impl Crc32 {
    fn new() -> Self {
        Self(0xffff_ffff)
    }
    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= *byte as u32;
            for _ in 0..8 {
                self.0 = if self.0 & 1 == 1 {
                    (self.0 >> 1) ^ 0xedb8_8320
                } else {
                    self.0 >> 1
                };
            }
        }
    }
    fn finish(self) -> u32 {
        !self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Crc32;
    #[test]
    fn crc32_matches_standard_vector() {
        let mut crc = Crc32::new();
        crc.update(b"123456789");
        assert_eq!(crc.finish(), 0xcbf4_3926);
    }
}
