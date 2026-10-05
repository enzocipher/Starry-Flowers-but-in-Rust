//! Input gating for read-only skip. Cancellation applies in the same frame.
#[derive(Default)]
pub struct SkipGate {
    blocked_hold: bool,
    next_advance: f64,
}
impl SkipGate {
    pub fn update(
        &mut self,
        latch: &mut bool,
        held: bool,
        toggle: bool,
        cancel: bool,
        eligible: bool,
    ) -> bool {
        if !held {
            self.blocked_hold = false;
        }
        if cancel {
            *latch = false;
            self.blocked_hold |= held;
            return false;
        }
        if toggle {
            *latch = !*latch;
            if !*latch {
                self.blocked_hold |= held;
                return false;
            }
        }
        let requested = *latch || held && !self.blocked_hold;
        if requested && !eligible {
            *latch = false;
            self.blocked_hold |= held;
            return false;
        }
        requested
    }
    pub fn advance_due(&mut self, active: bool, time: f64) -> bool {
        if !active {
            self.next_advance = time;
            return false;
        }
        if time < self.next_advance {
            return false;
        }
        self.next_advance = time + 0.08;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::SkipGate;
    #[test]
    fn cancellation_is_immediate() {
        let mut g = SkipGate::default();
        let mut latch = true;
        assert!(!g.update(&mut latch, true, false, true, true));
        assert!(!latch);
        assert!(!g.update(&mut latch, true, false, false, true));
        assert!(!g.update(&mut latch, false, false, false, true));
        assert!(g.update(&mut latch, true, false, false, true));
    }
    #[test]
    fn unread_line_requires_trigger_release() {
        let mut g = SkipGate::default();
        let mut latch = true;
        assert!(!g.update(&mut latch, true, false, false, false));
        assert!(!latch);
        assert!(!g.update(&mut latch, true, false, false, true));
        g.update(&mut latch, false, false, false, true);
        assert!(g.update(&mut latch, true, false, false, true));
    }
    #[test]
    fn toggle_off_does_not_advance() {
        let mut g = SkipGate::default();
        let mut latch = false;
        assert!(g.update(&mut latch, false, true, false, true));
        assert!(!g.update(&mut latch, false, true, false, true));
    }
    #[test]
    fn skip_has_bounded_pacing_and_restarts_immediately() {
        let mut g = SkipGate::default();
        let advances = (0..60)
            .filter(|frame| g.advance_due(true, *frame as f64 / 60.))
            .count();
        assert!(advances >= 10 && advances <= 13);
        assert!(!g.advance_due(false, 2.));
        assert!(g.advance_due(true, 2.));
    }
}
