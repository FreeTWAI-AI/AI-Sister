# HANDOFF — 交給下一位 agent（Codex）

**更新於 2026-09-19（Codex 續接）；最後出貨點仍是 alpha.143 `2de931b`，alpha.144
已備妥、尚未 push／tag。** Claude 的原始交接保留在本機的
`HANDOFF-CODEX-SEAT-2026-09-19.md`，已加入本機 Git exclude，不進公開提交。
這份是「打開就能接著做」的交接紀錄，不是
路線圖。路線圖在 `docs/PHASES.md`，規格在 `docs/SPEC.md`，產品定義在
`docs/PRODUCT.md`，工作紀律在 `AGENTS.md`。四份都要讀，順序就是這個順序。

> **接手更正（2026-09-13）：**這份交接的第 4.2 節與原步驟 3 把 #42 誤寫成尚未
> 實作。實際上它已由 `fbb61e2` 與 `85f6f16` 在 alpha.100 完成並出貨；PHASES 同一段
> 後文也有完整收據。下方已改成不再指示下一位重做。
>
> **Codex 續接收據（2026-09-13）：**`v0.1.0-alpha.141` 已由 `9c6fbca` 切 tag 並公開；
> tag CI 八個 job 全綠，四個 artifact 齊全。Ted 已在真 Windows 用正式 Setup 覆蓋舊版，
> 確認舊角色、四張同意、Grok 選擇與記憶保留，且問答的本機出處可點回原截圖。精確人工
> 驗收範圍記在 `docs/WINDOWS-CHECKLIST.md` 的 alpha.141 smoke，不把未做項目算通過。
>
> **Codex alpha.142 收據（2026-09-15）：**`9162cf0` 把背景解釋改成從前往後一次完成一段，
> 前一張工作假設落地後才處理下一段；新證據會支持、推翻或修正同一套理解。答題檢索也改成
> 多查詢輪流取證、保留原問題時間範圍、補開頭／中段／結尾或命中附近的 L2，再按時間成句。
> `v0.1.0-alpha.142` 已公開；tag run `34932696475` attempt 2 的八個 job 全部 success。
> attempt 1 只在最後的 Windows 簽章 fixture 第二次 NSIS bundle 遇到一次 runner 連線
> `10054`，同一 commit 重跑已通過，沒有產品改碼。Release 有四個 uploaded artifact：
> `AI-Sister-Setup.exe` 276,017,955 bytes、`sister-desktop.exe` 69,828,608 bytes、
> `sister.exe` 10,929,152 bytes、`AI-Sister-Linux-X11-amd64.deb` 63,810,044 bytes。
>
> **Codex alpha.143 收據（2026-09-15）：**`2de931b` 補上一次性的舊記憶重讀：第一次在新條文下
> 取得第二張同意後固定歷史右界，從最早仍保留的事件按六小時窗往後掃，一次完成一段、進度寫回
> SQLite 的 `meta`；現場優先，舊資料每天最多 20 段且不超過每日解釋預算的四分之一。第二張
> `cloud-reading` 條文因此重寫（131 → 191 字，`CLOUD_READING_TERMS_VERSION` 1 → 2，升級只
> 重問第二張），17 支 bundled 朗讀跟著重錄。`v0.1.0-alpha.143` 已公開；tag run `34959625002`
> 第一次就八個 job 全部 success，沒有重跑。Release 有四個 uploaded artifact：
> `AI-Sister-Setup.exe` 277,421,179 bytes、`sister-desktop.exe` 71,220,736 bytes、
> `sister.exe` 10,945,536 bytes、`AI-Sister-Linux-X11-amd64.deb` 65,201,634 bytes。
> Release body 和 `scripts/release-notes.sh v0.1.0-alpha.143` 逐字前綴相同，後面只多一行
> GitHub 自己附加的 Full Changelog。
>
> **下一位動同意書語音之前先讀這三件事：**（1）alpha.137 那份切點檔一度遺失，重跑 polish
> 會安靜地把 51 支「不該變的」換成另一批；切點已找回並存在 `~/voice-lab/`
> `consent-voice-trims-consent-v1.json`，每一筆都以「切完重走流水線、解出來的 PCM 和出貨的
> 那一支逐位元組相同」驗過。重跑一律帶 `--trims` 加 `--reuse-ogg-from`，它印的
> 「原封搬回來的：N / 68」就是收據。（2）同意書那包的品管引擎是 Whisper `large-v3`，不是
> `medium`；GPU 被佔住時用 `~/voice-lab/_a143_qc_fp16.py` 包一層（whisper 的 fp16 只轉 mel
> 不轉權重，LayerNorm 要留在 fp32）。（3）新的 17 支朗讀是 32.8–46.0 秒，**超過
> Breeze-ASR-25 的 30 秒硬上限**，所以 NOTICE 那句「兩個訓練資料不同的 ASR」目前只對 30 秒
> 以內的那 51 支成立；要再拉長條文之前先確認第二個引擎吃不吃得下。

---

## 0. 這是什麼專案

**AI-Sister 是一個在 Windows 上安靜看著螢幕、事後答得出「我昨天在幹嘛」、而且每一
句話都點得開證據的本機記錄器。**

- **repo**：`https://github.com/teddashh/AI-Sister`（public，Apache-2.0；產出的角色
  圖與語音以 NOTICE 排除在該授權之外）
- **本機工作目錄**：`/home/ted-h/projects/AI-Sister`，branch `main`，直接推 `main`，
  不開 PR。
- **「本機」的精確意思**：錄下的截圖、OCR 與記憶留在這台機器。**腦（L2/L3）接的是
  使用者自己已經裝好的 CLI agent，不是內建 HTTP client。** desktop 只有兩條具名、
  窄化的內建 outbound：Persona 素材包的使用者發起 GET，以及預設關閉、另行同意後才
  送答案正文的 Azure TTS POST。這條界線不可以擴散到 recorder／core／capture／
  brain／hands 或 WebView。

程式碼分佈：

| 位置 | 是什麼 |
|---|---|
| `crates/sister-core` | 記憶、斷句、同意書、審閱、回答組裝（邏輯的家） |
| `crates/sister-capture` | 擷取與 OCR 熱路徑 |
| `crates/sister-cli` | `sister` 指令（record／watch／ask／forget／export／doctor…） |
| `crates/sister-hands` | Phase 6 的手（sidecar，預設關） |
| `crates/sister-shell` | 桌面外殼共用邏輯（例：點擊穿透判斷 `hit.rs`） |
| `crates/sister-tts` | 本機朗讀與 Azure 可選 TTS |
| `crates/sister-assets` | 角色素材的存取層 |
| `apps/desktop` | Tauri 桌面（**另一個 workspace**，見第 6 節的坑） |
| `scripts/` | 44 支 `check-*` 閘門與 promote 腳本 |
| `site/`＋`scripts/build-website.py` | 公開網站 |

---

## 1. 我是誰、做到哪裡

| | |
|---|---|
| 這一段的執行者 | 原主段是 Claude Code（Opus 5, 1M context），session `e74b7a6f-34e8-4500-925c-8e0d020ac13c`；alpha.142／143 由 Codex 續接；2026-09-15～16 的 15 個 commit 由 Claude Code session `5c0ee346` 做 |
| 時間範圍 | 2026-09-09T05:04:30Z → 2026-09-16（UTC；中途壓縮數十次） |
| **已公開的最後一版** | **`v0.1.0-alpha.143` → `2de931b`**，published 2026-09-15 |
| **2026-09-19 接手點** | **`e5645c7`，當時 ahead `origin/main` 16 個 commit，未 push、未 tag**；下述 Codex 續接另有測試與交接修正 |
| 下一版版號 | 已 bump 到 `0.1.0-alpha.144`（17 個位置一致，`check-release-version.py` 綠），`docs/RELEASE-NOTES.md` 的 `## v0.1.0-alpha.144` 那一節已寫好 |
| alpha.143 CI | tag run `34959625002` 第一次就八個 job 全部 `success`；Release 四個 artifact 齊全 |
| alpha.142 CI | tag run `34932696475` attempt 2 八個 job 全部 `success` |
| 本機閘門 | Claude 收據：`e5645c7` 已追蹤內容的 gates-all.sh **52 通過／0 失敗**；Codex 續接驗證範圍見下，不冒充重新跑完 52 條 |
| alpha.141 真機 | 正式 Setup 覆蓋成功；舊 persona／同意／Grok／記憶保留，本機出處可點回原截圖 |
| alpha.141 後續 | 真機 receipt／交接更新；CI 的 GitHub Actions 已升到 Node 24 majors，branch run `34793019345` 六個平台 job 全綠。沒有產品程式碼或新出貨內容 |

**Codex 續接（2026-09-19）：**

- `cargo test --workspace`：1,982 通過、0 失敗、3 忽略。Persona、主對話與時間軸三支
  renderer 檢查通過。主對話的等待秒數測試原以 5 秒自動交回答案、約 4.36 秒取樣，
  本次取樣時答案已返回，三個斷言讀到 `null`；改成完成等待畫面的斷言後才 resolve
  答案，原斷言保留、整支重驗通過。產品程式未改。
- 原始交接把三條 `codex/overview-*` 分支列為未落地，**這是誤判**。三條的
  `git cherry main` 全部為 `-`；等價提交是 `2512920`、`0a9c82d`、`7b43b92`，
  並由 `c85e648` 完成、隨 alpha.116 出貨。不需要 rebase 或重寫；分支保留。
- alpha.144 新增的六項 Windows 人工驗收仍未勾；本機 renderer 不代替真 WebView2。

---

## 2. Spec → 現在：八個 Phase 的完成度

`docs/PHASES.md` 用 checkbox 記退場條件，**退場條件就是驗收條件**。機械數過一次
（`- [x]` / `- [ ]`）：

| Phase | 名稱 | 完成 / 未完 | 現況一句話 |
|---|---|---|---|
| 0 | 感官與地基 | 3 / 2 | 剩「連續 7 天零 crash」與「磁碟 < 300MB/天」兩個要真機時間才拿得到 |
| 1 | S1 回憶核心 | 2 / 3 | 產品已在跑；剩效能數字、全離線走查、README 首段的實測足跡 |
| 2 | 重播評測 harness | 7 / 3 | harness 在；剩題庫 ≥100 題、baseline 進 README、當 regression gate |
| 3 | 斷句 + 事實層 | 2 / 1 | 剩斷句邊界 F1 ≥ 0.75（要手標語料） |
| 4 | 理解與記憶（大腦） | 2 / 3 | L2/L3 已接 CLI；剩 A/B +10pt、成本實測、兩週自用 |
| 5 | **Release 1.0** | 2 / 10 | **現在的主戰場**，見下一節 |
| 6 | 手 v1（hands sidecar） | 0 / 3 | 有實作與 injection 套件，三個退場條件都沒收 |
| 7 | 接手模式 | 0 / 2 | 沒開始 |
| 8 | 生態與 Preview 成熟化 | — | 持續，不擋 1.0 |
| | **合計** | **18 / 27** | |

### Release 1.0 合約〔2026-09-06 由 Ted 定案，不要重新辯論〕

- **Windows 10+ = 正式支援（GA），是唯一的平台支援 blocker。**
- macOS = Public Preview、Linux = X11-only Developer Preview。**沒達到最小合約就
  不發那個 artifact，但缺席不擋 Windows GA。**
- **Persona 角色體驗是 Windows GA 的產品面 blocker**（17 人：四姊妹＋13 位閨密），
  但「使用者選擇關掉角色或聲音」是必須支援的正常路徑。
- 「可升級」= 使用者手動下載新 installer、關掉 desktop／recorder 後原地安裝，並拿
  真的舊版 binary → 新版 binary 跑過。**自動 updater 不在 1.0 合約內。**
- 明確**不擋** 1.0 的：Wayland、macOS 長期足跡數字、≥100 題真題庫、斷句 F1、
  A/B +10pt、兩週開口有用率、b／e 類主動開口、完整 hands、Phase 7。
  數字繼續照實公開，但不再拿來擋發版。

平台現況：Windows Setup／portable CLI／desktop 已公開；Linux X11 Developer Preview
`.deb` 已公開；macOS 只有 CI 上的 app-tree 診斷，**沒有公開 `.app`／`.dmg`**，缺
Developer ID、notarization 與真機 TCC 收據。

---

## 3. 這一輪（alpha.140 之後的 23 顆）做了什麼

主軸是 **ASR／QC／compliance／privacy**。Ted 這一輪的原話：
「**該做的做一做，不要用 compliance gate 卡，一次把他都做過去，讓 compliance 不再
是問題。**」以下由新到舊，全部已 push：

| commit | 做了什麼 |
|---|---|
| `9886ff5` | **他按同意的字和她唸出來的聲音可以是兩件事。** 四份文字互相釘死，但鏈到 manifest 的 `text` 欄位就斷了——同步改五個檔、音檔不動，六條同意書閘門全綠。時長量不回來（最危險的改法是等長的），改問 git 歷史：聲音最後一次真的換，必須不早於文案最後一次真的變。新增 `scripts/check-spoken-consent-is-not-older-than-the-words.py`，linux job 因此加 `fetch-depth: 0` |
| `b193dce` | NOTICE 那句「沒做過壓縮或限幅」掛名的閘門其實在讀流水線自己寫的字。量過 crest 對限幅／壓縮的靈敏度（限幅完全無感、壓縮和現況整段重疊），**老實從 GATE 降級成 LAB**；順帶把 `contains("no compression")` 的針擴成整個承諾片語 |
| `c9d8ea9` | manifest 的 `truePeakDbtp` **從來沒有人拿它對過聲音**（改一支 −9.99，九道閘門全綠），而它的最大值正是 NOTICE 印給使用者的數字。補上逐支比對，門檻 0.5 dB 是先量 952 支的儀器雜訊（0.000）才挑的 |
| `b95acc2` | 公開網站上那份被複製過去的 NOTICE，落地之後沒人問過它還成不成立 |
| `812716f` | **六份出貨 NOTICE 的每一句話都要有人負責**：MEASURED／GATE／LAB／PROSE 四格沒有第五格，多一句沒認領要紅、刪一句讓分類變死也紅，`--list` 拿得出整份清單給法務 |
| `7153e87` | NOTICE 寫「整平到 −23 LUFS／−1 dBTP 天花板」，出貨實測是 −28.0…−22.8、最高真峰 −0.96。改成從 manifest 算出來，不手抄 |
| `566e3eb` | 錄音那邊的私有素材（refs／WAV／QC 收據）在出貨資料夾**外面**一個人都沒守。`.gitignore` 加音檔規則（預防），repo 全域 `git ls-files` 白名單掃描（證明） |
| `a55e6e6`／`3262c53` | 兩支語音數值閘門各自漏印另一半餘裕，而註解替它寫了一句沒證明的話 |
| `4dfa7f6`／`b5beebb`／`5313e12`／`3f9a11b`／`9f58660` 等 | ASR 那一軸的量化收尾：互動短句為什麼比較差、「煩耶。」那一支 60 骰 0 骰過線（量出上限，不是沒解法） |
| `123c6bc`／`c58cf0d`／`4fe049e`／`46c8884`／`e22f7d8`／`acb9991` | NOTICE 內容誰改都沒人知道、出貨資料夾只准放該放的、文件 bytes 反向刀 |

**結果**：compliance 這一軸現在是「六份 NOTICE 共 69 句，當場量 22、別的閘門 9、
錄音那邊 15、非事實宣稱 23，沒有一句沒人認領」。Ted 交代的「讓 compliance 不再是
問題」已經做到，**不要再從頭掃一次**。

---

## 4. 進行中／未完成，以及關鍵檔案

### 4.1 原交接的兩件立刻工作已收掉

- `9886ff5` 的 CI 已是 `completed/success`；`fetch-depth: 0` 與新閘門第一次在 runner
  上執行都通過。
- `v0.1.0-alpha.140` 後的 23 顆 commit 已由 `9c6fbca` 出貨為
  `v0.1.0-alpha.141`；tag CI 八個 job 全綠，Release 的四個 artifact 齊全。
- alpha.141 後的 `7472cea`、`e84ae97`、`77eba17`、`dfbd3f0` 只記真 Windows receipt、
  修正會自我污染的零命中 fixture 與整理交接。`3567d51` 將七個 checkout 升到 v7；
  `67ca70d` 將六個 upload 升到 v7、五個 release download 升到 v8，Pages 三顆升到
  configure v6／upload v5／deploy v5；`c645375` 同步網站 gate。branch run `34793019345`
  六個平台 job 全綠、六個 upload 點全數成功且沒有 Node 20 annotation。Release download
  與 Pages 只在 tag job 執行；alpha.142 的 tag run `34932696475` 已把兩者真的跑過。
- `9162cf0` 回應 Ted 對片段回答與「問了才想」的批評：背景 L2 現在按時間逐段修正同一套
  工作假設；S1 答題則讓多條查詢、原問題時間範圍與鄰近 L2 一起形成有前後文的來源集。
  `v0.1.0-alpha.142` 已公開，沒有尚待出貨的產品程式碼。下一步仍是第 5 節步驟 3 的真機驗收，
  不重切同一版。

### 4.2 交接更正：#42 的 URL 設定已完成

`docs/PHASES.md` 的 Phase 6 那一節先保留「#42 沒關」的歷史問題，後文再記錄
**alpha.100 落地**。交接時只讀到前半段，因而把已完成能力誤列成下一步。

- `crates/sister-hands/src/url_policy.rs` 有兩個答案與 `Option` 三態；`None` 是「還沒
  問過」，型別上和「你說了要當場按」分開。
- `apps/desktop/ui/app.js` 會在主對話主動提出問題；`sister url-policy` 是同一題的 CLI
  入口。
- `Grant::authorize_unattended` 在唯一 standing-grant 授權邊界執行 host provenance
  規則；當場按的路徑維持另一種明確同意。

這一塊已經包含在 alpha.100 之後的公開版本，**不要再做一次**。Phase 6 的 injection
exit criterion 仍未勾，是因為同站 path、redirect 與當場按的邊界，不是缺這個設定。

### 4.3 已知但刻意沒做的

- **點擊穿透的最後一段**（`crates/sister-shell/src/hit.rs`）：`POLL_BLIND_MS` 的修法
  已經想好、刻意沒做——那是連續第三個改同一條線的版本，而「會不會真的踩到」只有真機
  答得出來。**評估過並否決**：在實心塊外圍加 keepout（會把「手臂和身體之間的空隙點
  得過去」這個賣點一起關掉）。
- **`docs/PHASES.md` 裡搜 `allowed_next_step_fact`**：字母人（`apps/desktop`）那半
  沒有那道閘門。看到「還缺」先分清楚「閘門在寫入端還是執行端」再動。
- **Dependabot**：只剩 glib `GHSA-wrw7-89jp-8q8g`，被 tauri 的 gtk 0.18 整套釘死，
  但不可達（沒碰 `glib::Variant`）且只進 Linux 的 `.deb`。**升 Tauri 或 dismiss 都
  是 Ted 的決定，不要自己決定。**
- **Windows 程式碼簽章**：pipeline 與四層隔離 fixture 都通了，公開 alpha 仍是
  verified-unsigned。stable 必須有 public-CA PFX 與密碼 secrets。
  **不可以假造，要等 Ted 的憑證。**

### 4.4 關鍵檔案路徑

| 要找什麼 | 去哪 |
|---|---|
| 路線圖與退場條件 | `docs/PHASES.md` |
| 規格 | `docs/SPEC.md`；產品定義 `docs/PRODUCT.md` |
| 工作紀律／文案規則 | `AGENTS.md` |
| 隱私宣言與資料位置 | `docs/PRIVACY.md`、`docs/DATA_INVENTORY.md`、`docs/THREAT_MODEL.md` |
| 版本歷史 | `docs/RELEASE-NOTES.md`（**歷史區段不要改**，見第 5 節） |
| Windows 驗收 | `docs/WINDOWS-CHECKLIST.md`、`docs/WINDOWS-CODE-SIGNING.md` |
| 同意書文字（權威） | `crates/sister-core/src/consent.rs` 的 `wording()`／`without()` |
| 同意書的另外三份副本 | `apps/desktop/ui/onboarding.js`、`apps/desktop/ui/persona-consent-voices/catalog-v1.json`、同資料夾的 `v1/manifest.json` |
| 出貨語音 | `apps/desktop/ui/persona-voices/v1`（544 支）、`persona-consent-voices/v1`（68 支）、`persona-banter-voices/v1`（340 支），合計 952 支 Ogg Opus |
| 上一位的 living plan | `.handoff/PLAN.md`（**刻意 gitignored**，2892 行，最前面那章是「唯一現行摘要」但停在 2026-09-11＝已落後九個版本；下面的日誌區仍是精確 receipt） |

---

## 5. 下一步最小可驗證步驟（照順序，打開就能做）

**2026-09-19 現行下一步：** Ted 明確授權後 push `main`，等 branch CI 通過，
再對通過的 commit 打 `v0.1.0-alpha.144` 並推送 tag；確認 Release 四個 artifact
公開且說明與 `scripts/release-notes.sh` 產出的前綴相符。版號與說明已備妥，不再 bump。
這次交接明列「push 仍問」，尚未收到解除此限制的授權。下方步驟 1–2 是歷史完成紀錄，
步驟 3 的真機待辦繼續有效，連同 alpha.144 新增項目用最新正式 artifact 驗收。

### 步驟 1（已完成）：確認交接點的 CI

```bash
cd /home/ted-h/projects/AI-Sister
gh run list --limit 3 --json headSha,status,conclusion \
  --jq '.[] | "\(.headSha[0:7]) \(.status)/\(.conclusion // "-")"'
```

**驗收**：`9886ff5` 是 `completed/success`。若紅，最可能的兩個成因是
`fetch-depth: 0` 那個改動，或新閘門 `check-spoken-consent-is-not-older-than-the-words.py`
在 runner 上拿不到歷史——那一條**設計成「問不到就紅」**，不是 bug。

### 步驟 2（已完成）：把 23 顆未出貨的 commit 切成 `v0.1.0-alpha.141`

版號散在 **7 個檔**（`check-release-version.py` 會全部對一次）：
`Cargo.toml`、`Cargo.lock`、`apps/desktop/src-tauri/Cargo.toml`、
`apps/desktop/src-tauri/Cargo.lock`、`apps/desktop/src-tauri/tauri.conf.json`、
`docs/RELEASE-NOTES.md`、`scripts/check-windows-upgrade.ps1`。
兩個 `Cargo.lock` 要跑過 cargo 才會變髒，**很容易漏掉**。

**驗收**：`python3 ./scripts/check-release-version.py` 綠 → 推 commit → CI 全綠 →
才打 tag。**release body 在打 tag 那一刻就定死**，說明要先寫對；驗法是整份比
**前綴**（`generate_release_notes: true` 會在後面附加 Full Changelog，比相等會每次
假紅）。打完 tag 要回頭確認 release job 真的跑了——linux job 一紅，release job 會
被靜靜跳過。

### 步驟 3（進行中）：在 alpha.142 正式 artifact 上驗背景連續理解與完整回答

alpha.142 的 release job 已公開四個 artifact；CI 已驗 build、installer、alpha.110→current
升級與記憶保留，但不冒充 Ted 的真日常畫面。先安裝正式 Setup，照
`docs/WINDOWS-CHECKLIST.md` 新增的兩條 S1 項目實測：

1. 不先問問題，連續錄同一件工作的起因、處理、反證與結果，再看「她知道了什麼」；後段判讀
   必須真的改寫前段工作假設，不能每張只是孤立摘要。
2. 再問「為什麼會這樣，後來怎麼了」；答案要按時間串成一段、來源跨過不同查詢與時間段，
   沒證據時不能把先後冒充因果。

alpha.141 尚未收掉的零命中 smoke 仍有效：先暫停錄製，確認停下後才產生從未顯示過的新 GUID，
保持暫停拿它提問，最後恢復錄製。Ted 尚未回報上述三條；不要預先勾選，也不要拿 CI fixture
或另一輪 source gate 代替正式安裝副本上的結果。#42 已在 alpha.100 完成，不要重做。

---

## 6. 已知坑、不要重做的事、站立約束

### 6.1 Ted 的站立指示（優先於一切）

- **「不要用 compliance gate 卡，一次把他都做過去。」** 這一軸已經做完（第 3 節），
  不要再從頭掃一次。
- **「有執行檔他就下載測，沒有就繼續推。不要停下來等回覆；做完一段就切 tag。
  一次多做一點再叫他測。」**
- **「一條能力要嘛端到端做好再露出，要嘛整條留在產品外。」** 不交半成品、占位選項、
  「之後會補」文字。
- **產品介面不講開發過程**，也不把責任推回使用者。
- **不要加 gate、要交出看得見的體驗。** 連兩版只加 CI gate 會被打槍。
  （這一輪是 Ted 明確點名的 compliance 例外。）
- **兩邊都站得住的時候，先問「這題該不該是使用者的」**，不要拿二選一去問他。

### 6.2 產品邊界（不可以回歸）

沒有 Neutral／字母人 persona；預設 ChatGPT；brain 沒有 HTTP client；Azure 是
opt-in；WebView CSP 只走 IPC；暫停鍵不可以藏進設定裡（「她整個產品的前提是你隨時
停得掉」）。

### 6.3 隱私（fail-closed，不要擅自破壞）

- **錄音那邊的私有素材（`refs.json`、`refs/*.wav`、WAV 母帶、QC 收據）一個都不可以
  進這個 public repo。** 出貨的只有 Ogg ＋ manifest ＋ NOTICE。預防在 `.gitignore`，
  證明在 `scripts/check-shipped-asset-trees-hold-nothing-else.py`。
- `.handoff/` 與 `research/extracts` 刻意留本機。
- **Windows 程式碼簽章要 Ted 的憑證，不可以假造。**

### 6.4 這台機器的坑

- **沒有 sudo。** `apps/desktop/src-tauri` **本機編不起來**（缺 `libdbus-1-dev`）。
  驗證路徑是 `./scripts/check-windows.sh` ＋ CI 的 macOS job。
  **不要再寫「這裡編不出來所以沒人驗」——macOS job 真的會跑桌面那棵樹的測試。**
- **`/tmp` 是 31G RAM disk。** 跑測試前一定 `export TMPDIR=/home/ted-h/tmp-tests`；
  那裡有 26G 是別人的東西，不要清。
- **`export PATH="$HOME/.cargo/bin:$PATH"`** 是必要的。
- 語音實驗室的 Python 在 `/home/ted-h/voice-lab/.venv/bin/python`。

### 6.5 反覆踩到、代價最高的幾個

1. **`apps/desktop` 是另一個 workspace。** 根 `Cargo.toml` 是
   `members = ["crates/*"]`，所以 `cargo test --workspace` 碰不到桌面那棵樹。
   **邏輯要搬進 `crates/`**，在 `main.rs` 裡加測試等於沒加。
2. **改 `apps/desktop/` 一定要跑 `cargo fmt`**（macOS job 會 fmt-check 它）。
3. **`docs/RELEASE-NOTES.md` 舊版本標題底下的段落是歷史，不要為了「數字過期」去改。**
   但**當時就不成立的假話要改**（有三個先例：`3cfc0ba`、`e4cb088`、`7402f2f`）。
4. **突變有粗細兩種，只做粗的會騙過自己。** 刪掉整個分支會紅；把那個分支**算出來的
   值**換成空字串或隔壁那一臂的值卻全綠——而使用者讀到的是後者。
5. **doc 裡每寫一句「這擋不住 X」，驗收就要有一刀 `want=綠` 去證明它。** 正向的「會
   紅」很容易驗，反向的「抓不到」幾乎沒人驗，而它太樂觀或太保守都會讓下一輪做錯決定。
6. **還原突變一律從檔案備份，不要用 `git checkout --`**（git 只知道 HEAD，不知道
   這一輪的起點），還原後 `cmp` 逐位元組確認。
7. **「紅了」不等於「被測試抓到」**——編不過也是紅的，刀量級不夠會紅在別條規則上。
8. **報 before/after 之前先量儀器自己的雜訊。** 對照組通常是免費的。
9. **閘門不能問執行者本人。** 讀 manifest 上流水線自己寫的字串不是證據，是同步檢查；
   要降級成「量不回來」得先拿數字證明量不回來。
10. **壓縮後 summary 裡的數字是二手的，特別盯全稱句。** 這份交接檔裡每個數字都是
    這一輪從 repo 機械跑出來的，但**你接手後要自己再跑一次**。

---

## 7. 常用指令

```bash
cd /home/ted-h/projects/AI-Sister
export TMPDIR=/home/ted-h/tmp-tests          # /tmp 是 RAM disk，必須改
export PATH="$HOME/.cargo/bin:$PATH"

# ── 一次跑完所有閘門（本機工具，不在 repo 裡；要帶 worktree 參數）──
bash /home/ted-h/tmp-tests/gates-all.sh /home/ted-h/projects/AI-Sister
#   交接時：通過 52 條，失敗 0 條

# ── Rust ──
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p sister-capture privacy
#   注意：cargo test | tee | grep | head 會 SIGPIPE 殺掉 cargo，
#   「全綠」是假的。驗法是數 log 裡 "^test result:" 的行數。

# ── Windows 那半（本機唯一能驗的方式）──
./scripts/check-windows.sh

# ── 單條閘門（44 支都在 scripts/check-*）──
python3 ./scripts/check-consent-copy.py
python3 ./scripts/check-persona-voice-loudness.py          # 要 ffmpeg，解 952 支
python3 ./scripts/check-notice-claims-are-accounted-for.py
python3 ./scripts/check-notice-claims-are-accounted-for.py --list   # 整份宣稱清單
python3 ./scripts/check-spoken-consent-is-not-older-than-the-words.py
python3 ./scripts/check-release-version.py

# ── CI ──
gh run list --limit 5 --json headSha,status,conclusion,displayTitle \
  --jq '.[] | "\(.headSha[0:7]) \(.status)/\(.conclusion // "-") \(.displayTitle)"'

# ── 發版（步驟 2）──
#   1. 改 7 個檔的版號（兩個 Cargo.lock 要跑過 cargo 才會變髒）
#   2. python3 ./scripts/check-release-version.py
#   3. 推 commit → 等 CI 六個 job 全綠
#   4. git tag v0.1.0-alpha.N && git push origin v0.1.0-alpha.N
#   5. 回頭確認 release job 真的跑了、四個 artifact 齊、不是 draft
```

CI 有六個 job：`linux`（測試／lint／隱私／全部資產閘門）、`linux_preview`（原生
capture＋`.deb`）、`msrv`（Rust 1.88）、`macos_spike`、`windows`、`release`
（只在 `refs/tags/v*` 上跑，且 `needs: [linux, linux_preview, msrv, windows]`）。

---

## 8. 接手後的第一個動作

1. 讀 `AGENTS.md` 第零節（全域交付與產品文案規則）。
2. 讀 `docs/PHASES.md` 最前面的 Release 1.0 合約。
3. 不重切 alpha.141、不重掃 compliance；沿用第 5 節步驟 3，讓 Ted 在正式安裝副本上
   一次驗 `docs/WINDOWS-CHECKLIST.md` 的一條真機邊界，精確記下通過或失敗的範圍。

**不要**開新大軸、不要重掃 compliance、不要動 `docs/RELEASE-NOTES.md` 的歷史區段、
不要碰簽章憑證、不要把錄音那邊的東西搬進 repo。
