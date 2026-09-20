//! 從正式入庫、檢索到 RAG，讀字來源與原畫面編號不能被來源 ref 的種類蓋掉。

use sister_core::db::Db;
use sister_core::grounded_answer::{self, SourceRef};
use sister_core::model::{ClipboardKind, FocusKind, OcrBlock, SourceKind};
use sister_core::replay::{
    Corpus, Event, FORMAT_VERSION, RedactionSummary, ReplayFocus, ReviewStatus,
};
use sister_core::retrieval::RetrievalProfile;

#[test]
fn rag_preserves_recorded_origins_and_their_own_frame_reference() {
    let clipboard = "phone evidence 0912-345-678";
    let corpus = Corpus {
        format_version: FORMAT_VERSION,
        name: "RAG origin fixture".into(),
        duration_ms: 100,
        review: ReviewStatus::Draft,
        redactions: RedactionSummary::default(),
        events: vec![
            Event::Frame {
                assistive: vec![sister_core::model::AssistiveBlock {
                    text: "phone evidence 0800-080-123".into(),
                    role: "edit".into(),
                    bbox: None,
                }],
                at_ms: 10,
                monitor: 0,
                width: 800,
                height: 600,
                dhash: 1,
                dup_run: 0,
                focus: ReplayFocus::default(),
                ocr: vec![OcrBlock {
                    text: "phone evidence 0800-080-123".into(),
                    x: 10,
                    y: 20,
                    w: 300,
                    h: 40,
                    confidence: 0.9,
                }],
            },
            Event::Focus {
                at_ms: 20,
                kind: FocusKind::Focus,
                snapshot: ReplayFocus {
                    window_title: Some("phone evidence 02-2233-4455".into()),
                    url: Some("https://evidence.example/phone".into()),
                    ..Default::default()
                },
            },
            Event::Clipboard {
                at_ms: 30,
                kind: ClipboardKind::Text,
                text: Some(clipboard.into()),
                byte_len: clipboard.len() as i64,
                truncated: false,
                secret_suspected: false,
                source_app: None,
            },
        ],
    };
    let mut db = Db::open_in_memory().expect("db");
    let imported = db
        .import_replay(&corpus, 1_000)
        .expect("normal insert paths");
    assert_eq!(imported.frames, 1);
    let got = RetrievalProfile::TextAndFacts
        .retrieve(&mut db, "phone", 10)
        .expect("retrieval");
    assert_eq!(
        got.hits.len(),
        5,
        "OCR, assistive, clipboard, title and URL"
    );
    assert_eq!(got.answers.len(), 3, "three different phone facts");
    assert!(
        got.answers.iter().all(|answer| answer.sightings == 1),
        "OCR and UIA on one frame must not double the sightings"
    );
    let frame_id = got
        .hits
        .iter()
        .find(|hit| hit.source_kind == SourceKind::Ocr)
        .and_then(|hit| hit.frame_id)
        .expect("OCR frame reference");
    assert!(db.frame_context(frame_id).unwrap().is_some());
    // Replay 保留 OCR 文字與 frame metadata，不帶圖片；來源類別不能當圖檔存在的證據。
    assert!(db.frames_with_image(&[frame_id]).unwrap().is_empty());
    let rag = grounded_answer::prepare("phone", &[], &got.answers, &got.hits, 2_000)
        .unwrap()
        .unwrap();
    assert_eq!(rag.sources.len(), 8);
    let prompt_rows: Vec<serde_json::Value> = rag
        .payload
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|row: &serde_json::Value| row.get("ref").is_some())
        .collect();
    assert_eq!(prompt_rows.len(), rag.sources.len());
    for source in &rag.sources {
        let (origin, original_frame) = match source.reference {
            SourceRef::Fact(id) => {
                let row = &got
                    .answers
                    .iter()
                    .find(|a| a.latest.id == id)
                    .unwrap()
                    .latest;
                (row.source_kind.as_str(), row.frame_id)
            }
            SourceRef::Chunk(id) => {
                let hit = got.hits.iter().find(|hit| hit.chunk_id == id).unwrap();
                (hit.source_kind.as_str(), hit.frame_id)
            }
            SourceRef::Card(_) => panic!("fixture contains no interpretations"),
        };
        let expected_frame = matches!(origin, "ocr" | "assistive").then_some(frame_id);
        assert_eq!(original_frame, expected_frame);
        assert_eq!(
            source.frame_id,
            expected_frame,
            "{}",
            source.reference.as_str()
        );
        let prompt = prompt_rows
            .iter()
            .find(|row| row["ref"] == source.reference.as_str())
            .unwrap();
        assert_eq!(prompt["kind"], origin, "{}", source.reference.as_str());
    }
}
