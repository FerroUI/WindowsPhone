//! Platform-independent axis arbitration. A chosen vertical axis never turns
//! into horizontal navigation, even if the finger later bends back across it.
pub(crate) const TAP_SLOP: f64 = 8.0;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    Pending,
    Horizontal,
    Vertical,
    Invalid,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Motion {
    pub origin: (f64, f64),
    pub position: (f64, f64),
    pub axis: Axis,
    pub distance: f64,
    pub started: u64,
    pub timestamp: u64,
}
impl Motion {
    pub fn new(x: f64, y: f64, timestamp: u64) -> Self {
        Self {
            origin: (x, y),
            position: (x, y),
            axis: if x.is_finite() && y.is_finite() {
                Axis::Pending
            } else {
                Axis::Invalid
            },
            distance: 0.0,
            started: timestamp,
            timestamp,
        }
    }
    pub fn update(&mut self, x: f64, y: f64, timestamp: u64) {
        self.timestamp = timestamp;
        if !x.is_finite() || !y.is_finite() {
            self.axis = Axis::Invalid;
            self.distance = f64::INFINITY;
            return;
        }
        self.position = (x, y);
        let (dx, dy) = self.delta();
        self.distance = self.distance.max(dx.hypot(dy));
        if self.axis == Axis::Pending {
            if dx.abs() >= TAP_SLOP && dx.abs() > dy.abs() * 1.4 {
                self.axis = Axis::Horizontal;
            } else if dy.abs() >= TAP_SLOP {
                self.axis = Axis::Vertical;
            }
        }
    }
    pub fn delta(&self) -> (f64, f64) {
        (
            self.position.0 - self.origin.0,
            self.position.1 - self.origin.1,
        )
    }
    pub fn is_tap(&self) -> bool {
        self.axis != Axis::Invalid && self.distance < TAP_SLOP
    }
    pub fn swipe(&self, axis: Axis) -> bool {
        if self.axis != axis {
            return false;
        }
        let (dx, dy) = self.delta();
        let displacement = if axis == Axis::Horizontal { dx } else { dy };
        let elapsed = self.timestamp.saturating_sub(self.started).max(1) as f64 / 1000.0;
        displacement.abs() >= 46.0
            || (displacement.abs() >= 24.0 && displacement.abs() / elapsed >= 380.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_vertical_pan_never_turns_into_navigation() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(3.0, 10.0, 20);
        m.update(100.0, 20.0, 100);
        assert_eq!(m.axis, Axis::Vertical);
        assert!(!m.swipe(Axis::Horizontal));
    }
    #[test]
    fn returning_to_origin_does_not_restore_tap() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(15.0, 1.0, 20);
        m.update(0.0, 0.0, 40);
        assert!(!m.is_tap());
        assert!(!m.swipe(Axis::Horizontal));
    }
    #[test]
    fn tiny_jitter_is_tap_but_fast_short_flick_is_navigation() {
        let mut m = Motion::new(30.0, 30.0, 0);
        m.update(33.0, 32.0, 20);
        assert!(m.is_tap());
        m.update(60.0, 33.0, 60);
        assert!(m.swipe(Axis::Horizontal));
    }
    #[test]
    fn diagonal_motion_prefers_scrolling() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(13.0, 13.0, 50);
        assert_eq!(m.axis, Axis::Vertical);
    }
    #[test]
    fn malformed_pointer_coordinates_never_activate() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(f64::NAN, 0.0, 20);
        assert!(!m.is_tap());
        assert!(!m.swipe(Axis::Horizontal));
    }
}
