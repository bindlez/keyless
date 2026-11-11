//! Geometry and state transitions for a virtual desktop spanning two machines.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Display {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Display {
    pub fn new(id: impl Into<String>, x: i32, y: i32, width: i32, height: i32) -> Self {
        assert!(
            width > 0 && height > 0,
            "a display must have positive dimensions"
        );
        Self {
            id: id.into(),
            x,
            y,
            width,
            height,
        }
    }

    fn right(&self) -> i32 {
        self.x + self.width
    }
    fn bottom(&self) -> i32 {
        self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Handoff {
    pub side: Side,
    /// Position on the receiving edge, normalized from 0.0 to 1.0.
    pub fraction: f32,
}

#[derive(Clone, Debug)]
pub struct DesktopLayout {
    local: Display,
    remote: Display,
}

impl DesktopLayout {
    pub fn new(local: Display, remote: Display) -> Self {
        Self { local, remote }
    }

    /// Returns a side only when the two displays share a non-zero edge span.
    pub fn adjacent_side(&self) -> Option<Side> {
        let x_overlap = overlap(
            self.local.x,
            self.local.right(),
            self.remote.x,
            self.remote.right(),
        );
        let y_overlap = overlap(
            self.local.y,
            self.local.bottom(),
            self.remote.y,
            self.remote.bottom(),
        );
        if self.remote.x == self.local.right() && y_overlap > 0 {
            Some(Side::Right)
        } else if self.remote.right() == self.local.x && y_overlap > 0 {
            Some(Side::Left)
        } else if self.remote.y == self.local.bottom() && x_overlap > 0 {
            Some(Side::Bottom)
        } else if self.remote.bottom() == self.local.y && x_overlap > 0 {
            Some(Side::Top)
        } else {
            None
        }
    }

    /// Reports a handoff only if the pointer attempts to leave through the
    /// shared section of the configured edge.
    pub fn handoff_for(&self, x: i32, y: i32) -> Option<Handoff> {
        let side = self.adjacent_side()?;
        let (coordinate, start, length, on_edge) = match side {
            Side::Right => (
                y,
                self.remote.y.max(self.local.y),
                overlap(
                    self.local.y,
                    self.local.bottom(),
                    self.remote.y,
                    self.remote.bottom(),
                ),
                x >= self.local.right() - 1,
            ),
            Side::Left => (
                y,
                self.remote.y.max(self.local.y),
                overlap(
                    self.local.y,
                    self.local.bottom(),
                    self.remote.y,
                    self.remote.bottom(),
                ),
                x <= self.local.x,
            ),
            Side::Bottom => (
                x,
                self.remote.x.max(self.local.x),
                overlap(
                    self.local.x,
                    self.local.right(),
                    self.remote.x,
                    self.remote.right(),
                ),
                y >= self.local.bottom() - 1,
            ),
            Side::Top => (
                x,
                self.remote.x.max(self.local.x),
                overlap(
                    self.local.x,
                    self.local.right(),
                    self.remote.x,
                    self.remote.right(),
                ),
                y <= self.local.y,
            ),
        };
        if !on_edge || coordinate < start || coordinate >= start + length {
            return None;
        }
        Some(Handoff {
            side,
            fraction: (coordinate - start) as f32 / length as f32,
        })
    }
}

fn overlap(a_start: i32, a_end: i32, b_start: i32, b_end: i32) -> i32 {
    (a_end.min(b_end) - a_start.max(b_start)).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layout() -> DesktopLayout {
        DesktopLayout::new(
            Display::new("primary", 0, 0, 1920, 1080),
            Display::new("ai", 1920, 200, 2560, 1440),
        )
    }
    #[test]
    fn recognizes_only_the_shared_right_edge() {
        assert_eq!(layout().adjacent_side(), Some(Side::Right));
        assert!(layout().handoff_for(1919, 100).is_none());
        let handoff = layout().handoff_for(1919, 600).unwrap();
        assert_eq!(handoff.side, Side::Right);
        assert!((handoff.fraction - (400.0 / 880.0)).abs() < 0.01);
    }
    #[test]
    fn rejects_disconnected_displays() {
        let disconnected = DesktopLayout::new(
            Display::new("a", 0, 0, 100, 100),
            Display::new("b", 101, 0, 100, 100),
        );
        assert_eq!(disconnected.adjacent_side(), None);
    }
}
