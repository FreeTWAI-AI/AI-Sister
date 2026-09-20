//! 真子行程固定住 fork → exec 前的 handle 繼承窗，不靠競速重跑。
#![cfg(unix)]

use sister_core::consent::{self, Consent, RecordingStartConsent, Sheet};
use std::fs::File;
use std::io::Write;
use std::os::fd::FromRawFd;
use std::path::PathBuf;

struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("sister-consent-fork-{}-{name}", std::process::id()));
        std::fs::create_dir(&path).expect("unique fixture directory");
        Self(path)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Child 不碰同意書；只暫留 fork 繼承的 handles，直到 parent 完成斷言。
struct InheritedHandles {
    pid: libc::pid_t,
    release: File,
}

impl InheritedHandles {
    fn hold() -> Self {
        let mut fds = [-1; 2];
        // SAFETY: 指向兩個有效 fd 欄位；成功後交給 File 管理 parent 的 handles。
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        let reader = unsafe { File::from_raw_fd(fds[0]) };
        let release = unsafe { File::from_raw_fd(fds[1]) };
        // SAFETY: fork 後的 child 只呼叫 async-signal-safe 的 close/poll/_exit，
        // 不配置、不使用 Rust 鎖、不跑 inherited guard 的 destructor。
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            unsafe {
                libc::close(fds[1]);
                let mut ready = libc::pollfd {
                    fd: fds[0],
                    events: libc::POLLIN,
                    revents: 0,
                };
                // 30 秒只防 fixture 孤兒；成功條件是 child 還活著時就能取新快照。
                while libc::poll(&mut ready, 1, 30_000) < 0 {}
                libc::_exit(0);
            }
        }
        drop(reader);
        Self { pid, release }
    }

    fn assert_alive(&self) {
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) },
            0,
            "child must still hold the inherited descriptors"
        );
    }
}

impl Drop for InheritedHandles {
    fn drop(&mut self) {
        // 斷言失敗也喚醒並回收 child，不把鎖和背景行程留給下一條測試。
        let _ = self.release.write_all(&[1]);
        let mut status = 0;
        loop {
            let waited = unsafe { libc::waitpid(self.pid, &mut status, 0) };
            if waited >= 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                break;
            }
        }
    }
}

#[test]
fn committed_grant_and_revoke_are_visible_before_an_unrelated_child_exits() {
    let tmp = Tmp::new("commit");
    for grant in [true, false] {
        let mut child = None;
        consent::mutate(&tmp.0, |consent| {
            child = Some(InheritedHandles::hold());
            if grant {
                consent.grant(Sheet::LocalRecording, 42);
            } else {
                consent.revoke(Sheet::LocalRecording);
            }
            assert!(matches!(
                consent::try_begin_recording_start(&tmp.0),
                RecordingStartConsent::Busy
            ));
            Ok(())
        })
        .expect("commit consent");
        let child = child.expect("fork during the write transaction");
        child.assert_alive();
        match consent::try_begin_recording_start(&tmp.0) {
            RecordingStartConsent::Allowed(guard) if grant => {
                assert!(guard.consent().allows_recording());
                assert_eq!(guard.consent().local_recording, Some(42));
            }
            RecordingStartConsent::NotAllowed(snapshot) if !grant => {
                assert!(!snapshot.allows_recording());
            }
            other => panic!("finished grant={grant} must expose its new snapshot, got {other:?}"),
        }
        child.assert_alive();
    }
}

#[test]
fn failed_mutation_releases_the_writer_without_saving_its_grant() {
    let tmp = Tmp::new("failed");
    let mut child = None;
    let result = consent::mutate(&tmp.0, |consent| {
        child = Some(InheritedHandles::hold());
        consent.grant(Sheet::LocalRecording, 42);
        anyhow::bail!("fixture rejects the mutation")
    });
    assert!(result.is_err());
    let child = child.expect("fork during the rejected transaction");
    child.assert_alive();
    match consent::try_begin_recording_start(&tmp.0) {
        RecordingStartConsent::NotAllowed(snapshot) => assert_eq!(snapshot, Consent::default()),
        other => panic!("rejected mutation must remain unsigned, got {other:?}"),
    }
    assert!(!consent::path(&tmp.0).exists());
    child.assert_alive();
}
