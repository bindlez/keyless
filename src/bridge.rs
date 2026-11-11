use crate::protocol::InputEvent;
use std::io::{self, Read, Write};
use std::net::TcpStream;

pub const DEFAULT_PORT: u16 = 33_991;
const MAGIC: [u8; 4] = *b"KLS1";

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
