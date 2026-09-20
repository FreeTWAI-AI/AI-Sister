//! Bounded visible-text collection shared by native UIA and deterministic privacy tests.
use sister_core::model::AssistiveBlock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

pub(crate) const READ_BUDGET: Duration = Duration::from_millis(900);
const MAX_RANGES: usize = 16;
const MAX_CHARS: usize = 8192;

#[derive(Clone)]
pub(crate) struct ReadWindow {
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}
impl ReadWindow {
    pub(crate) fn new() -> Self {
        Self {
            deadline: Instant::now() + READ_BUDGET,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    fn active(&self) -> bool {
        !self.cancelled.load(Ordering::Acquire) && Instant::now() < self.deadline
    }
}

/// `eligible` must positively establish exact foreground ownership, unchanged focus,
/// an Edit control, IsPassword=false and IsOffscreen=false. Unknown is rejection.
pub(crate) trait VisibleText {
    fn context_matches(&mut self) -> bool;
    fn is_edit(&mut self) -> Option<bool>;
    fn is_password(&mut self) -> Option<bool>;
    fn is_offscreen(&mut self) -> Option<bool>;
    fn range_count(&mut self) -> Option<usize>;
    fn text(&mut self, index: usize, limit: usize) -> Option<String>;
}

fn eligible(source: &mut impl VisibleText) -> bool {
    source.context_matches()
        && source.is_edit() == Some(true)
        && source.is_password() == Some(false)
        && source.is_offscreen() == Some(false)
}

pub(crate) fn collect(source: &mut impl VisibleText, window: &ReadWindow) -> Vec<AssistiveBlock> {
    if !window.active() || !eligible(source) || !window.active() {
        return Vec::new();
    }
    let Some(count) = source.range_count() else {
        return Vec::new();
    };
    let mut remaining = MAX_CHARS;
    let mut out = Vec::new();
    for index in 0..count.min(MAX_RANGES) {
        if remaining == 0 {
            break;
        }
        if !window.active() || !eligible(source) || !window.active() {
            return Vec::new();
        }
        let Some(text) = source.text(index, remaining) else {
            return Vec::new();
        };
        if !window.active() {
            return Vec::new();
        }
        // Enforce our bound even if a provider ignores GetText(maxLength).
        let text: String = text.chars().take(remaining).collect();
        remaining -= text.chars().count();
        let text = text.trim();
        if !text.is_empty() {
            out.push(AssistiveBlock {
                text: text.into(),
                role: "edit".into(),
                bbox: None,
            });
        }
    }
    if !window.active() || !eligible(source) || !window.active() {
        return Vec::new();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Source {
        allowed: bool,
        reads: usize,
        lose_focus: bool,
        cancel: Option<ReadWindow>,
        edit: Option<bool>,
        password: Option<bool>,
        offscreen: Option<bool>,
    }
    impl VisibleText for Source {
        fn context_matches(&mut self) -> bool {
            self.allowed
        }
        fn is_edit(&mut self) -> Option<bool> {
            self.edit
        }
        fn is_password(&mut self) -> Option<bool> {
            self.password
        }
        fn is_offscreen(&mut self) -> Option<bool> {
            self.offscreen
        }
        fn range_count(&mut self) -> Option<usize> {
            Some(30)
        }
        fn text(&mut self, _: usize, _: usize) -> Option<String> {
            self.reads += 1;
            if self.lose_focus {
                self.allowed = false;
            }
            if let Some(window) = &self.cancel {
                window.cancel();
            }
            Some(format!("電話 0800-123-456{}", "甲".repeat(10000)))
        }
    }
    fn source() -> Source {
        Source {
            allowed: true,
            reads: 0,
            lose_focus: false,
            cancel: None,
            edit: Some(true),
            password: Some(false),
            offscreen: Some(false),
        }
    }
    #[test]
    fn password_offscreen_nonedit_and_unknown_never_read() {
        for value in [Some(true), None] {
            let mut s = source();
            s.password = value;
            assert!(collect(&mut s, &ReadWindow::new()).is_empty());
            assert_eq!(s.reads, 0);
            let mut s = source();
            s.offscreen = value;
            assert!(collect(&mut s, &ReadWindow::new()).is_empty());
            assert_eq!(s.reads, 0);
        }
        for value in [Some(false), None] {
            let mut s = source();
            s.edit = value;
            assert!(collect(&mut s, &ReadWindow::new()).is_empty());
            assert_eq!(s.reads, 0);
        }
    }

    #[test]
    fn denied_or_cancelled_never_reads_text() {
        let mut source = source();
        source.allowed = false;
        assert!(collect(&mut source, &ReadWindow::new()).is_empty());
        assert_eq!(source.reads, 0);
        source.allowed = true;
        let window = ReadWindow::new();
        window.cancel();
        assert!(collect(&mut source, &window).is_empty());
        assert_eq!(source.reads, 0);
    }
    #[test]
    fn late_result_or_focus_change_discards_everything() {
        for cancelled in [false, true] {
            let window = ReadWindow::new();
            let mut source = source();
            source.lose_focus = !cancelled;
            source.cancel = cancelled.then(|| window.clone());
            assert!(collect(&mut source, &window).is_empty());
            assert_eq!(source.reads, 1);
        }
    }
    #[test]
    fn provider_text_is_bounded_and_has_no_invented_coordinates() {
        let mut source = source();
        let out = collect(&mut source, &ReadWindow::new());
        assert_eq!(out.len(), 1);
        assert_eq!(source.reads, 1);
        assert!(out[0].text.starts_with("電話 0800-123-456"));
        assert_eq!(out[0].text.chars().count(), MAX_CHARS);
        assert_eq!(out[0].bbox, None);
        assert_eq!(out[0].role, "edit");
    }
}
