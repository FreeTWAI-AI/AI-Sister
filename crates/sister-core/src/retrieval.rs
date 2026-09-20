//! 產品與 replay harness 共用的檢索接線。
//!
//! `Db::search` 是文字索引（FTS trigram/unicode61/bigram，必要時 LIKE fallback）；
//! [`crate::answer::answers`] 是 L1 facts。這裡用型別選擇要不要接上後者，讓 CLI、
//! 字母人與評測不必各手抄一次「時間題不能跑 facts」那組分支。

use anyhow::{Result, ensure};

use crate::activity::Activity;
use crate::answer::{Answer, answers_during};
use crate::db::Db;
use crate::model::{Millis, SearchHit};
use crate::question::{self, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalProfile {
    /// 文字檢索產品路徑；不是 raw FTS，必要時仍會走正確性用的 LIKE fallback。
    TextOnly,
    /// 文字檢索之前先列出規則抽出的 L1 facts，和目前產品畫面相同。
    TextAndFacts,
    /// 文字 + facts，再加上活動級章節。章節是一個範圍，不是單一時刻。
    TextFactsAndSession,
}

impl RetrievalProfile {
    pub fn name(self) -> &'static str {
        match self {
            Self::TextOnly => "baseline_text",
            Self::TextAndFacts => "facts",
            Self::TextFactsAndSession => "facts_session",
        }
    }

    pub fn wants_facts(self) -> bool {
        matches!(self, Self::TextAndFacts | Self::TextFactsAndSession)
    }

    pub fn wants_session(self) -> bool {
        matches!(self, Self::TextFactsAndSession)
    }

    pub fn retrieve(self, db: &mut Db, question: &str, limit: usize) -> Result<Retrieval> {
        self.retrieve_at(db, question, RetrievalLimits::same(limit), crate::now_ms())
    }

    pub fn retrieve_with_limits(
        self,
        db: &mut Db,
        question: &str,
        limits: RetrievalLimits,
    ) -> Result<Retrieval> {
        self.retrieve_at(db, question, limits, crate::now_ms())
    }

    /// `now` 是解「昨天」用的鐘。產品路徑傳 [`crate::now_ms`]；評測傳語料終點。
    pub fn retrieve_at(
        self,
        db: &mut Db,
        question: &str,
        limits: RetrievalLimits,
        now: Millis,
    ) -> Result<Retrieval> {
        self.retrieve_for_question_at(db, question, question, limits, now)
    }

    /// CLI 可以改寫檢索詞，但原問題指定的日期優先；整題共用同一個時鐘。
    pub fn retrieve_for_question_at(
        self,
        db: &mut Db,
        query: &str,
        question: &str,
        limits: RetrievalLimits,
        now: Millis,
    ) -> Result<Retrieval> {
        ensure!(
            limits.answers > 0 && limits.text > 0,
            "retrieval limits 必須大於 0"
        );
        ensure!(!question.trim().is_empty(), "retrieval question 不可為空");
        ensure!(!query.trim().is_empty(), "retrieval query 不可為空");

        let shape = question::shape(query);
        let range =
            question::time_range(question, now).or_else(|| question::time_range(query, now));
        let (terms, answer_set, mut hits) = match shape {
            Shape::Recent | Shape::Range => {
                let hits = match range.as_ref() {
                    Some(range) => db.chunks_in_range(range.from, range.to, limits.text + 1)?,
                    None if shape == Shape::Recent => db.recent(limits.text + 1)?,
                    None => Vec::new(),
                };
                (None, Default::default(), hits)
            }
            Shape::Keywords => {
                let terms = question::terms(query).to_string();
                let answer_set = if self.wants_facts() {
                    answers_during(db, query, limits.answers, range.as_ref())?
                } else {
                    Default::default()
                };
                let hits = db.search_during(&terms, limits.text + 1, range.as_ref())?;
                (Some(terms), answer_set, hits)
            }
        };

        let hits_truncated = hits.len() > limits.text;
        hits.truncate(limits.text);

        let mut activities = Vec::new();
        let mut activities_truncated = false;
        if self.wants_session()
            && let Some((_, acts)) = db.chapters_for_question(question, now)?
        {
            activities_truncated = acts.len() > limits.text;
            activities = acts;
            activities.truncate(limits.text);
        }

        Ok(Retrieval {
            profile: self,
            shape,
            time_range: range,
            terms,
            answers: answer_set.items,
            hits,
            activities,
            answers_truncated: answer_set.truncated,
            hits_truncated,
            activities_truncated,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrievalLimits {
    pub answers: usize,
    pub text: usize,
}

impl RetrievalLimits {
    pub const fn new(answers: usize, text: usize) -> Self {
        Self { answers, text }
    }

    pub const fn same(limit: usize) -> Self {
        Self::new(limit, limit)
    }
}

#[derive(Debug, Clone)]
pub struct Retrieval {
    pub profile: RetrievalProfile,
    pub shape: Shape,
    /// 真正交給資料庫的時間窗；空結果的掃描說明也要使用同一個範圍。
    pub time_range: Option<question::TimeRange>,
    /// `None` 代表時間題，沒有拿任何字去比對。
    pub terms: Option<String>,
    pub answers: Vec<Answer>,
    pub hits: Vec<SearchHit>,
    /// 活動級章節。只有 [`RetrievalProfile::TextFactsAndSession`] 會填。
    /// 沒認到時間範圍是空的（沒去算）；認到但切不出來也是空的——兩者靠
    /// [`crate::db::Db::chapters_for_question`] 的 `Option` 在呼叫端分開。
    pub activities: Vec<Activity>,
    pub answers_truncated: bool,
    pub hits_truncated: bool,
    pub activities_truncated: bool,
}

impl Retrieval {
    pub fn truncated(&self) -> bool {
        self.answers_truncated || self.hits_truncated || self.activities_truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FocusSnapshot, FrameCapture, OcrBlock};

    fn db_with_bill() -> Db {
        let mut db = Db::open_in_memory().expect("db");
        let session = db.start_session("test", "test").expect("session");
        db.insert_frame(
            session,
            &FrameCapture {
                assistive: Vec::new(),
                ts: 100,
                monitor: 0,
                width: 800,
                height: 600,
                dhash: 1,
                image: None,
                image_ext: "png",
                ocr: vec![OcrBlock {
                    text: "客服專線 0800-000-123".into(),
                    x: 0,
                    y: 0,
                    w: 300,
                    h: 20,
                    confidence: 1.0,
                }],
                focus: FocusSnapshot::default(),
            },
            None,
            0,
        )
        .expect("frame");
        db
    }

    #[test]
    fn facts_add_the_synonym_answer_without_changing_the_text_baseline() {
        let mut db = db_with_bill();
        let text = RetrievalProfile::TextOnly
            .retrieve(&mut db, "電話是什麼", 5)
            .expect("text");
        let facts = RetrievalProfile::TextAndFacts
            .retrieve(&mut db, "電話是什麼", 5)
            .expect("facts");
        assert!(text.answers.is_empty());
        assert!(text.hits.is_empty(), "螢幕上沒有『電話』兩字");
        assert_eq!(facts.answers.len(), 1);
        assert_eq!(facts.answers[0].latest.raw, "0800-000-123");
    }

    #[test]
    fn recent_questions_are_the_same_in_both_profiles() {
        let mut db = db_with_bill();
        let text = RetrievalProfile::TextOnly
            .retrieve(&mut db, "剛剛發生什麼事", 5)
            .expect("text");
        let facts = RetrievalProfile::TextAndFacts
            .retrieve(&mut db, "剛剛發生什麼事", 5)
            .expect("facts");
        assert_eq!(text.shape, Shape::Recent);
        assert!(text.answers.is_empty() && facts.answers.is_empty());
        assert_eq!(
            text.hits.iter().map(|hit| hit.chunk_id).collect::<Vec<_>>(),
            facts
                .hits
                .iter()
                .map(|hit| hit.chunk_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn calendar_questions_list_the_window_instead_of_searching_time_words() {
        use crate::model::{FocusSnapshot, FrameCapture, OcrBlock};
        use chrono::{Local, TimeZone};

        let now = Local
            .with_ymd_and_hms(2026, 8, 26, 15, 30, 0)
            .single()
            .expect("local")
            .timestamp_millis();
        let yesterday_15 = Local
            .with_ymd_and_hms(2026, 8, 25, 15, 0, 0)
            .single()
            .expect("local")
            .timestamp_millis();

        let mut db = Db::open_in_memory().expect("db");
        let session = db.start_session("test", "test").expect("session");
        db.insert_frame(
            session,
            &FrameCapture {
                assistive: Vec::new(),
                ts: yesterday_15,
                monitor: 0,
                width: 800,
                height: 600,
                dhash: 1,
                image: None,
                image_ext: "png",
                ocr: vec![OcrBlock {
                    text: "SQLite user_version".into(),
                    x: 0,
                    y: 0,
                    w: 300,
                    h: 20,
                    confidence: 1.0,
                }],
                focus: FocusSnapshot {
                    app_id: Some("chrome.exe".into()),
                    window_title: Some("SQLite 文件".into()),
                    ..FocusSnapshot::default()
                },
            },
            None,
            0,
        )
        .expect("frame");

        let got = RetrievalProfile::TextOnly
            .retrieve_at(&mut db, "我昨天下午在弄什麼", RetrievalLimits::same(5), now)
            .expect("range");
        assert_eq!(got.shape, Shape::Range);
        assert!(got.terms.is_none(), "日曆題沒有拿字去比對");
        assert_eq!(got.hits.len(), 1, "那段時間的原文要列得出來");
        assert!(got.hits[0].text.contains("SQLite"));
    }

    #[test]
    fn dated_keywords_select_yesterdays_evidence_before_limiting_results() {
        use chrono::{Local, TimeZone};

        let now = Local
            .with_ymd_and_hms(2026, 9, 20, 15, 0, 0)
            .single()
            .expect("local")
            .timestamp_millis();
        let range = question::time_range("昨天", now).expect("yesterday");
        let mut db = Db::open_in_memory().expect("db");
        let session = db.start_session("test", "test").expect("session");
        let mut expected_source = None;
        for (ts, phone) in [
            (range.from - 1, "0800-000-123"),
            (range.from, "0800-000-123"),
            (range.from + 1_200_000, "0800-000-123"),
            (range.to, "0800-000-123"),
            (now, "0800-000-123"),
            (now + 1, "0800-000-124"),
            (now + 2, "0800-000-125"),
            (now + 3, "0800-000-126"),
        ] {
            let inserted = db
                .insert_frame(
                    session,
                    &FrameCapture {
                        assistive: Vec::new(),
                        ts,
                        monitor: 0,
                        width: 800,
                        height: 600,
                        dhash: 1,
                        image: None,
                        image_ext: "png",
                        ocr: vec![OcrBlock {
                            text: format!("帳單客服專線 ID {phone}"),
                            x: 0,
                            y: 0,
                            w: 300,
                            h: 20,
                            confidence: 1.0,
                        }],
                        focus: FocusSnapshot::default(),
                    },
                    None,
                    0,
                )
                .expect("frame");
            if ts == range.from + 1_200_000 {
                expected_source = Some(inserted.0);
            }
        }

        // 兩字中文、長中文、短英文整詞與數字子字串涵蓋四條取證路徑。
        // 今天的同文紀錄必須在 LIMIT 前排除，不能先取一筆再 retain。
        for query in ["昨天帳單", "昨天客服專線", "昨天 ID", "昨天 80"] {
            let got = RetrievalProfile::TextAndFacts
                .retrieve_at(&mut db, query, RetrievalLimits::same(1), now)
                .expect("retrieval");
            assert_eq!(got.hits.len(), 1, "{query}");
            assert_eq!(got.hits[0].ts, range.from + 1_200_000, "{query}");
            assert_eq!(got.hits[0].frame_id, expected_source, "仍保留同筆畫面出處");
            assert!(got.hits_truncated, "昨天還有第二筆原文");
        }

        let got = RetrievalProfile::TextAndFacts
            .retrieve_at(&mut db, "昨天電話", RetrievalLimits::same(1), now)
            .expect("facts");
        assert_eq!(got.answers.len(), 1);
        assert_eq!(got.answers[0].latest.ts, range.from + 1_200_000);
        assert_eq!(got.answers[0].latest.frame_id, expected_source);
        assert_eq!(got.answers[0].sightings, 2, "只數問句時間內的目擊");
        assert!(!got.answers_truncated, "重複目擊不是多一個答案");

        for query in ["帳單", "今天帳單"] {
            let rewritten = RetrievalProfile::TextAndFacts
                .retrieve_for_question_at(&mut db, query, "昨天帳單", RetrievalLimits::same(1), now)
                .expect("rewritten retrieval");
            assert_eq!(rewritten.hits.len(), 1);
            assert_eq!(
                rewritten.hits[0].ts,
                range.from + 1_200_000,
                "改寫成 {query} 也不能丟掉原問句的昨天"
            );
        }

        let undated = RetrievalProfile::TextAndFacts
            .retrieve_at(&mut db, "電話", RetrievalLimits::same(1), now)
            .expect("undated");
        assert_eq!(
            undated.answers[0].latest.ts,
            now + 3,
            "沒有指定日期仍取最新出處"
        );

        for query in ["前天早上客服專線", "前天早上電話"] {
            let missing = RetrievalProfile::TextAndFacts
                .retrieve_at(&mut db, query, RetrievalLimits::same(1), now)
                .expect("no match");
            assert!(missing.time_range.is_some());
            assert!(
                missing.hits.is_empty() && missing.answers.is_empty(),
                "{query} 沒有命中不能放寬成所有日期"
            );
        }
    }

    #[test]
    fn session_profile_returns_activities_the_text_profile_does_not() {
        use crate::model::{FocusEvent, FocusKind, FocusSnapshot};
        use chrono::{Local, TimeZone};

        let now = Local
            .with_ymd_and_hms(2026, 8, 26, 15, 30, 0)
            .single()
            .expect("local")
            .timestamp_millis();
        let t0 = Local
            .with_ymd_and_hms(2026, 8, 25, 13, 0, 0)
            .single()
            .expect("local")
            .timestamp_millis();

        let mut db = Db::open_in_memory().expect("db");
        let session = db.start_session("test", "test").expect("session");
        let min = 60_000i64;
        for (ts, app, title) in [
            (t0, "code.exe", "db.rs — AI-Sister"),
            (t0 + 45 * min, "chrome.exe", "SQLite user_version 文件"),
            (t0 + 70 * min, "notion.exe", "週報"),
            (t0 + 115 * min, "notion.exe", "週報"),
        ] {
            db.insert_focus(
                session,
                &FocusEvent {
                    ts,
                    kind: FocusKind::Focus,
                    snapshot: FocusSnapshot {
                        app_id: Some(app.into()),
                        window_title: Some(title.into()),
                        ..FocusSnapshot::default()
                    },
                },
            )
            .expect("focus");
        }

        let text = RetrievalProfile::TextOnly
            .retrieve_at(&mut db, "我昨天下午在弄什麼", RetrievalLimits::same(5), now)
            .expect("text");
        assert!(text.activities.is_empty(), "文字配置不進章節");

        let session = RetrievalProfile::TextFactsAndSession
            .retrieve_at(&mut db, "我昨天下午在弄什麼", RetrievalLimits::same(5), now)
            .expect("session");
        assert_eq!(session.activities.len(), 3, "三件事");
        assert_eq!(
            session
                .activities
                .iter()
                .map(|a| a.segment_count)
                .collect::<Vec<_>>(),
            vec![5, 3, 5]
        );
    }
}
