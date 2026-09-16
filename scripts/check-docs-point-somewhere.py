#!/usr/bin/env python3
"""文件裡指出去的每一條路，要指得到東西。

這支腳本守兩件事，兩件都是 2026-08-20 那天當場踩到的：

1. **`docs/WINDOWS-CHECKLIST.md` 裡有五行寫著 `%APPDATA%\\sister`。**
   真的位置是 `%APPDATA%\\ted-h\\AI-Sister\\data`，而同一份檔案的第 52 行和
   第 443 行本來就寫對了——五行錯、三行對，全部在同一個檔案裡。那份清單是
   他照著一條一條做的，所以錯的那幾行的後果不是「讀起來怪怪的」，是**他打開
   檔案總管、找不到那個資料夾、然後回報一個沒有壞的東西壞了**。

2. **`scripts/check-combo-is-readable.sh` 改名成 `.py` 之後，清單裡還指著
   `.sh`。** 這一種更安靜：那句話讀起來完全正常，只有真的去 `ls` 才知道。

兩個都是同一個形狀——**一份文件在描述一個它不再對得上的世界**，而沒有任何
東西會為此變紅。清單是這個 repo 唯一一份「只有他那台機器執行得了」的測試，
所以它自己壞掉的時候，壞的是那一整輪驗證。

**`%APPDATA%\\ted-h\\AI-Sister\\data` 這四段全部是從 `config.rs` 的
`default_data_dir()` 推出來的，而且是一次抓完的。** 寫死的話，哪天有人改了那
幾個字串，這支腳本會繼續拿舊的答案去判每一份文件都對。假的那一半要從真的那一
半推出來（見 memory 那條「一個假 DOM 只要有一個欄位比真的寬鬆」）——而且要從
**同一個**真的那一半推：分兩次搜的那一版（同一天寫的）org/app 讀的是整份檔案
裡第一個 `ProjectDirs::from(…)`，那在 config.rs 是 459 行的 `default_path()`，
設定檔那支；accessor 才是從 `default_data_dir()` 讀的。兩個函式各答一半，湊成
一條沒有人產生過的路。實測：只改 `default_path()` 的 app 名，這支腳本會對著八
行完全正確的文件噴紅。

只認**反引號裡**的路徑，不認散文裡的。理由是 `PHASES.md` 有一行寫著
`結構化 grant（task/apps/actions/expiry）`——那是四個欄位名，不是一條路徑，
而任何認得出斜線的正規表示式都會把它抓成違規。一支會誤報的閘門會被關掉，
關掉之後它守的那條線是一格空白。**要它檢查就放進反引號**，那本來就是這幾份
文件的寫法。（`%APPDATA%` 那一圈是例外，它整行掃——那個形狀夠特別，不會誤中。）

**這支腳本守不住的那一半，寫在這裡。**

  - **`%APPDATA%` 以外的絕對路徑一律不看。** `C:\\Program Files\\…`、
    `~/.config/…`、登錄檔路徑寫錯了都不會紅。會挑 `%APPDATA%` 出來是因為它
    是這幾份文件裡唯一一條「他會照著去開檔案總管」的路。
  - **「這條路存在」≠「這條路是對的」。** `docs/PHASES.md` 指得到，但那句話
    指的是不是他要看的那一段，這裡不知道。改名抓得到，改內容抓不到。
  - **只掃根目錄的每一份 `.md`，加上 `docs/**`。** `load_docs()` 是
    `ROOT.glob("*.md")`，不是寫死 `README.md`——今天掃到的是 `AGENTS.md` 和
    `README.md` 兩份，明天根目錄多一份就自動多一份。（這一行原本寫「只掃
    `README.md`」，於是 `AGENTS.md` 讀起來像一格沒人守的空白，而它一直都
    被守著；一份誤報「我沒在看那裡」的自白，和誤報「我有在看那裡」一樣會
    讓下一個人做錯決定。）`crates/**` 裡的 doc comment 沒有人掃，
    `research/` 是刻意排除的（見 `load_docs()`）。
  - 底下那兩個「看了 N 條」是活體檢查的門檻，不是覆蓋率。數到的是這幾份
    文件裡**寫成那個形狀**的那些，不是全部。

    **那兩個數字會往上漂，所以不要把它寫死在句子裡。** 這一行本來寫「22 條
    路 + 8 條 `%APPDATA%`」，2026-09-05 實測是 **79** 和 **10**——文件長大了，
    而句子沒有。同一天在 `die()` 訊息裡還找到兩句同樣過期的：一句說
    「WINDOWS-CHECKLIST.md 有七個」（是八個）、「README 各一」（那一條在
    `RELEASE-NOTES.md`）。三個數字互相打架，而它們全都躺在**只有壞掉才會印
    出來**的字串裡，所以沒有人會發現。

    要看現在是多少就跑一次：`python3 scripts/check-docs-point-somewhere.py`，
    它每一圈都會印。**會出事的是數字往下掉**（glob 掃不到、正規表示式對不
    上），往上長是文件變多，不是故障。
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# 這幾個開頭才算「repo 裡的一條路」。`target/` 刻意不在裡面：那是建置產物，
# 乾淨的 checkout 上本來就沒有。
PREFIX = ("scripts/", "docs/", "crates/", "apps/", ".github/")

# 光禿禿的一個檔名也要認。`PHASES.md:114` 寫的是 `check-no-network.sh`，沒有
# `scripts/` 前綴——只認前綴的話，那一行改名之後不會有任何東西紅。這三種副檔名
# 在這個 repo 裡**只出現在 `scripts/` 底下**（`find` 過），所以往那裡解析是安全
# 的；`.rs` 不在名單裡，因為散文裡的 `main.rs` / `ops.rs` 對得到好幾個檔案。
BARE = re.compile(r"^[A-Za-z0-9_-]+\.(?:mjs|py|sh)$")

failed = []


def die(msg, *rest):
    failed.append(msg)
    print(f"✗ {msg}")
    for line in rest:
        print(f"  {line}")


def read(rel):
    """讀一份檔案。找不到就當場算輸，不是「沒找到違規」。"""
    path = ROOT / rel
    if not path.is_file():
        die(
            f"要掃的檔案不在：{rel}",
            "在修好之前，這支腳本守的那條線是一格空白。",
        )
        return None
    return path.read_text(encoding="utf-8")


def load_docs():
    """要掃的那幾份，連內容一起讀出來。

    空的話當場算輸——glob 打錯一個字，底下每一圈都是空轉，而輸出長得和
    「掃過了，沒事」一模一樣。讀一次收在這裡，兩圈共用：分兩次 glob 的話，
    「一份都沒掃到」會被講兩遍，而那則訊息講的是同一件事。

    **`research/` 是刻意不掃的**，別「順手」加進來：那底下有一份
    `ted-repos-dna.md` 在 `.gitignore` 裡（它抄的是**別的 repo** 的路徑，像
    `apps/companion/src/main/pet/pet.ts`）。加進來的話本機會亮一排紅、而 CI
    上那個檔案根本不存在所以照樣綠——一道兩邊答案不一樣的閘門，比沒有更糟。

    **`docs/**/*.md` 而不是 `docs/*.md`。** 一層的 glob 遇到一次很普通的整理
    （`git mv docs/WINDOWS-CHECKLIST.md docs/windows/`）就會讓整份檔案不再被
    掃，而唯一的痕跡是底下那個「看了 N 條」少了三——一個沒有人在看的數字。
    實測：搬進子目錄、同時種兩個真的錯誤進去，這支腳本 exit 0、印 ✓。
    """
    found = sorted(ROOT.glob("*.md")) + sorted((ROOT / "docs").glob("**/*.md"))
    if not found:
        die("一份 .md 都沒掃到", "glob 對不到東西的時候，底下每一圈都是空轉。")
    return [
        (d.relative_to(ROOT), d.read_text(encoding="utf-8").split("\n")) for d in found
    ]


DOCS = load_docs()


# ── 真的資料目錄長什麼樣 ─────────────────────────────────────────────
#
# 從產生它的那一行推出來，不要寫死。
print("▶ 那個 %APPDATA% 前綴，從 config.rs 推出來")
DATA_PREFIX = None
config = read("crates/sister-core/src/config.rs")
if config is not None:
    # **前綴有四段，上一版只推出了中間那兩段。** `%APPDATA%`（漫遊還是本機）
    # 和結尾的 `\data`，決定它們的是 `default_data_dir()` 呼叫哪一個
    # accessor——而那一行以前沒有人讀。把 `d.data_dir()` 改成
    # `d.data_local_dir()`（對一顆會被 OneDrive 同步的螢幕資料庫來說是很合理的
    # 一個改動，而這份清單自己就在擔心 OneDrive 鎖檔），真的位置變成
    # `%LOCALAPPDATA%\…`，文件裡那八行**全部**變成錯的，而這支腳本照樣綠。
    #
    # docstring 承諾的正是這件事不會發生：「假的那一半要從真的那一半推出來」。
    # 推出來的是四段裡的兩段，而沒推出來的那兩段才是會動的那兩段。
    #
    # **而且要一次抓完，不可以分兩次搜。** 分兩次的那一版是這樣壞的：org/app
    # 用的是整份檔案裡**第一個** `ProjectDirs::from(…)`，那在 config.rs 是第
    # 459 行的 `default_path()`——設定檔那支，不是資料目錄那支；accessor 才是
    # 從 `default_data_dir()` 裡讀的。兩個答案來自兩個不同的函式，湊成一條路，
    # 而它們今天一致純粹是因為那兩行的字串剛好一樣。實測：只把 `default_path()`
    # 的 app 名改成 `AI-Sister-Cfg`（資料目錄一個字都沒動），這支腳本對著八行
    # **完全正確**的文件噴紅，說真的位置是 `…\AI-Sister-Cfg\data\…`——一條兩支
    # 函式都不產生的路。同一族：兩個獨立的來源餵同一句話，中間那條「誰算數」的
    # 規則沒有人審過。
    WINDOWS_DIRS = {
        # `directories` 在 Windows 上的攤法（每一個 accessor 一組）：
        "data_dir": ("%APPDATA%", "data"),
        "data_local_dir": ("%LOCALAPPDATA%", "data"),
        "config_dir": ("%APPDATA%", "config"),
        "config_local_dir": ("%LOCALAPPDATA%", "config"),
        "preference_dir": ("%APPDATA%", "config"),
        "cache_dir": ("%LOCALAPPDATA%", "cache"),
    }
    m = re.search(
        r"pub fn default_data_dir\b[^{]*\{\s*"
        r'(?:\w+::)*ProjectDirs::from\(\s*"[^"]*"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)\s*'
        r"\.map\(\s*\|d\|\s*d\.(\w+)\(\)",
        config,
        re.S,
    )
    if m is None:
        die(
            "config.rs 的 default_data_dir() 讀不出資料目錄是怎麼拼出來的",
            "要嘛不再是 ProjectDirs，要嘛換了寫法。推不出來就別猜——",
            "猜錯的話，底下那一圈會拿一條假的路去判每一份文件，而且全部說對。",
        )
    elif m.group(3) not in WINDOWS_DIRS:
        die(
            f"default_data_dir() 用的是 `{m.group(3)}()`，這裡不知道它在 Windows 上攤成什麼",
            "把它加進 WINDOWS_DIRS，順便確認文件裡那幾行跟著改了。",
        )
    else:
        org, app, accessor = m.group(1), m.group(2), m.group(3)
        root, tail = WINDOWS_DIRS[accessor]
        DATA_PREFIX = rf"{root}\{org}\{app}\{tail}"
        print(f"  真的是：{DATA_PREFIX}（來自 {accessor}()）")

# ── 文件裡的每一條路 ─────────────────────────────────────────────────
BACKTICK = re.compile(r"`([^`\n]+)`")
# **兩個都要認，不是只認寫對的那一個。** 只認 `%APPDATA%` 的話，一份改寫成
# `%LOCALAPPDATA%\…` 的文件對這支腳本是完全隱形的（`%LOCALAPPDATA%` 裡面沒有
# `%APPDATA%` 這個子字串，第一個 `%` 後面接的是 `LOCALAPP`）——而那正是
# accessor 改動之後文件會被改成的樣子：改對了一半、或者改錯了邊，兩種都不紅。
# 認得出來才比得了，比不了就只能沉默。
#
# 正斜線也要收。只收反斜線的話，`%APPDATA%/ted-h/AI-Sister/data` 會被切成一個
# 光禿禿的 `%APPDATA%`（group(1) 是 None）然後當成「沒指定路徑」放過去——寫錯了
# 是沉默，寫對了也是沉默，兩種都不紅。今天這幾份文件裡一條正斜線都沒有（量過），
# 收它是為了「哪天有人順手寫成正斜線」那一次不會變成一格空白。
APPDATA = re.compile(r"%(?:LOCAL)?APPDATA%([\\/][A-Za-z0-9_.\\/-]+)?")

print("▶ 反引號裡的 repo 路徑，指得到東西嗎")
checked = 0
for rel, lines in DOCS:
    for i, line in enumerate(lines, 1):
        for m in BACKTICK.finditer(line):
            token = m.group(1).strip()
            # 佔位符、萬用字元、帶參數的指令一律跳過——它們本來就指不到單一檔案。
            if any(c in token for c in "*<>{}… ()"):
                continue
            token = token.rstrip(".,;:，。、")
            if BARE.match(token):
                token = f"scripts/{token}"
            elif not token.startswith(PREFIX):
                continue
            checked += 1
            if not (ROOT / token).exists():
                die(
                    f"{rel}:{i} 指著一個不在的東西：{token}",
                    line.strip()[:160],
                    "改名或刪掉之後，指著它的那幾句話不會有任何東西幫你找出來。",
                )
print(f"  看了 {checked} 條")
# 活體。這一圈沒有下限的話，`PREFIX` 打錯一個字、或者條目的形狀變了，它會安靜
# 地一條都挑不到然後回報綠——那正是它要抓的那種壞法，發生在它自己身上。隔壁
# `check-checklist-quotes-exist.py` 同一天寫的，只有那一支裝了。
if checked < 15:
    die(
        f"只挑出 {checked} 條路來對，太少了",
        "2026-09-05 量到 79 條（根目錄的 .md + docs/**，反引號裡、對得上 PREFIX/BARE 的）。",
        "多半是 PREFIX / BARE 對不上了，或者 glob 掃不到那幾份檔案。",
    )

print("▶ 每一個 %APPDATA%，要嘛是光禿禿的一個字，要嘛是真的那條路")
seen_appdata = 0
if DATA_PREFIX is not None:
    for rel, lines in DOCS:
        for i, line in enumerate(lines, 1):
            for m in APPDATA.finditer(line):
                whole = m.group(0)
                # 「躺在 `%APPDATA%` 深處」這種講法沒有指定路徑，放它過。
                if m.group(1) is None:
                    continue
                seen_appdata += 1
                # 比之前把分隔符拉齊，不然正斜線那一版會對著正確的路噴紅。
                if whole.replace("/", "\\").startswith(DATA_PREFIX):
                    continue
                die(
                    f"{rel}:{i} 指著一個這台產品上不存在的資料夾：{whole}",
                    line.strip()[:160],
                    f"真的位置是 {DATA_PREFIX}\\…",
                    "他會照著這一行去開檔案總管，找不到，然後回報一個沒有壞的東西壞了。",
                )
    print(f"  看了 {seen_appdata} 條")
    # 活體。這一圈以前連數字都沒有——`APPDATA` 對不上（少認一種寫法、或者文件
    # 改用了正斜線）的時候，它一個都挑不到，而輸出和「八條都對」一模一樣。
    if seen_appdata < 6:
        die(
            f"只挑出 {seen_appdata} 條 %APPDATA% 路徑來對，太少了",
            "2026-09-05 量到 10 條：WINDOWS-CHECKLIST.md 八個，DATA_INVENTORY.md 和 RELEASE-NOTES.md 各一。",
            "多半是 APPDATA 這條正規表示式對不上文件現在的寫法了。",
        )

# ── 答案可以引用哪幾種來源 ───────────────────────────────────────────
#
# 同樣的形狀，第三種：**一份文件在描述一個它不再對得上的世界。**
#
# `SourceRef::parse` 收三種 ref：`fact:`（regex 從 OCR 抽出來的事實）、`chunk:`
# （當時螢幕上真的出現過的原文）、`card:`（她稍早自己寫下的判讀）。`card:` 是
# `672248c` 加的，而 SPEC §8.2、PRODUCT.md 與 PHASES.md 的合約段落當時沒有跟著
# 改，於是三份出貨文件一起說「來源只能是 `fact:<id>`／`chunk:<id>`」——少講的正好
# 是**唯一一種背後沒有螢幕原文的**那一種。讀者（和下一個照著文件做事的我）會得到
# 一個結論：每一句答案背後都有一行螢幕上真的出現過的字。那不是真的。
#
# 所以合法的那份名單從 `SourceRef::parse` 自己那個 `match` 推出來，不寫死：寫死的
# 話，下一次多一種 ref、這支腳本會繼續拿舊的答案去判每一份文件都對。
print("▶ 文件講的來源種類，和 SourceRef 收的那幾種一樣嗎")
# 兩份名單，兩個獨立的來源，然後互相對。
#
# 一份從 `enum SourceRef` 的 variant 讀，一份從 `parse()` 的 match arm 讀。只讀一份的
# 那一版寫過一個寫死的下限（「少於三種就算正規表示式壞了」），而那條判準把兩件事
# 混成同一則訊息：**產品真的少收一種**和**我的正規表示式對不上了**。實測拿掉
# `parse` 裡的 `card` 那一臂，它印的是「多半是那個 match 的寫法變了」——一個當場
# 說謊的診斷。兩份互相對就不用猜：兩邊一致＝兩邊都讀懂了，不一致＝指名是哪一邊。
KINDS = set()
grounded = read("crates/sister-core/src/grounded_answer.rs")
if grounded is not None:
    enum_body = re.search(r"pub enum SourceRef \{(.*?)\n\}", grounded, re.S)
    parse_body = re.search(r"pub fn parse\(value: &str\).*?match kind \{(.*?)\n\s*\}", grounded, re.S)
    if enum_body is None or parse_body is None:
        die(
            "在 grounded_answer.rs 裡找不到 SourceRef 的 enum 或 parse",
            f"enum={enum_body is not None} parse={parse_body is not None}",
            "找不到就推不出合法的 ref 種類，底下那一圈會空轉。",
        )
    else:
        from_enum = {v.lower() for v in re.findall(r"^\s{4}(\w+)\(i64\),", enum_body.group(1), re.M)}
        from_parse = set(re.findall(r'"(\w+)" => Some\(Self::', parse_body.group(1)))
        if not from_enum or from_enum != from_parse:
            die(
                "SourceRef 的 enum 和 parse 對不起來",
                f"enum 有：{sorted(from_enum)}",
                f"parse 收：{sorted(from_parse)}",
                "只差一邊就是真的有一種 ref 沒有入口（或多了一個沒有 variant 的字），"
                "兩邊都空就是這兩條正規表示式對不上這份原始碼了。",
            )
        else:
            KINDS = from_enum
            print(f"  產品收 {len(KINDS)} 種：{'、'.join(sorted(KINDS))}")

# 對的單位是**那串清單本身**，不是行、也不是段。
#
# 按行看：這幾份文件都折行，同一句合約話被折成兩三行，折到第二行的那半自己「只列
# 了一種」——我改完 SPEC 第一次跑就是這樣噴的紅，一句列全了的話被自己的解釋句判成
# 沒列全。按段看更糟，而且是**假綠**：我在同一段裡寫了一句「`card:<id>` 是她稍早
# 寫下的判讀」，於是把合約那句退回只剩兩種，那一段照樣集滿三種，閘門一聲不吭。
# 實測過，這一刀本來是綠的。
#
# 所以抓的是 `` `a:<id>`／`b:<id>`… `` 這種用「／」串起來的**連續**清單（中間准折
# 行）。散文裡單獨提到一種不會被挑到，因為它不是清單。
RUN = re.compile(r"`(\w+):<id>`(?:\s*／\s*`\w+:<id>`)+")
seen_lists = 0
if KINDS:
    want = "／".join(f"`{k}:<id>`" for k in sorted(KINDS))
    for rel, lines in DOCS:
        # RELEASE-NOTES 的版本區段是歷史：alpha.126 那一段當時真的只有兩種，
        # 改成今天的名單才會變成假話。這一條和 check-doc-byte-counts.py 對同一份
        # 檔案的處理是同一個理由。
        if rel.name == "RELEASE-NOTES.md":
            continue
        text = "\n".join(lines)
        for m in RUN.finditer(text):
            seen_lists += 1
            at = text.count("\n", 0, m.start()) + 1
            listed = set(re.findall(r"`(\w+):<id>`", m.group(0)))
            missing, extra = KINDS - listed, listed - KINDS
            if not missing and not extra:
                continue
            detail = []
            if missing:
                detail.append(f"少的是：{'、'.join(sorted(missing))}")
            if extra:
                detail.append(f"多的是（產品收不下）：{'、'.join(sorted(extra))}")
            # 底下那句「為什麼要緊」只對**少列**成立。兩個方向共用一句話的話，
            # 「文件多列了一種」會被配上一段在講漏字的說明——那是這支腳本自己在做
            # 它正在抓的那件事。
            if "card" in missing:
                detail.append(
                    "少列 card: 的後果不是漏字——它是唯一一種背後沒有螢幕原文的來源，"
                    "漏掉它，這句話就在保證一件產品沒有保證的事。"
                )
            elif extra:
                detail.append("文件在保證一件產品現在做不到的事：那種 ref 送回來會被整份拒絕。")
            die(
                f"{rel}:{at} 那串清單列了 {len(listed)} 種來源，產品收 {len(KINDS)} 種",
                " ".join(m.group(0).split()),
                f"合約那句話要列完：{want}",
                *detail,
            )
    print(f"  看了 {seen_lists} 串")
    # 活體。文件改用別的寫法（`fact:123`、去掉反引號、換一個分隔符）的時候，這一圈
    # 會一串都挑不到，而輸出和「三串都對」一模一樣。
    if seen_lists < 3:
        die(
            f"只挑出 {seen_lists} 串在列來源種類，太少了",
            "2026-09-15 量到 3 串：SPEC.md、PRODUCT.md、PHASES.md 的合約段落各一。",
            "多半是那幾份文件改掉了 `fact:<id>`／`chunk:<id>` 這個寫法。",
        )

# 這一圈抓不到什麼：
#   - **只認 `x:<id>` 這個佔位符寫法。** 散文裡寫「fact 和 chunk 兩種」、或者
#     舉具體例子（`card:41`）都不會被挑到。挑佔位符是刻意的——那是合約段落的
#     寫法，而具體例子在歷史區段裡本來就該原封不動。
#   - **沒有用「／」串起來的就不算清單。** 一句合約話如果只寫了 `fact:<id>` 一種、
#     或者改用頓號分隔，這裡看不見它。
#   - **它只問「列全了嗎」，不問旁邊那句解釋對不對。** 把「`card:` 是她稍早的
#     判讀」改成「`card:` 是另一種螢幕原文」照樣綠。

print()
if failed:
    sys.exit(1)
print("✓ 文件裡指出去的每一條路都指得到東西")
