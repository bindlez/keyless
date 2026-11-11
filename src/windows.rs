//! Dependency-free Windows input adapter. The host polls global physical state;
//! the client injects events with the standard Win32 SendInput API.
use crate::protocol::InputEvent;
use std::{mem::size_of, sync::mpsc::Sender, thread, time::Duration};

#[derive(Clone, Copy)]
#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
#[derive(Clone, Copy)]
#[repr(C)]
struct Mouse {
    dx: i32,
    dy: i32,
    data: u32,
    flags: u32,
    time: u32,
    extra: usize,
}
#[derive(Clone, Copy)]
#[repr(C)]
struct Key {
    vk: u16,
    scan: u16,
    flags: u32,
    time: u32,
    extra: usize,
}
#[repr(C)]
union Payload {
    mouse: Mouse,
    key: Key,
}
#[repr(C)]
struct Input {
    kind: u32,
    payload: Payload,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn GetCursorPos(point: *mut Point) -> i32;
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn GetAsyncKeyState(key: i32) -> i16;
    fn SendInput(count: u32, input: *const Input, size: i32) -> u32;
}
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;
const INPUT_MOUSE: u32 = 0;
const INPUT_KEYBOARD: u32 = 1;
const MOVE: u32 = 1;
const LD: u32 = 2;
const LU: u32 = 4;
const RD: u32 = 8;
const RU: u32 = 16;
const MD: u32 = 32;
const MU: u32 = 64;
const WHEEL: u32 = 0x800;
const HWHEEL: u32 = 0x1000;
const KEYUP: u32 = 2;

pub fn show_error(message: &str) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(window: isize, text: *const u16, title: *const u16, flags: u32) -> i32;
    }
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Keyless startup error"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    unsafe {
        MessageBoxW(0, text.as_ptr(), title.as_ptr(), 0x10);
    }
}

pub fn capture(tx: Sender<InputEvent>) -> ! {
    let mut active = false;
    let mut prior = Point { x: 0, y: 0 };
    let mut keys = [false; 256];
    let mut buttons = [false; 3];
    loop {
        let ctrl = unsafe { GetAsyncKeyState(0x11) < 0 };
        let alt = unsafe { GetAsyncKeyState(0x12) < 0 };
        let space = unsafe { GetAsyncKeyState(0x20) < 0 };
        if ctrl && alt && space {
            active = !active;
            if active {
                recenter(&mut prior);
            } else {
                for vk in 8..=254 {
                    if keys[vk] {
                        let _ = tx.send(InputEvent::Key {
                            scan_code: vk as u16,
                            down: false,
                        });
                        keys[vk] = false;
                    }
                }
                for (index, down) in buttons.iter_mut().enumerate() {
                    if *down {
                        let _ = tx.send(InputEvent::Button {
                            button: (index + 1) as u8,
                            down: false,
                        });
                        *down = false;
                    }
                }
            }
            while unsafe { GetAsyncKeyState(0x20) < 0 } {
                thread::sleep(Duration::from_millis(10));
            }
        }
        if active {
            let mut now = Point { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut now) };
            let dx = now.x - prior.x;
            let dy = now.y - prior.y;
            if dx != 0 || dy != 0 {
                let _ = tx.send(InputEvent::PointerMove {
                    dx: dx.clamp(-32768, 32767) as i16,
                    dy: dy.clamp(-32768, 32767) as i16,
                });
                recenter(&mut prior);
            }
            for (i, vk) in [(0, 1i32), (1, 2), (2, 4)] {
                let down = unsafe { GetAsyncKeyState(vk) < 0 };
                if down != buttons[i] {
                    buttons[i] = down;
                    let _ = tx.send(InputEvent::Button {
                        button: (i + 1) as u8,
                        down,
                    });
                }
            }
            for vk in 8..=254 {
                let down = unsafe { GetAsyncKeyState(vk) < 0 };
                if down != keys[vk as usize] {
                    keys[vk as usize] = down;
                    {
                        let _ = tx.send(InputEvent::Key {
                            scan_code: vk as u16,
                            down,
                        });
                    }
                }
            }
        } else {
            let mut now = Point { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut now) };
            if now.x >= unsafe { GetSystemMetrics(SM_CXSCREEN) - 1 } {
                active = true;
                recenter(&mut prior);
            }
        }
        thread::sleep(Duration::from_millis(if active { 2 } else { 12 }));
    }
}
fn recenter(point: &mut Point) {
    point.x = unsafe { GetSystemMetrics(SM_CXSCREEN) / 2 };
    point.y = unsafe { GetSystemMetrics(SM_CYSCREEN) / 2 };
    unsafe { SetCursorPos(point.x, point.y) };
}
pub fn inject(event: InputEvent) {
    let input = match event {
        InputEvent::PointerMove { dx, dy } => Input {
            kind: INPUT_MOUSE,
            payload: Payload {
                mouse: Mouse {
                    dx: dx as i32,
                    dy: dy as i32,
                    data: 0,
                    flags: MOVE,
                    time: 0,
                    extra: 0,
                },
            },
        },
        InputEvent::Button { button, down } => {
            let flags = match (button, down) {
                (1, true) => LD,
                (1, false) => LU,
                (2, true) => RD,
                (2, false) => RU,
                (3, true) => MD,
                (3, false) => MU,
                _ => return,
            };
            Input {
                kind: INPUT_MOUSE,
                payload: Payload {
                    mouse: Mouse {
                        dx: 0,
                        dy: 0,
                        data: 0,
                        flags,
                        time: 0,
                        extra: 0,
                    },
                },
            }
        }
        InputEvent::Scroll {
            horizontal,
            vertical,
        } => {
            let (flags, n) = if horizontal != 0 {
                (HWHEEL, horizontal)
            } else {
                (WHEEL, vertical)
            };
            Input {
                kind: INPUT_MOUSE,
                payload: Payload {
                    mouse: Mouse {
                        dx: 0,
                        dy: 0,
                        data: n as u16 as u32,
                        flags,
                        time: 0,
                        extra: 0,
                    },
                },
            }
        }
        InputEvent::Key { scan_code, down } => Input {
            kind: INPUT_KEYBOARD,
            payload: Payload {
                key: Key {
                    vk: scan_code,
                    scan: 0,
                    flags: if down { 0 } else { KEYUP },
                    time: 0,
                    extra: 0,
                },
            },
        },
    };
    unsafe {
        SendInput(1, &input, size_of::<Input>() as i32);
    }
}
