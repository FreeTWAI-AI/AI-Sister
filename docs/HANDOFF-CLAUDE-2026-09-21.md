# HANDOFF — 2026-09-21 交回 Claude

寫給下一位。這份只記這個日子兩段 agent 工作實際做了什麼、現在停在哪、接下來不要重做什麼。
路線圖仍是 `docs/PHASES.md`。更早的 Codex 長交接在 `docs/HANDOFF-CODEX.md`。
版本仍是 `0.1.0-alpha.145`。**沒有切 tag，沒有發 release。**

快照時間：2026-09-21。兩條分支的 CI 都已結束。Release／Website 因沒有 tag 而 skipped，這是預期的。

| 分支 | HEAD | CI |
|---|---|---|
| `main` | `5cfd22c5e21c44e379366ca60fe5e006ee57f62b` | [35645786858](https://github.com/teddashh/AI-Sister/actions/runs/35645786858) **success**。Windows、Linux、X11、macOS、兩支 1.88 compile 都綠 |
| `codex/grok-sweep-20260921` | `d249b6198deceeeca992d773db1ec9c94a004ef8` | [35645789163](https://github.com/teddashh/AI-Sister/actions/runs/35645789163) **success**。同一組 job 都綠。產品檔與 main 對過是同一份 |

本機 main 工作目錄：`/home/ted-h/projects/AI-Sister`。
整合 worktree：`/home/ted-h/tmp-tests/sister-word-boundaries`。
掃蕩根目錄：`/home/ted-h/tmp-tests/sister-grok-sweep-20260921T014642Z`。
Cargo 共用 `flock /home/ted-h/tmp-tests/sister-grok-build.lock`，`CARGO_TARGET_DIR=/home/ted-h/projects/AI-Sister/target`。進 lock 之前先跑該目錄的 `/home/ted-h/tmp-tests/sister-grok-sweep-20260921T014642Z/fresh-sources.py`（本機掃蕩腳本，不在 repo 裡）。

---

## 0. 兩段 session 是誰

**Codex session `01a0c443-f9eb-7dd1-b7d6-54219dd83760`**（使用者用短碼 `01a0c443` 點名）。
短碼在 `/home/ted-h/projects/AI-Sister` 上直接 `session_reader show` 會失敗，要用完整 UUID。
那個 VS Code session 本身是續接 stub；真正的前一段是 `01a0c422`，加上 2026-09-21 九個 agent 的 Grok 掃蕩。
Codex 在這裡的角色是 Castle／root：收 agent 的 slice、自己補完被 SIGTERM 的 RAG、cherry-pick 到整合分支、送原生 CI。
起點是 `a22aabe`。使用者當時說 main／tag／release 先別動。

**這個 Grok session `01a0c448-ee05-7e30-b86c-ba8de4ead8fe`。**
使用者第一句是接著 Codex 做。後來明確說「merge, and continue」。
於是 main 被推上去了，但仍然沒有 tag。這段 session 中途被 compact 過一次；compact 之後繼續修 Windows PDF UIA，再把語音和用量接到 main。

---

## 1. Codex／Castle 在 `a22aabe..9eebbff` 收進來的

`9eebbff` 是第一個推上 `origin/main` 的掃蕩整合點。`main` 從 `a22aabe` fast-forward 到這裡，然後又往上長。
這些 commit 現在都在 `origin/main` 上：

| Commit | 做了什麼 |
|---|---|
| `7651aa2` | CI 在原生文件檢查前先裝 OCR 語言包 |
| `2aebee4` | 刪截圖失敗時，已保留的那幀文字也要清掉 |
| `8f8db6d` | 原生 UIA provider 要等到就緒；OCR 語言別名走繁體 |
| `110b7b4` | Edge HTML 內層 Document、PDF 活祖先的 fixture |
| `fd8d67f` | 事實主題在分組與 `LIMIT` 之前就濾掉（`fact_sightings_matching`）。問句填充（「電話是多少」「請幫我找昨天電話」）不能變成必備主題。全形 OCR 數字先折再映回原文。網址列 URL 抽成 L1 `url` 事實 |
| `1db5d02` | 桌面：停掉還沒播完的，晚到的 media／source 失敗要忽略 |
| `849f9ef` | 同一章 L2 只有證據真的動了才重試 |
| `2442872` | standing grant 的 URL origin 要比對記下來的目的地（host + path + query + fragment） |
| `df0fa7d` | 「要繳多少錢」是金額問題，不是主題 |
| `497ddee` | 證明 raw guard 時，目標 URL 的每一份複本都要換掉 |
| `9eebbff` | PDF 捲離第一頁時，fixture 的 bottom-ready 改看第一頁 Group 已經在畫面外 |

RAG 是 Castle 自己收的。原本的 rag worker 被 SIGTERM。重點是：同名主題要在 SQL 裡先套上，再 `GROUP BY`／`LIMIT`，不然較新的無關值會把較舊的命中源擠掉；同一個數字出現在兩個標題下時，要留被問到的那個來源。

當時還有一輪原生 CI 在追：

- Windows Edge PDF fixture 曾在 `show("bottom")` 超時。`9eebbff` 放寬成第一頁 Group 已 offscreen。
- run `35614839471`（`9eebbff`，`workflow_dispatch`）Linux／X11／macOS／Windows／兩支 1.88 全綠。PDF UIA 印過 `SISTER-PDF-UIA: VERIFIED`。
- 同一 SHA 的 push run `35621391570` 又紅：Edge 這次把焦點放在 TextPattern **Document** 上，fixture 只認 Group，`Get-SisterPdfPage` 回 null，`show("top")` 超時。
- 語系包 `zh-TW` 在 push 路徑會跳過（`SISTER_OCR_ZH_ABSENT=1`）。`workflow_dispatch` 才裝語系包。OCR 跳過要老實印 SKIPPED，不要假裝跑過。

桌面對外連線在這段結束時仍是 Persona GET 與 Azure TTS POST 兩條。BreezyVoice 與 LimitReset 是下一節，當時還在整合分支。

---

## 2. 整合分支上先做好、後來才上 main 的語音與用量

掃蕩 worktree：

- `wt-local_voice` 終點 `96541a9`（BreezyVoice）
- `wt-usage` 先是 `ea44e5b`，review 之後是 `4b7b8d3`
- 整合時 cherry-pick 有衝突。`Cargo.toml`、`main.rs`、`settings.css`、`config.rs`、`check-no-network.sh`、`check-settings-say.mjs` 都是**兩邊都留**。

上到整合分支、後來原樣 cherry-pick 到 main 的是這六個（main 上的 SHA 不同，內容對過，見第 4 節）：

| 整合 SHA | main SHA | 內容 |
|---|---|---|
| `1aaa205` | `eb048f0` | 可選 BreezyVoice loopback。`sister-tts` feature `local`。空 feature，**沒有 ureq**。只對 `127.0.0.1:8231` 做 `GET /health`、`POST /tts`。找不到本機服務就靜音，不改走 Azure 或系統 `localService` |
| `58b1702` | `69bcca9` | 新 crate `sister-usage`。本機會話 JSONL。可選 LimitReset GET，feature `public-status`，預設關 |
| `1fe243c` | `6a6c6d7` | usage review：看板要留得住、停止要承認、掃描可以是部分結果 |
| `77cc93b` | `c4434f2` | 文件寫明四條具名 outbound |
| `c0b1bb6` | `4c614cc` | `scripts/check-pet-says-why.mjs` 要掃 `local_tts.rs`。`local-tts-changed`／`local-tts-stop` 是從那裡 `emit` 的，只掃 `main.rs` 和 `recorder_supervisor.rs` 會誤判成沒有人送 |
| `daa88c6` | `f32aeb2` | `settings.js` 的 `paintUsage` 參數不能叫 `view`。`check-combo-is-readable.py` 把每一個 `view` 都當成熱鍵物件，要求後面是 `view.`。用量狀態改叫 `raw` |

### 四條具名 outbound（現在的產品界線）

1. **Persona GET**：`sister-assets` feature `download`。使用者看見 `cdn.ted-h.com` 與資料邊界後按一次。不 redirect、不 retry、不帶 cookie。
2. **Azure TTS POST**：`sister-tts` feature `azure`。預設關。region 只有 `eastasia`／`southeastasia`／`japaneast`。只送當前答案正文。key 在 Windows Credential Manager，target `ted-h/AI-Sister/AzureSpeech/v1`。
3. **BreezyVoice loopback**：`sister-tts` feature `local`。std TCP 到 `127.0.0.1:8231`。沒有 HTTP client crate。
4. **LimitReset GET**：`sister-usage` feature `public-status`。預設關。`https://limitreset.net/api/v1/status` 與 `/api/v1/{product}/latest`。公開看板是全球產品公告，不是使用者帳號的重置證明。本機 JSONL 是本機觀察，不是帳單。

`sister-core`／`sister-capture`／`sister-brain`／`sister-hands` 仍然沒有 HTTP client。WebView CSP 仍然只有 IPC。不要把 CDN、Azure、LimitReset 加進 `connect-src`／`img-src`／`media-src`。一般 CI 不打真 CDN、真 Azure、真 LimitReset。

---

## 3. 這個 Grok session 在 PDF UIA 上實際改了什麼

Edge 在原生 CI 上有時把鍵盤焦點放在整份 PDF 的 TextPattern Document，而不是第一頁的 Group。
Document 的 `GetVisibleRanges` 會把畫面外的那一頁也算進去。`SetFocus` 叫在頁面 Group 上，`GetFocusedElement` 仍停在 Document。Group 的 `IsKeyboardFocusable` 是 true（量到過 713×923 的 Group），MTA 與 STA 都沒有把焦點移過去。對那個 Group 的中心點擊會讓接下來的 `GetVisibleRanges` 不再返回。`FindAll` 整棵 Edge 樹也會掛住。

所以現在的契約是：

- **產品**（`crates/sister-capture/src/windows/text.rs`，`175f70e`）：焦點若是自帶 TextPattern 的 Document，而且底下剛好有一個在螢幕上、頁面大小的 Group，就把可見範圍剪到那個 Group 的 `RangeFromChild`。兩個那麼大的 Group 同時在螢幕上時不剪。走查有上限（128 個節點、深度 6），避免把一次文字讀取變成整棵樹。角色仍是焦點元素自己的：Document 就是 `"document"`，Group 才是 `"document-region"`。
- **測試**（`windows_uia.rs`）：第一頁文字接受 `"document"` 或 `"document-region"`。捲動之後若 UIA 是空的，代表焦點還在已經跑到畫面外的第一頁 Group，存下來的 assistive blocks 也必須是空的。若 UIA 不是空的，代表焦點仍是活著的 Document，文字必須是現在這一頁（`02-6655-4433`），而且不能再含第一頁的 `0800-444-555`。
- **Fixture**（`uia-edge-reader.ps1`）：
  - Edge reader 用 **STA** 啟動（`ae95d08`）。HTML 那條在 STA 上仍通過過。
  - `show("top")` 接受「焦點是 Document，而且範圍裡有 `PDF-FIRST`」。第一頁就緒後再等 **2.5 秒** 才交還，否則截圖 OCR 只看得到 Edge 的標題列（`reader.pdf` 和暫存路徑），看不到頁面。
  - `show("bottom")` **不要**在迴圈裡呼叫 `GetVisibleRanges`。Ctrl+End 之後那支呼叫會不返回。第二頁電話 `02-6655-4433` 在第二頁底部（PDF 座標 `60 120 Td`），所以送的是 `^{END}`，然後等 2.5 秒。滑鼠滾輪到不了那一行。
  - **不要**在啟動第一頁時走 Group 的 `RangeFromChild`（`5cfd22c`，這個 session 最後一個產品 commit）。那次走查會讓 stage 停在 `activating PDF viewport`、metadata 是空的，45 秒後 `show("top")` 超時。頂部是否就緒只看焦點 Document 自己的文字。

已經證明過、不要再走回去的死路：

- 把 Document 焦點直接當成第一頁 Group。產品角色變成 `"document"`，`text(..., "document-region")` 在 `windows_uia.rs` 的 helper 裡失敗。後來 helper 已放寬，但捲動契約不能靠「焦點還在第一頁 Group」這一句，因為焦點根本不在 Group 上。
- 對整扇視窗 `FindAll`。掛住，metadata 寫不出來。
- 每 40ms 對工具列 Group 呼叫 `RangeFromChild`。第二次走查吃掉 45 秒。
- 點 Group 的中心。下一輪文字範圍不再返回。
- 只靠 `SetFocus`。焦點仍是 Document。
- 用滑鼠滾輪代替 Ctrl+End。第二幀 OCR 仍是 Edge chrome，沒有 `02-6655-4433`。
- 啟動時先把頁面 Group 找出來再快取。這次 CI（push，`35645245212`）掛在 `activating PDF viewport`，metadata 空白。整合分支上一次 `workflow_dispatch`（`35639148088`）同一份 fixture 是通過的，所以這是 Edge UIA 的不穩定，不是語音 merge 改壞了截圖。

通過過的收據（同一類 fixture，不是目前這個 HEAD）：

- 整合 `234434c`，run `35639148088`：八個該跑的 job 全綠，含 Windows。log 有 `SISTER-PDF-UIA: VERIFIED native-screenshots native-ocr scroll old-focus-denied same-frame-rag source-url address-denied`。
- main `4da7d5c`，run `35639146255`：全綠。那個 HEAD 還沒有語音／用量，也還沒有 `5cfd22c` 拿掉啟動時的 Group 走查。
- main `5cfd22c`，run `35645786858`：Windows、Linux、X11、macOS、兩支 1.88 compile 全綠。這是拿掉啟動時 Group 走查之後的 HEAD，也是語音／用量已經在上面的 HEAD。Release／Website skipped（沒有 tag）。

`push` 與 `workflow_dispatch` 的差別仍然在：push 設 `SISTER_OCR_ZH_ABSENT=1`，不裝 `zh-TW`。PDF 這條不依賴那個語言包；文件 OCR 的其他測試會老實 SKIP。

---

## 4. `sister diagnose` 在 Windows 上的堆疊

整合分支第一次把語音／用量和 PDF 修補放在一起跑 Windows 時，`crates/sister-cli/tests/diagnose_reads_what_is_already_on_disk.rs` 的 `the_report_reads_the_audit_that_was_already_on_disk` 讓 `sister` 行程印出 `thread 'main' has overflowed its stack`。
Windows 主執行緒預設 1MB。debug 組裝那份已在磁碟上的稽核報告會超過。Linux 主執行緒比較大，所以 Linux job 看不出來。

先試過把報告組在一條 4MB 的 `sister-diagnose` 執行緒上（`8d81377`）。結果更差：四個 diagnose 測試全部在 `thread 'main'` 溢位，包含先前會過的空機器那則。那條執行緒不是溢位的那條。

現在的修法是 `4da7d5c`／`234434c`：`crates/sister-cli/build.rs` 在 Windows 上對 `sister` 這個執行檔加連結參數 `/STACK:8388608`。`ops::diagnose::run` 回到原本的呼叫形狀。報告內容沒有改。

---

## 5. main 上現在有什麼

`origin/main` = `5cfd22c`。自 `9eebbff` 之後可以分成三段：

1. **PDF UIA 與 diagnose 堆疊**，`9e4fd8b` 到 `4da7d5c`。見第 3、4 節。中間有一個已不再使用的執行緒實驗 `8d81377`，下一個 commit 把它撤掉了。
2. **BreezyVoice + usage**，`eb048f0`..`f32aeb2`。從整合分支 cherry-pick，沒有衝突。cherry-pick 之後拿 desktop、tts、usage、`config.rs`、capture、`sister-cli`、三支 check script、`AGENTS.md` 與隱私文件對 `origin/codex/grok-sweep-20260921` 做過 `git diff`，那些路徑是空的。
3. **`5cfd22c`**：啟動第一頁時不再走 Group 走查。這個 commit 也在整合分支上，SHA 是 `d249b61`。

工作樹裡的程式跟 `origin/main` 的 `5cfd22c` 一致。這份交接與 `docs/HANDOFF-CODEX.md` 開頭的指標當時還沒提交；提交它們會再觸發一輪 main CI，程式本身不會變。

---

## 6. 還沒有上 main 的東西

### macOS app-tree probe（不要直接 merge）

- worktree：`/home/ted-h/tmp-tests/sister-grok-sweep-20260921T014642Z/wt-platforms`
- 分支：`grok/sweep-20260921t014642z-platforms`
- commit：`1e4c2fa1df0f46410a03f3d8b47de074a9eec390`
- **沒有 push。** base 仍是 `a22aabe`，不是現在的 main。
- 報告：`.../platforms/REPORT.md`
- 內容是 schema 3 診斷，不是 Public Preview，也不是 S1 Preview。TCC 與 ScreenCaptureKit 綁在一起；被拒絕就是 `not_attempted`。截到的畫面 OCR 只留 block 數，JSON 裡沒有螢幕文字。bundled 自測圖的文字會進記憶體裡的 `Db`，用「電話」「金額」检索，grounded source 必須指向那一幀。期望值是精確的 `+886800080123` 和 `TWD:13450`。另一個 `0800` 號碼不算。停止栓是 `request_stop` → `consume_stop`，不是錄製驗收。
- 本機在 flock 裡跑過 `cargo test -p sister-capture macos_probe --offline`，14 個測過，含 `a_different_0800_number_is_not_the_self_test_phone`。
- `check-windows.sh` 沒有為這個 slice 跑。desktop 的改動是 `macos_ci.rs`。
- 九個 agent 裡的 platforms worker（掃蕩根目錄的 `/home/ted-h/tmp-tests/sister-grok-sweep-20260921T014642Z/resume-3.py`，不在 repo 裡）在寫 REPORT 和 commit 之前就死了。上面這個 commit 是這個 session 補上 Castle 要求的查詢之後下的。
- 要進 main 之前先 rebase 到 `5cfd22c`（或更新的 main），再看衝突。不要在 `a22aabe` 上 fast-forward。

### 其他掃蕩 worktree

`wt-local_voice`、`wt-usage` 的成果已經在 main 上。不要再從那些 worktree cherry-pick 一次。
掃蕩根目錄的 `INTEGRATION.md`、`manifest.json`、各任務 `REPORT.md` 是過程紀錄，不是產品。

---

## 7. 接手時不要做的事

- 不要切 tag，不要發 release，不要為了這次掃蕩 bump `alpha.145`。使用者沒有要求。
- 不要把 HTTP client 加進 recorder／core／capture／brain／hands，也不要加進 WebView。
- 不要把 LimitReset、BreezyVoice、CDN、Azure 寫進 WebView CSP。
- 不要重做擷取路徑（DXGI、降 `OCR_LONG_EDGE`、再跑一輪 changed-region 對抗）。`AGENTS.md` 第五節的數字仍然有效。
- 不要為了 PDF fixture 再對整棵 Edge 樹 `FindAll`，也不要在 `show("bottom")` 的迴圈裡呼叫 `GetVisibleRanges`。
- 不要用 `git checkout <file>` 還原別人改過的檔。先複製到 `/tmp`。
- `apps/desktop` 的 Prettier 沒有進 CI，已有檔案過不了。不要順手重排。
- 改到 `#[cfg(windows)]`、`crates/sister-capture/src/windows/`、`windows_ocr.rs`、`apps/desktop/` 時，commit 前跑 `./scripts/check-windows.sh`。Linux 的 `cargo test` 不編譯那一半。
- 原生 CI：push 到 `main` 會自己跑 `.github/workflows/ci.yml`。其他分支要 `gh workflow run ci.yml --ref <branch>`。
- 共用 cargo lock 時先跑掃蕩根目錄的 `/home/ted-h/tmp-tests/sister-grok-sweep-20260921T014642Z/fresh-sources.py`。零個測試配到不算通過。

---

## 8. 建議的下一步

1. `5cfd22c` 的 main CI [35645786858](https://github.com/teddashh/AI-Sister/actions/runs/35645786858) 已綠。整合分支同一個 fixture 的 [35645789163](https://github.com/teddashh/AI-Sister/actions/runs/35645789163) 也綠。語音／用量就在 main 這個 HEAD 上。這就是可留的 main。不要自己切 tag。
2. 若下一次 Edge PDF 又紅，先讀 fixture 的 `stage`／`metadata`，不要再加一輪無上限的樹走查。
   - stage 停在 `activating PDF viewport`、metadata 空白：掛點在點擊或 `^{HOME}`，或在就緒迴圈的 `Get-SisterPdfPage`。
   - 第一幀 OCR 只有 `reader.pdf` 和暫存路徑：頁面 canvas 還沒畫進 BitBlt。現在的等待是 2.5 秒。
   - 第二幀沒有 `02-6655-4433`：Ctrl+End 沒有把第二頁底部送進截圖。不要改回短滾輪。
3. macOS probe 若要做，從 `wt-platforms` 的 `1e4c2fa` rebase 到當時的 main，再跑原生 macOS CI。那條不是 Preview。
4. 隱私閘門仍是 `./scripts/check-no-network.sh`、同意書那幾支 `check-consent-*`、`check-no-keylogging.py`。四條 outbound 的名字以 `AGENTS.md` 開頭那節和 `77cc93b`／`c4434f2` 的文件為準。
