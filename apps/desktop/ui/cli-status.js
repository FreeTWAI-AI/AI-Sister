// Only explicit expansion/refresh probes CLIs. Account snapshots stay in memory.
const panel = document.querySelector('[data-cli-status]');
const rows = document.querySelector('[data-cli-rows]');
const refresh = document.querySelector('[data-cli-refresh]');
const cancel = document.querySelector('[data-cli-cancel]');
const outbound = document.querySelector('[data-cli-outbound]');
const board = document.querySelector('[data-cli-board]');
const invoke = (name, args) => window.__TAURI__.core.invoke(name, args);
const ids = ['claude', 'codex', 'grok', 'gemini'];
let generation = 0;
let busy = false;
function paintRows(values, fallback = '問不到；請重新查詢') {
  rows.replaceChildren(...ids.map(id => {
    const li = document.createElement('li');
    const value = values?.find(row => row.id === id)?.status;
    const note = id === 'codex' ? '；這支 CLI 不報帳號' :
      id === 'grok' ? '；這支 CLI 沒有提供登入狀態查詢' :
      id === 'gemini' ? '；此處只查安裝狀態' : '';
    li.textContent = `${id}：${typeof value === 'string' && value ? value : fallback}${note}`;
    return li;
  }));
}
function paintBoard(view) {
  if (!view?.config_readable) board.textContent = '公開看板問不到；可重新查詢';
  else if (view.stopped) board.textContent = '公開看板已停止';
  else if (!view.enabled) board.textContent = '公開看板未開啟，可到設定開啟';
  else {
    const labels = {confirmed: '已驗證重置', unverified: '事件未驗證，不當成重置', other: '不是重置的事件', none: '沒有已驗證重置事件'};
    const lines = (view.products ?? []).map(p => `${p.name}：${labels[p.reset] ?? '問不到'}${p.announced_at ? `（${p.announced_at}）` : ''}`);
    const source = view.board_live ? '' : view.last_success_unix_ms != null ? '上次儲存的紀錄：' : '尚未查到公開看板：';
    board.textContent = `${source}${lines.join('；') || '尚無紀錄'}。${view.attribution ?? ''}`;
  }
}
async function readStatus(force = false) {
  if (busy || !panel.open) return;
  const current = ++generation;
  busy = true;
  refresh.disabled = true;
  cancel.hidden = false;
  paintRows([], '查詢中');
  outbound.textContent = '今天 AI-Sister 的大腦外送：查詢中';
  const valid = () => current === generation && panel.open;
  await Promise.allSettled([
    invoke('cli_status_read', { refresh: force }).then(value => {
      if (valid()) paintRows(value);
    }).catch(() => { if (valid()) paintRows([]); }),
    invoke('cli_status_outbound').then(value => {
      if (valid()) outbound.textContent = Number.isSafeInteger(value) && value >= 0
        ? `今天 AI-Sister 的大腦外送：${value} 趟（所有 CLI 合計）`
        : '今天 AI-Sister 的大腦外送：問不到；請重新查詢';
    }).catch(() => { if (valid()) outbound.textContent = '今天 AI-Sister 的大腦外送：問不到；請重新查詢'; }),
    invoke('usage_status_read').then(value => { if (valid()) paintBoard(value); })
      .catch(() => { if (valid()) paintBoard(null); }),
  ]);
  if (current === generation) {
    busy = false;
    refresh.disabled = false;
    cancel.hidden = true;
  }
}
async function stop() {
  const wasBusy = busy;
  ++generation;
  busy = false;
  refresh.disabled = true;
  cancel.hidden = true;
  paintRows([], '查詢已停止');
  outbound.textContent = '今天 AI-Sister 的大腦外送：查詢已停止';
  try { if (wasBusy) await invoke('cli_status_cancel'); }
  finally { refresh.disabled = false; }
}
panel.addEventListener('toggle', () => {
  if (panel.open) void readStatus();
  else void stop().catch(() => {});
});
refresh.addEventListener('click', event => { if (event.isTrusted) void readStatus(true); });
cancel.addEventListener('click', event => { if (event.isTrusted) void stop().catch(() => {}); });
window.addEventListener('pagehide', () => { void stop().catch(() => {}); });
window.__TAURI__?.event?.listen?.('usage-status-changed', event => {
  if (panel.open) paintBoard(event.payload);
});
window.__TAURI__?.event?.listen?.('master-stop-changed', () => {
  void stop().catch(() => {});
});
