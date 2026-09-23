#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { spawnSync } from 'node:child_process';
const html = readFileSync('apps/desktop/ui/index.html', 'utf8');
const source = readFileSync('apps/desktop/ui/cli-status.js', 'utf8');
const nativeSource = readFileSync('apps/desktop/src-tauri/src/main.rs', 'utf8');
const usageViewSource = readFileSync('crates/sister-usage/src/view.rs', 'utf8');
let passed = 0;
let failedChecks = 0;
function check(name, condition) {
  console.log(`  ${condition ? '✔' : '✗'} ${name}`);
  if (condition) passed++; else failedChecks++;
}
function bracedBody(text, startPattern) {
  const start = text.search(startPattern);
  if (start < 0) return null;
  const open = text.indexOf('{', start);
  if (open < 0) return null;
  let depth = 0;
  for (let index = open; index < text.length; index++) {
    if (text[index] === '{') depth++;
    else if (text[index] === '}' && --depth === 0) return text.slice(open + 1, index);
  }
  return null;
}
function captures(text, pattern) {
  return [...text.matchAll(pattern)].map(match => match[1]);
}
const nativeReadBody = bracedBody(nativeSource, /async fn cli_status_read\s*\(/);
const failureReasonBody = bracedBody(source, /function failureReason\s*\(/);
check('native-stop-function-extracted', nativeReadBody !== null);
check('renderer-failure-function-extracted', failureReasonBody !== null);
const nativeStopReasons = [...(nativeReadBody ?? '').matchAll(/Err\(\s*(["'])(.*?)\1\.to_owned\(\)\s*\)/g)].map(match => match[2]);
const rendererStopReasons = [...(failureReasonBody ?? '').matchAll(/return\s+reason\s*===\s*(["'])(.*?)\1/g)].map(match => match[2]);
check('native-stop-reason-extracted', nativeStopReasons.length > 0);
check('renderer-stop-reason-extracted', rendererStopReasons.length > 0);
check('native-renderer-stop-reason-match', nativeStopReasons.length === 1 && rendererStopReasons.length === 1 && nativeStopReasons[0] === rendererStopReasons[0]);
if (nativeStopReasons[0] !== rendererStopReasons[0]) {
  console.log(`    native cli_status_read: ${JSON.stringify(nativeStopReasons)}`);
  console.log(`    renderer failureReason: ${JSON.stringify(rendererStopReasons)}`);
}
const rendererStopReason = rendererStopReasons[0];

const paintBoardBody = bracedBody(source, /function paintBoard\s*\(/);
const statusStructBody = bracedBody(usageViewSource, /pub struct UsageStatusView\s*/);
const productStructBody = bracedBody(usageViewSource, /pub struct UsageProductView\s*/);
check('paint-board-function-extracted', paintBoardBody !== null);
check('usage-status-fields-extracted', captures(statusStructBody ?? '', /^\s*pub\s+([A-Za-z_]\w*)\s*:/gm).length > 0);
check('usage-product-fields-extracted', captures(productStructBody ?? '', /^\s*pub\s+([A-Za-z_]\w*)\s*:/gm).length > 0);
const statusFields = new Set(captures(statusStructBody ?? '', /^\s*pub\s+([A-Za-z_]\w*)\s*:/gm));
const productFields = new Set(captures(productStructBody ?? '', /^\s*pub\s+([A-Za-z_]\w*)\s*:/gm));
const readStatusFields = [...new Set(captures(paintBoardBody ?? '', /\bview\??\.([A-Za-z_]\w*)/g))].sort();
const readProductFields = [...new Set(captures(paintBoardBody ?? '', /\bp\.([A-Za-z_]\w*)/g))].sort();
check('paint-board-status-fields-extracted', readStatusFields.length > 0);
check('paint-board-product-fields-extracted', readProductFields.length > 0);
const missingStatusFields = readStatusFields.filter(field => !statusFields.has(field));
const missingProductFields = readProductFields.filter(field => !productFields.has(field));
check('paint-board-status-fields-declared', missingStatusFields.length === 0);
check('paint-board-product-fields-declared', missingProductFields.length === 0);
if (missingStatusFields.length) console.log(`    undeclared UsageStatusView fields: ${JSON.stringify(missingStatusFields)}`);
if (missingProductFields.length) console.log(`    undeclared UsageProductView fields: ${JSON.stringify(missingProductFields)}`);

const labelsBody = bracedBody(paintBoardBody ?? '', /const labels\s*=\s*/);
const resetMatchBody = bracedBody(usageViewSource, /let \(reset, event_id, announced_at\)\s*=\s*match\s+&row\.reset/);
const labelKeys = captures(labelsBody ?? '', /\b([A-Za-z_]\w*)\s*:/g).sort();
const resetValues = [...new Set(captures(resetMatchBody ?? '', /(?:=>\s*|\(\s*)["']([^"']+)["']\s*,/g))].sort();
check('paint-board-labels-extracted', labelKeys.length > 0);
check('rust-reset-values-extracted', resetValues.length > 0);
check('paint-board-labels-match-rust-reset-values', JSON.stringify(labelKeys) === JSON.stringify(resetValues));
if (JSON.stringify(labelKeys) !== JSON.stringify(resetValues)) {
  console.log(`    renderer labels: ${JSON.stringify(labelKeys)}`);
  console.log(`    Rust reset values: ${JSON.stringify(resetValues)}`);
}
check('quota-absence-explanation', html.includes('不提供剩餘額度與重置時間，這裡不猜'));
check('global-board-not-account-quota', html.includes('全球產品重置紀錄，不是你的帳號額度'));
check('collapsed-on-restart', /<details[^>]*data-cli-status[^>]*>/.test(html) && !/<details[^>]*\bopen\b/.test(html));
check('shipped-script-wired', html.includes('src="./cli-status.js"'));
function boot(responses = {}, desktop = true) {
  const elements = new Map();
  const events = {};
  const calls = [];
  const el = key => {
    if (!elements.has(key)) elements.set(key, {textContent:'', open:false, hidden:false, disabled:false, handlers:{}, children:[], addEventListener(n,f){this.handlers[n]=f;}, replaceChildren(...v){this.children=v;}});
    return elements.get(key);
  };
  const window = { addEventListener(){}, __TAURI__: {core: {invoke(name,args){calls.push([name,args]); return Promise.resolve().then(() => {
    const r = responses[name];
    if (r instanceof Error) throw r;
    return typeof r === 'function' ? r() : r;
  });}}, event:{listen(name,fn){events[name]=fn;}}}};
  if (!desktop) delete window.__TAURI__;
  runInNewContext(source, {__TAURI__:window.__TAURI__,document:{querySelector:el,createElement:() => ({textContent:''})},window});
  return {el,calls,events, rows:()=>el('[data-cli-rows]').children.map(v=>v.textContent).join('\n')};
}
const tick = () => new Promise(r => setTimeout(r, 10));
const p = boot({cli_status_read:[{id:'claude',status:'已登入（oauth） · 帳號：fixture@example.test · 方案：pro'},{id:'codex',status:'已登入（ChatGPT）'},{id:'grok',status:'已安裝'},{id:'gemini',status:'未安裝'}],cli_status_outbound:0,usage_status_read:{config_readable:true,enabled:false}});
check('no-startup-probes',p.calls.length===0);
p.el('[data-cli-status]').open=true;p.el('[data-cli-status]').handlers.toggle();await tick();
check('four-rows',p.el('[data-cli-rows]').children.length===4);
check('claude-reported-fields',p.rows().includes('fixture@example.test · 方案：pro'));
check('codex-no-account-explanation',p.rows().includes('已登入（ChatGPT）；這支 CLI 不報帳號'));
check('grok-no-login-query-explanation',p.rows().includes('已安裝；這支 CLI 沒有提供登入狀態查詢'));
check('gemini-missing',p.rows().includes('gemini：未安裝'));
check('measured-zero-is-zero',p.el('[data-cli-outbound]').textContent.includes('0 趟'));
check('disabled-board-visible',p.el('[data-cli-board]').textContent.includes('未開啟'));
p.el('[data-cli-refresh]').handlers.click({isTrusted:false});await tick();
check('synthetic-refresh-no-probe',p.calls.filter(c=>c[0]==='cli_status_read').length===1);
for (const state of ['未登入','未安裝','問不到；請重新查詢']) {
 const q=boot({cli_status_read:['claude','codex','grok','gemini'].map(id=>({id,status:state})),cli_status_outbound:new Error('failed')});
 q.el('[data-cli-status]').open=true;q.el('[data-cli-status]').handlers.toggle();await tick();
 check(`all-rows-${state}`,q.el('[data-cli-rows]').children.every(r=>r.textContent.includes(state)));
 check(`unknown-count-not-zero-${state}`,q.el('[data-cli-outbound]').textContent.includes('問不到')&&!q.el('[data-cli-outbound]').textContent.includes('0 趟'));
}
let resolve;
const q=boot({cli_status_read:()=>new Promise(r=>{resolve=r;})});
q.el('[data-cli-status]').open=true;q.el('[data-cli-status]').handlers.toggle();await tick();
q.el('[data-cli-cancel]').handlers.click({isTrusted:true});await tick();
resolve([{id:'claude',status:'late account'}]);await tick();
check('cancel-discards-late-account',!q.rows().includes('late account')&&q.rows().includes(rendererStopReason));
check('cancel-reaches-native',q.calls.some(c=>c[0]==='cli_status_cancel'));
p.el('[data-cli-status]').open=false;p.el('[data-cli-status]').handlers.toggle();await tick();
check('idle-collapse-clears-account-without-evicting-cache',!p.rows().includes('fixture@example.test')&&!p.calls.some(c=>c[0]==='cli_status_cancel'));
p.events['usage-status-changed']({payload:{config_readable:true,enabled:true,board_live:false,last_success_unix_ms:1,products:[{name:'Claude',reset:'confirmed',announced_at:'fixture-time'}]}});
check('closed-panel-not-repainted',p.el('[data-cli-board]').textContent.includes('已停止'));
p.el('[data-cli-status]').open=true;
p.events['usage-status-changed']({payload:{config_readable:true,enabled:true,board_live:false,last_success_unix_ms:1,products:[{name:'Claude',reset:'confirmed',announced_at:'fixture-time'}]}});
check('stored-global-board-labelled',p.el('[data-cli-board]').textContent.includes('上次儲存的紀錄：Claude：已驗證重置（fixture-time）'));
let finishClosed;
const closed = boot({cli_status_read:()=>new Promise(r=>{finishClosed=r;})});
closed.el('[data-cli-status]').open=true;closed.el('[data-cli-status]').handlers.toggle();await tick();
closed.el('[data-cli-status]').open=false;closed.el('[data-cli-status]').handlers.toggle();await tick();
finishClosed([{id:'claude',status:'late hidden account'}]);await tick();
check('active-collapse-cancels-native',closed.calls.some(c=>c[0]==='cli_status_cancel'));
check('active-collapse-discards-late-result',!closed.rows().includes('late hidden account'));
const failed = boot({cli_status_read:new Error('IPC failed')});
failed.el('[data-cli-status]').open=true;failed.el('[data-cli-status]').handlers.toggle();await tick();
check('ipc-failure-all-rows-unknown',failed.el('[data-cli-rows]').children.every(r=>r.textContent.includes('問不到')));
check('ipc-failure-can-retry',!failed.el('[data-cli-refresh]').disabled && failed.el('[data-cli-cancel]').hidden);
let browser;
const browserErrors = [];
const catchBrowserError = error => browserErrors.push(error);
process.on('unhandledRejection', catchBrowserError);
try {
  browser = boot({}, false);
  browser.el('[data-cli-status]').open = true;
  browser.el('[data-cli-status]').handlers.toggle();
} catch (error) { browserErrors.push(error); }
await tick();
process.off('unhandledRejection', catchBrowserError);
check('browser-only-expand-no-exception', browserErrors.length === 0 && browser?.calls.length === 0);
check('browser-only-explains-desktop-required', browser?.rows().includes('桌面程式') &&
  ['[data-cli-outbound]', '[data-cli-board]'].every(key => browser?.el(key).textContent.includes('桌面程式')));
check('browser-only-no-retry-action', browser?.el('[data-cli-refresh]').disabled && browser.el('[data-cli-cancel]').hidden);
for (const error of [rendererStopReason, new Error(rendererStopReason)]) {
  const stopped = boot(Object.fromEntries(['cli_status_read', 'cli_status_outbound', 'usage_status_read'].map(name => [name, () => Promise.reject(error)])));
  stopped.el('[data-cli-status]').open = true;
  stopped.el('[data-cli-status]').handlers.toggle();
  await tick();
  for (const [name, content] of [['rows', stopped.rows()], ['outbound', stopped.el('[data-cli-outbound]').textContent], ['board', stopped.el('[data-cli-board]').textContent]]) {
    check(`stopped-expand-${typeof error}-${name}-keeps-reason`, content.includes(rendererStopReason));
    check(`stopped-expand-${typeof error}-${name}-no-retry-advice`, !content.includes('重新查詢'));
  }
}
const stoppedView = boot({cli_status_read: () => Promise.reject(rendererStopReason), cli_status_outbound: 3, usage_status_read: { stopped: true, config_readable: false }});
stoppedView.el('[data-cli-status]').open = true;
stoppedView.el('[data-cli-status]').handlers.toggle();
await tick();
check('stopped-board-view-before-config-failure', stoppedView.el('[data-cli-board]').textContent.includes('已停止') && !stoppedView.el('[data-cli-board]').textContent.includes('重新查詢'));
check('stopped-outbound-preserves-measured-count', stoppedView.el('[data-cli-outbound]').textContent.includes('3 趟'));
check('ipc-failure-outbound-can-retry', failed.el('[data-cli-outbound]').textContent.includes('請重新查詢'));
check('ipc-failure-board-can-retry', failed.el('[data-cli-board]').textContent.includes('重新查詢'));
// Feed the production runner's real timeout result to the shipped renderer.
const native = spawnSync('cargo', ['test', '-p', 'sister-core', '--test', 'desktop_cli_status', 'actual_timeout_is_unknown', '--', '--nocapture'], { encoding: 'utf8', timeout: 120_000 });
process.stdout.write(native.stdout ?? '');
if (native.status !== 0) process.stderr.write(native.stderr ?? '');
const totals = [...(native.stdout ?? '').matchAll(/(\d+) passed; (\d+) failed/g)];
check('timeout-native-test-ran', totals.reduce((sum, m) => sum + Number(m[1]) + Number(m[2]), 0) >= 1);
const receipt = native.stdout?.match(/^CLI_STATUS_TIMEOUT_ROW=(.+)$/m);
check('timeout-native-row-received', Boolean(receipt));
const timed = boot({cli_status_read: receipt ? [JSON.parse(receipt[1])] : []});
timed.el('[data-cli-status]').open = true;
timed.el('[data-cli-status]').handlers.toggle();
await tick();
const codexRow = timed.el('[data-cli-rows]').children[1]?.textContent ?? '';
check('timeout-row-says-unknown', codexRow.startsWith('codex：問不到'));
check('timeout-row-no-fabricated-number', !/[0-9]/.test(codexRow));
check('timeout-native-test-passed', native.status === 0 && totals.length > 0 && totals.every(m => Number(m[2]) === 0));
console.log(`✓ ${passed} checks passed; ${failedChecks} failed`);
if (failedChecks) process.exitCode = 1;
