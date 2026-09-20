//! Only the focused, visible, non-password Edit or Document control. No full-document/tree dump,
//! ValuePattern fallback, focus changes, content cache or event/keystroke listener.
use crate::assistive::{self, ReadWindow, TextEnd, TextRange, TextRole, VisibleText};
use sister_core::model::AssistiveBlock;
use windows::{
    Win32::{
        Foundation::HWND,
        UI::{
            Accessibility::{
                IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
                IUIAutomationTextRange, IUIAutomationTextRangeArray, TextPatternRangeEndpoint,
                TextPatternRangeEndpoint_End, TextPatternRangeEndpoint_Start,
                UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_TextPatternId,
            },
            WindowsAndMessaging::GetForegroundWindow,
        },
    },
    core::Interface,
};

pub(super) fn read(
    automation: &IUIAutomation,
    hwnd: HWND,
    pid: u32,
    window: &ReadWindow,
) -> Vec<AssistiveBlock> {
    let Ok(root) = (unsafe { automation.ElementFromHandle(hwnd) }) else {
        return Vec::new();
    };
    let Ok(element) = (unsafe { automation.GetFocusedElement() }) else {
        return Vec::new();
    };
    let mut source = FocusedText {
        automation,
        hwnd,
        pid,
        root,
        element,
        ranges: None,
        scope: None,
        provider: None,
    };
    assistive::collect(&mut source, window)
}
struct FocusedText<'a> {
    automation: &'a IUIAutomation,
    hwnd: HWND,
    pid: u32,
    root: IUIAutomationElement,
    element: IUIAutomationElement,
    ranges: Option<IUIAutomationTextRangeArray>,
    scope: Option<IUIAutomationTextRange>,
    provider: Option<IUIAutomationElement>,
}
impl FocusedText<'_> {
    fn matches(&self) -> Option<bool> {
        unsafe {
            if GetForegroundWindow() != self.hwnd
                || super::focus::process_id(self.hwnd) != Some(self.pid)
            {
                return Some(false);
            }
            let focused = self.automation.GetFocusedElement().ok()?;
            if !self
                .automation
                .CompareElements(&focused, &self.element)
                .ok()?
                .as_bool()
            {
                return Some(false);
            }
            super::uia::belongs_to_root(self.automation, &self.element, &self.root)?;
            if let Some(provider) = &self.provider {
                super::uia::belongs_to_root(self.automation, &self.element, provider)?;
                super::uia::belongs_to_root(self.automation, provider, &self.root)?;
                if provider.CurrentControlType().ok()? != UIA_DocumentControlTypeId
                    || provider.CurrentIsPassword().ok()?.as_bool()
                    || provider.CurrentIsOffscreen().ok()?.as_bool()
                {
                    return Some(false);
                }
            }
            let (_, monitor) = super::screen::focused_monitor(self.hwnd)?;
            let bounds = self.element.CurrentBoundingRectangle().ok()?;
            // Another monitor's text cannot use this frame as its evidence.
            Some(
                bounds.right > bounds.left
                    && bounds.bottom > bounds.top
                    && bounds.left >= monitor.left
                    && bounds.top >= monitor.top
                    && bounds.right <= monitor.right
                    && bounds.bottom <= monitor.bottom,
            )
        }
    }
}
fn text_pattern(element: &IUIAutomationElement) -> Option<IUIAutomationTextPattern> {
    unsafe {
        element
            .GetCurrentPattern(UIA_TextPatternId)
            .ok()?
            .cast()
            .ok()
    }
}
impl FocusedText<'_> {
    fn visible_pattern(&mut self) -> Option<IUIAutomationTextPattern> {
        if let Some(pattern) = text_pattern(&self.element) {
            return Some(pattern);
        }
        // Only a positively identified Document can borrow an enclosing provider.
        // An Edit without TextPattern never falls back to the rest of its page.
        if self.role()? != TextRole::Document {
            return None;
        }
        unsafe {
            let walker = self.automation.ControlViewWalker().ok()?;
            let mut parent = self.element.clone();
            for _ in 0..8 {
                parent = walker.GetParentElement(&parent).ok()?;
                if self
                    .automation
                    .CompareElements(&parent, &self.root)
                    .ok()?
                    .as_bool()
                {
                    return None;
                }
                if parent.CurrentControlType().ok()? != UIA_DocumentControlTypeId {
                    continue;
                }
                if parent.CurrentIsPassword().ok()?.as_bool()
                    || parent.CurrentIsOffscreen().ok()?.as_bool()
                {
                    return None;
                }
                if let Some(pattern) = text_pattern(&parent) {
                    self.scope = Some(pattern.RangeFromChild(&self.element).ok()?);
                    self.provider = Some(parent);
                    return Some(pattern);
                }
            }
        }
        None
    }
}
fn native_end(end: TextEnd) -> TextPatternRangeEndpoint {
    match end {
        TextEnd::Start => TextPatternRangeEndpoint_Start,
        TextEnd::End => TextPatternRangeEndpoint_End,
    }
}
impl TextRange for IUIAutomationTextRange {
    fn compare(&self, end: TextEnd, other: &Self, other_end: TextEnd) -> Option<i32> {
        unsafe {
            self.CompareEndpoints(native_end(end), other, native_end(other_end))
                .ok()
        }
    }
    fn move_end(&self, end: TextEnd, other: &Self, other_end: TextEnd) -> Option<()> {
        unsafe {
            self.MoveEndpointByRange(native_end(end), other, native_end(other_end))
                .ok()
        }
    }
}
impl VisibleText for FocusedText<'_> {
    fn context_matches(&mut self) -> bool {
        self.matches() == Some(true)
    }
    fn role(&mut self) -> Option<TextRole> {
        match unsafe { self.element.CurrentControlType() }.ok()? {
            kind if kind == UIA_EditControlTypeId => Some(TextRole::Edit),
            kind if kind == UIA_DocumentControlTypeId => Some(TextRole::Document),
            _ => None,
        }
    }
    fn is_password(&mut self) -> Option<bool> {
        Some(unsafe { self.element.CurrentIsPassword() }.ok()?.as_bool())
    }
    fn is_offscreen(&mut self) -> Option<bool> {
        Some(unsafe { self.element.CurrentIsOffscreen() }.ok()?.as_bool())
    }
    fn range_count(&mut self) -> Option<usize> {
        unsafe {
            let pattern = self.visible_pattern()?;
            // https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomationtextpattern-getvisibleranges
            let ranges = pattern.GetVisibleRanges().ok()?;
            let count = usize::try_from(ranges.Length().ok()?).ok()?;
            self.ranges = Some(ranges);
            Some(count)
        }
    }
    fn text(&mut self, index: usize, limit: usize) -> Option<String> {
        unsafe {
            let range = self
                .ranges
                .as_ref()?
                .GetElement(i32::try_from(index).ok()?)
                .ok()?;
            if let Some(scope) = &self.scope
                && !assistive::clip_visible(&range, scope)?
            {
                return Some(String::new());
            }
            Some(range.GetText(i32::try_from(limit).ok()?).ok()?.to_string())
        }
    }
}
