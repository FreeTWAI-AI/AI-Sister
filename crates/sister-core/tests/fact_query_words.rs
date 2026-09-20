//! 查詢詞中的英文片段不能替不相干的 L1 事實拿到 RAG 來源席位。

use sister_core::db::Db;
use sister_core::grounded_answer::{self, SourceRef};
use sister_core::model::{FocusEvent, FocusKind, FocusSnapshot};
use sister_core::retrieval::RetrievalProfile;

#[test]
fn hotel_profile_and_update_keep_the_matching_text_without_unrelated_facts() {
    let mut db = Db::open_in_memory().expect("db");
    let session = db.start_session("test", "test").expect("session");
    for (i, title) in [
        "客服專線 0800-080-123",
        r"檔案 C:\reports\invoice.pdf",
        "期限 2026/09/30",
        "hotel booking confirmation",
        "profile settings",
        "update release summary",
    ]
    .into_iter()
    .enumerate()
    {
        db.insert_focus(
            session,
            &FocusEvent {
                ts: 1_000 + i as i64,
                kind: FocusKind::Focus,
                snapshot: FocusSnapshot {
                    app_id: Some("chrome.exe".into()),
                    window_title: Some(title.into()),
                    ..Default::default()
                },
            },
        )
        .expect("screen title");
    }

    // 先證明這些不相干的事實確實存在，且明確詢問時仍找得到。
    for query in ["phone", "file", "date"] {
        let got = RetrievalProfile::TextAndFacts
            .retrieve(&mut db, query, 10)
            .expect("facts");
        assert!(
            !got.answers.is_empty(),
            "fixture must contain {query} facts"
        );
    }
    for query in ["hotel", "profile", "update"] {
        let expected = db.search(query, 10).expect("text search");
        assert_eq!(expected.len(), 1, "fixture must contain one {query} title");
        let got = RetrievalProfile::TextAndFacts
            .retrieve(&mut db, query, 10)
            .expect("retrieval");
        assert!(
            got.answers.is_empty(),
            "{query} must not request unrelated fact types"
        );
        let rag = grounded_answer::prepare(query, &[], &got.answers, &got.hits, 2_000)
            .expect("prepare")
            .expect("matching evidence");
        assert_eq!(rag.sources.len(), 1, "{query}");
        assert_eq!(
            rag.sources[0].reference,
            SourceRef::Chunk(expected[0].chunk_id)
        );
        assert_eq!(rag.sources[0].text, expected[0].text);
        assert!(!rag.payload.contains("0800-080-123"));
        assert!(!rag.payload.contains("invoice.pdf"));
        assert!(!rag.payload.contains("2026/09/30"));
    }
}
