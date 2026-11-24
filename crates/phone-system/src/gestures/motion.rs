//! Platform-independent axis arbitration. A chosen vertical axis never turns
//! into horizontal navigation, even if the finger later bends back across it.
pub(crate) const TAP_SLOP: f64 = 8.0;
const VELOCITY_WINDOW_MS: u64 = 100;
const FLICK_VELOCITY: f64 = 380.0;
const MIN_FLICK_DISTANCE: f64 = 24.0;
const SAMPLE_CAPACITY: usize = 8;
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
    samples: [(f64, f64, u64); SAMPLE_CAPACITY],
    sample_count: usize,
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
            samples: [(x, y, timestamp); SAMPLE_CAPACITY],
            sample_count: 1,
        }
    }
    pub fn update(&mut self, x: f64, y: f64, timestamp: u64) {
        if !x.is_finite() || !y.is_finite() || timestamp < self.timestamp {
            self.axis = Axis::Invalid;
            self.distance = f64::INFINITY;
            return;
        }
        self.timestamp = timestamp;
        // A release may arrive with the last move's timestamp. Replace that
        // sample instead of manufacturing infinite velocity from zero time.
        if self.samples[self.sample_count - 1].2 == timestamp {
            self.samples[self.sample_count - 1] = (x, y, timestamp);
        } else {
            if self.sample_count == SAMPLE_CAPACITY {
                self.samples.copy_within(1.., 0);
                self.sample_count -= 1;
            }
            self.samples[self.sample_count] = (x, y, timestamp);
            self.sample_count += 1;
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
        displacement.abs() >= 46.0 || self.is_flick(axis, displacement)
    }

    /// Paging has a viewport-relative distance threshold. A deliberate short
    /// flick still advances, while a slow partial pan or a late reversal settles
    /// back. The projection is an interaction policy, not an original-device
    /// measurement; it is shared across phone sizes instead of fixed pixels.
    pub fn page_swipe(&self, width: f64) -> bool {
        if self.axis != Axis::Horizontal || !width.is_finite() || width <= 0.0 {
            return false;
        }
        let displacement = self.delta().0;
        let threshold = (width * 0.25).clamp(48.0, 120.0);
        let velocity = self.velocity(Axis::Horizontal);
        if displacement.abs() >= threshold {
            if velocity.abs() >= FLICK_VELOCITY && velocity.signum() != displacement.signum() {
                let projected = displacement + velocity * 0.12;
                return projected.signum() == displacement.signum() && projected.abs() >= threshold;
            }
            return true;
        }
        self.is_flick(Axis::Horizontal, displacement)
    }

    fn is_flick(&self, axis: Axis, displacement: f64) -> bool {
        let velocity = self.velocity(axis);
        displacement.abs() >= MIN_FLICK_DISTANCE
            && velocity.abs() >= FLICK_VELOCITY
            && velocity.signum() == displacement.signum()
    }

    /// Velocity belongs to the end of the gesture, not the time since contact.
    /// A held finger can flick; a flick followed by a pause must not coast.
    fn velocity(&self, axis: Axis) -> f64 {
        if !matches!(axis, Axis::Horizontal | Axis::Vertical) {
            return 0.0;
        }
        let latest = self.samples[self.sample_count - 1];
        let Some(first) = self.samples[..self.sample_count].iter().find(|sample| {
            latest.2.saturating_sub(sample.2) <= VELOCITY_WINDOW_MS && sample.2 < latest.2
        }) else {
            return 0.0;
        };
        let distance = if axis == Axis::Horizontal {
            latest.0 - first.0
        } else {
            latest.1 - first.1
        };
        distance * 1000.0 / (latest.2 - first.2) as f64
    }

    /// Both fingers must finish meaningful horizontal strokes in the same
    /// direction; a stationary second finger never turns a dismissal into
    /// the destructive clear-all gesture.
    pub fn parallel_swipe(&self, other: &Self) -> bool {
        self.swipe(Axis::Horizontal)
            && other.swipe(Axis::Horizontal)
            && self.delta().0.signum() == other.delta().0.signum()
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
    #[test]
    fn flick_uses_recent_motion_after_a_held_contact() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(0.0, 0.0, 1_000);
        m.update(-30.0, 1.0, 1_040);
        assert!(m.swipe(Axis::Horizontal));
        assert!(m.page_swipe(400.0));
    }
    #[test]
    fn pausing_after_a_short_flick_does_not_advance_the_page() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(-30.0, 0.0, 40);
        assert!(m.page_swipe(400.0));
        m.update(-30.0, 0.0, 400);
        assert!(!m.swipe(Axis::Horizontal));
        assert!(!m.page_swipe(400.0));
    }
    #[test]
    fn slow_page_pans_use_the_available_phone_width() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(-85.0, 0.0, 800);
        assert!(m.page_swipe(320.0));
        assert!(!m.page_swipe(400.0));
        m.update(-110.0, 0.0, 1_600);
        assert!(m.page_swipe(400.0));
        assert!(!m.page_swipe(f64::NAN));
        assert!(!m.page_swipe(0.0));
    }
    #[test]
    fn reversing_a_pan_before_release_cancels_page_navigation() {
        let mut m = Motion::new(0.0, 0.0, 0);
        m.update(-200.0, 0.0, 500);
        m.update(-110.0, 0.0, 520);
        assert!(!m.page_swipe(400.0));
        // Stopping after the reversal is still a deliberate quarter-page pan.
        m.update(-110.0, 0.0, 800);
        assert!(m.page_swipe(400.0));
    }
    #[test]
    fn duplicate_timestamps_do_not_create_a_flick_and_regression_cancels() {
        let mut m = Motion::new(0.0, 0.0, 10);
        m.update(-30.0, 0.0, 10);
        assert!(!m.page_swipe(400.0));
        m.update(-130.0, 0.0, 5);
        assert_eq!(m.axis, Axis::Invalid);
        assert!(!m.page_swipe(400.0));
    }
    #[test]
    fn clear_all_requires_two_parallel_completed_strokes() {
        let mut first = Motion::new(20.0, 80.0, 0);
        let mut second = Motion::new(40.0, 160.0, 5);
        first.update(120.0, 82.0, 200);
        assert!(!first.parallel_swipe(&second));
        second.update(-60.0, 160.0, 200);
        assert!(!first.parallel_swipe(&second));
        second.update(140.0, 160.0, 230);
        assert!(first.parallel_swipe(&second));
        second.update(40.0, 160.0, 260);
        assert!(!first.parallel_swipe(&second));
    }
}
