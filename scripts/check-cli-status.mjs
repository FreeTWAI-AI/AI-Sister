#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import assert from 'node:assert/strict';
const html = readFileSync('apps/desktop/ui/index.html', 'utf8');
const source = readFileSync('apps/desktop/ui/cli-status.js', 'utf8');
let passed = 0;
function check(name, condition) { assert.ok(condition, name); console.log(`  ✔ ${name}`); passed++; }
check('quota-absence-explanation', html.includes('不提供剩餘額度與重置時間，這裡不猜'));
check('global-board-not-account-quota', html.includes('全球產品重置紀錄，不是你的帳號額度'));
check('collapsed-on-restart', /<details[^>]*data-cli-status[^>]*>/.test(html) && !/<details[^>]*\bopen\b/.test(html));
check('shipped-script-wired', html.includes('src="./cli-status.js"'));
function boot(responses = {}) {
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
  runInNewContext(source, {document:{querySelector:el,createElement:() => ({textContent:''})},window});
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
check('cancel-discards-late-account',!q.rows().includes('late account')&&q.rows().includes('查詢已停止'));
check('cancel-reaches-native',q.calls.some(c=>c[0]==='cli_status_cancel'));
p.el('[data-cli-status]').open=false;p.el('[data-cli-status]').handlers.toggle();await tick();
check('idle-collapse-clears-account-without-evicting-cache',!p.rows().includes('fixture@example.test')&&!p.calls.some(c=>c[0]==='cli_status_cancel'));
p.events['usage-status-changed']({payload:{config_readable:true,enabled:true,board_live:false,last_success_unix_ms:1,products:[{name:'Claude',reset:'confirmed',announced_at:'fixture-time'}]}});
check('closed-panel-not-repainted',p.el('[data-cli-board]').textContent.includes('未開啟'));
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
console.log(`✓ ${passed} checks passed`);
