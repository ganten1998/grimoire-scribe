//! A renewable bound on touch diagnostics, using the contact frames' monotonic millisecond clock.
//! No timers, files or input-node operations: the first line after a window expires renews it.

const WINDOW_MS: u32 = 60_000;
const LINES: u16 = 2000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LogDecision {
    Line,
    Cap,
    Skip,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TouchLogBudget {
    start_ms: Option<u32>,
    lines: u16,
}

impl TouchLogBudget {
    pub(super) fn admit(&mut self, k_ms: u32) -> LogDecision {
        if self.start_ms.is_none_or(|start| k_ms.wrapping_sub(start) >= WINDOW_MS) {
            self.start_ms = Some(k_ms);
            self.lines = 0;
        }
        if self.lines < LINES {
            self.lines += 1;
            LogDecision::Line
        } else if self.lines == LINES {
            self.lines += 1;
            LogDecision::Cap
        } else {
            LogDecision::Skip
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exhaust(b: &mut TouchLogBudget, now: u32) {
        for _ in 0..LINES {
            assert_eq!(b.admit(now), LogDecision::Line);
        }
        assert_eq!(b.admit(now), LogDecision::Cap);
        // Even a noisy session cannot overflow the counter or repeat the cap notice.
        for _ in 0..100_000 {
            assert_eq!(b.admit(now), LogDecision::Skip);
        }
    }

    #[test]
    fn cap_once_then_later_touch_evidence_resumes() {
        let mut b = TouchLogBudget::default();
        exhaust(&mut b, 0);
        assert_eq!(b.admit(WINDOW_MS - 1), LogDecision::Skip);
        exhaust(&mut b, WINDOW_MS);
        // A long idle also renews on the first diagnostic, with no periodic work.
        assert_eq!(b.admit(20 * WINDOW_MS), LogDecision::Line);
    }

    #[test]
    fn low_volume_contacts_keep_logging_all_day() {
        let mut b = TouchLogBudget::default();
        // Session summaries every second, well beyond the former lifetime cap.
        for seconds in 0..24 * 60 * 60 {
            assert_eq!(b.admit(seconds * 1000), LogDecision::Line);
        }
    }

    #[test]
    fn the_millisecond_clock_can_wrap() {
        let mut b = TouchLogBudget::default();
        let start = u32::MAX - 1000;
        exhaust(&mut b, start);
        assert_eq!(b.admit(start.wrapping_add(WINDOW_MS - 1)), LogDecision::Skip);
        assert_eq!(b.admit(start.wrapping_add(WINDOW_MS)), LogDecision::Line);
    }
}
