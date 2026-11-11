//! Fixed-size, allocation-free events for the latency-sensitive input path.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEvent {
    PointerMove { dx: i16, dy: i16 },
    Button { button: u8, down: bool },
    Scroll { horizontal: i16, vertical: i16 },
    Key { scan_code: u16, down: bool },
}

impl InputEvent {
    /// Five-byte payload: event kind followed by little-endian fields.
    pub fn encode(self) -> [u8; 5] {
        match self {
            Self::PointerMove { dx, dy } => packet(1, dx, dy),
            Self::Button { button, down } => [2, button, u8::from(down), 0, 0],
            Self::Scroll {
                horizontal,
                vertical,
            } => packet(3, horizontal, vertical),
            Self::Key { scan_code, down } => [
                4,
                (scan_code & 0xff) as u8,
                (scan_code >> 8) as u8,
                u8::from(down),
                0,
            ],
        }
    }

    pub fn decode(bytes: [u8; 5]) -> Option<Self> {
        let first = i16::from_le_bytes([bytes[1], bytes[2]]);
        let second = i16::from_le_bytes([bytes[3], bytes[4]]);
        match bytes[0] {
            1 => Some(Self::PointerMove {
                dx: first,
                dy: second,
            }),
            2 => Some(Self::Button {
                button: bytes[1],
                down: bytes[2] != 0,
            }),
            3 => Some(Self::Scroll {
                horizontal: first,
                vertical: second,
            }),
            4 => Some(Self::Key {
                scan_code: u16::from_le_bytes([bytes[1], bytes[2]]),
                down: bytes[3] != 0,
            }),
            _ => None,
        }
    }
}

fn packet(kind: u8, first: i16, second: i16) -> [u8; 5] {
    let [a, b] = first.to_le_bytes();
    let [c, d] = second.to_le_bytes();
    [kind, a, b, c, d]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_packets_are_stable() {
        assert_eq!(
            InputEvent::PointerMove { dx: -3, dy: 42 }.encode(),
            [1, 253, 255, 42, 0]
        );
    }

    #[test]
    fn events_round_trip() {
        let event = InputEvent::Key {
            scan_code: 0x1d,
            down: true,
        };
        assert_eq!(InputEvent::decode(event.encode()), Some(event));
    }
}
