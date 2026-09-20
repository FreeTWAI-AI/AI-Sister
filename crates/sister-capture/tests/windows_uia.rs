//! Native UIA against an owned WPF provider in another process. Run alone: this
//! fixture deliberately owns the foreground. It does not inspect a user's apps.
#![cfg(windows)]

use sister_capture::{
    CapturePermit, FocusSource, PrivacyObservation, windows::focus::WindowsFocus,
};
use sister_core::model::{AssistiveBlock, PrivacyContext, SensitiveFieldState};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Fixture {
    child: Child,
    dir: PathBuf,
}
impl Fixture {
    fn start() -> Self {
        let dir = std::env::temp_dir().join(format!("sister-native-uia-{}", std::process::id()));
        std::fs::create_dir(&dir).expect("fresh fixture directory");
        let child = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-STA",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/uia-visible-text.ps1"
            ))
            .arg("-StateDir")
            .arg(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start owned WPF provider");
        Self { child, dir }
    }
    fn show(&mut self, mode: &str) {
        std::fs::write(self.dir.join("request"), mode).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "WPF fixture exited before {mode}"
            );
            if let Ok(error) = std::fs::read_to_string(self.dir.join("error")) {
                panic!("WPF fixture: {error}");
            }
            if std::fs::read_to_string(self.dir.join("ready"))
                .ok()
                .as_deref()
                == Some(mode)
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "WPF did not focus its {mode} control"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    fn observe(&self, focus: &mut WindowsFocus, expected: SensitiveFieldState) -> CapturePermit {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let PrivacyObservation::Known {
                context:
                    PrivacyContext::Known {
                        focus: snapshot,
                        sensitive_field,
                        ..
                    },
                permit,
            } = focus.context(0).unwrap()
            {
                assert_eq!(
                    snapshot.pid,
                    Some(i64::from(self.child.id())),
                    "only the owned provider may be read"
                );
                if sensitive_field == expected {
                    return permit;
                }
            }
            assert!(
                Instant::now() < deadline,
                "native UIA did not observe {expected:?}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
fn text(blocks: &[AssistiveBlock], role: &str) -> String {
    assert!(
        blocks
            .iter()
            .all(|block| block.role == role && block.bbox.is_none())
    );
    blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
#[ignore = "owns the Windows foreground; CI runs this target in a separate process"]
fn native_uia_reads_visible_edits_and_documents_and_rejects_excluded_text() {
    let mut fixture = Fixture::start();
    fixture.show("edit");
    let mut focus = WindowsFocus::new();
    let permit = fixture.observe(&mut focus, SensitiveFieldState::Clear);
    let initial = text(&focus.assistive_text(permit), "edit");
    assert!(
        initial.contains("電話 0800-123-456"),
        "native UIA did not read the visible Chinese phone line"
    );
    assert!(initial.contains("visible@example.test"));
    assert!(
        !initial.contains("OFFSCREEN-SENTINEL"),
        "scrolled-out text leaked into the captured frame"
    );

    // Same HWND, title and control; values must be read again instead of cached.
    fixture.show("changed");
    let changed = text(&focus.assistive_text(permit), "edit");
    assert!(changed.contains("CHANGED-SENTINEL 02-2233-4455"));
    assert!(!changed.contains("0800-123-456"));

    fixture.show("document");
    let document_permit = fixture.observe(&mut focus, SensitiveFieldState::Clear);
    let document = text(&focus.assistive_text(document_permit), "document");
    assert!(document.contains("文件 0800-222-333"));
    assert!(document.contains("DOCUMENT-SECOND-PARAGRAPH"));
    assert!(!document.contains("DOCUMENT-BOTTOM"));

    fixture.show("document-scrolled");
    let scrolled = text(&focus.assistive_text(document_permit), "document");
    assert!(scrolled.contains("DOCUMENT-BOTTOM 02-9988-7766"));
    assert!(!scrolled.contains("0800-222-333"));
    assert!(!scrolled.contains("DOCUMENT-SECOND-PARAGRAPH"));

    fixture.show("password");
    assert!(!focus.is_current(document_permit).unwrap());
    assert!(focus.assistive_text(document_permit).is_empty());
    assert!(!focus.is_current(permit).unwrap());
    assert!(focus.assistive_text(permit).is_empty());
    let password_permit = fixture.observe(&mut focus, SensitiveFieldState::Focused);
    assert!(focus.assistive_text(password_permit).is_empty());

    fixture.show("button");
    let button_permit = fixture.observe(&mut focus, SensitiveFieldState::Clear);
    assert!(
        focus.assistive_text(button_permit).is_empty(),
        "button names are not visible edit text"
    );

    fixture.show("other");
    assert!(!focus.is_current(button_permit).unwrap());
    assert!(focus.assistive_text(button_permit).is_empty());
    let other_permit = fixture.observe(&mut focus, SensitiveFieldState::Clear);
    assert!(text(&focus.assistive_text(other_permit), "edit").contains("OTHER-WINDOW-SENTINEL"));
    println!(
        "SISTER-UIA: VERIFIED visible-chinese fresh-text document-paragraphs document-scroll no-offscreen no-password no-button stale-window-denied"
    );
}
