// 'independent' — each account gets its own free-floating card across the screen.
// 'grouped'     — mini-cards stay inside the order card and it flies right as one.
const DISPATCH_STYLE = 'independent';

// Preview-only: how many fake accounts to animate when no server is running.
const DEMO_ACCOUNT_COUNT = 12;

const $ = (id) => document.getElementById(id);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// ============================================================================
// Order card
// ============================================================================

const card = $('order-card');
const cardBody = $('panel-order');
const form = $('order-form');
const submitBtn = $('submit-btn');
const resultsEl = $('results');
const noticeEl = $('notice');
const dispatchGrid = $('dispatch-grid');
const dispatchOverlay = $('dispatch-overlay');
const confirmOverlay = $('confirm-overlay');

let side = 'buy';

// Ideas loaded from /ideas — shared by the Order, Modify and Close panels.
let ideas = [];

// Market (unchecked): price field locked, fills at market.
// Pending (checked): price required, pre-filled with the Idea's planned entry.
const pendingBox = $('pending');
const priceInput = $('price');

function setOrderType(pending) {
  pendingBox.checked = pending;
  priceInput.disabled = !pending;
  priceInput.required = pending;
  priceInput.placeholder = pending ? 'Limit / stop price' : 'Market';
  if (!pending) priceInput.value = '';
  else if (!priceInput.value) {
    const idea = ideas.find((i) => i.id === $('order_idea').value);
    if (idea) priceInput.value = idea.planned_entry;
  }
}
pendingBox.addEventListener('change', () => {
  setOrderType(pendingBox.checked);
  if (pendingBox.checked) priceInput.focus();
});

async function postOrder(payload) {
  try {
    const res = await fetch('/order', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    return await res.json();
  } catch (err) {
    return { error: 'Preview mode: no server connected at /order. Run the oms binary to dispatch real orders.' };
  }
}

async function runGroupedDispatch(accountNames) {
  dispatchGrid.innerHTML = '';
  accountNames.forEach((name) => {
    const el = document.createElement('div');
    el.className = 'mini-card';
    el.textContent = name;
    dispatchGrid.appendChild(el);
  });
  cardBody.style.display = 'none';
  dispatchGrid.classList.add('active');

  await sleep(20);
  const els = dispatchGrid.querySelectorAll('.mini-card');
  els.forEach((el, i) => setTimeout(() => el.classList.add('show'), i * 30));
  await sleep(els.length * 30 + 280);

  card.style.transform = 'translateX(160%) rotate(3deg)';
  card.style.opacity = '0';
  await sleep(550);

  dispatchGrid.classList.remove('active');
  dispatchGrid.innerHTML = '';
  cardBody.style.display = '';
}

async function runIndependentDispatch(accountNames) {
  card.style.opacity = '0';
  await sleep(260);
  card.style.visibility = 'hidden'; // keeps its slot so the idea card doesn't jump

  dispatchOverlay.innerHTML = '';
  accountNames.forEach((name) => {
    const el = document.createElement('div');
    el.className = 'solo-card';
    el.textContent = name;
    dispatchOverlay.appendChild(el);
  });
  dispatchOverlay.classList.add('active');

  await sleep(20);
  const els = dispatchOverlay.querySelectorAll('.solo-card');
  els.forEach((el, i) => setTimeout(() => el.classList.add('show'), i * 25));
  await sleep(els.length * 25 + 350);

  els.forEach((el, i) => setTimeout(() => el.classList.add('dispatch'), i * 15));
  await sleep(els.length * 15 + 550);

  dispatchOverlay.classList.remove('active');
  dispatchOverlay.innerHTML = '';
  card.style.visibility = '';
}

async function flashConfirm() {
  confirmOverlay.style.opacity = '1';
  await sleep(1100);
  confirmOverlay.style.opacity = '0';
  await sleep(300);
}

function renderResults(data) {
  resultsEl.innerHTML = '';
  noticeEl.textContent = '';
  if (data.error) {
    noticeEl.textContent = data.error;
    return;
  }
  for (const r of data.results || []) {
    resultsEl.appendChild(resultRow(r.account, r.ok, r.ok ? 'Sent' : 'Failed', r.ok ? '' : r.message || r.error));
  }
}

function resultRow(account, ok, statusText, reasonText) {
  const row = document.createElement('div');
  row.className = 'result-row ' + (ok ? 'ok' : 'fail');

  const left = document.createElement('div');
  left.style.flex = '1';
  const name = document.createElement('span');
  name.className = 'name';
  name.textContent = account;
  left.appendChild(name);
  if (reasonText) {
    const reason = document.createElement('div');
    reason.className = 'reason';
    reason.textContent = reasonText;
    left.appendChild(reason);
  }

  const status = document.createElement('span');
  status.className = 'status';
  status.textContent = statusText;

  row.append(left, status);
  return row;
}

function dropCardInFromTop() {
  card.style.transition = 'none';
  card.style.transform = 'translateY(-160%)';
  card.style.opacity = '0';
  card.offsetHeight; // force reflow so the next transition runs
  card.style.transition = '';
  card.style.transform = '';
  card.style.opacity = '';
}

form.addEventListener('submit', async (e) => {
  e.preventDefault();
  submitBtn.disabled = true;
  submitBtn.textContent = 'Dispersing…';
  closeAccountPanel();

  const ideaId = $('order_idea').value;
  const idea = ideas.find((i) => i.id === ideaId);
  const pending = pendingBox.checked;

  // Instrument, side, setup and levels all come from the Idea.
  const data = await postOrder({
    idea_id: ideaId,
    instrument: idea.symbol,
    side: idea.side,
    setup: idea.setup,
    stop_loss: idea.stop_loss,
    take_profit: idea.take_profit,
    order_type: pending ? 'pending' : 'market',
    price: pending ? priceInput.value.trim() : null,
  });

  const accountNames = (data.results && data.results.length)
    ? data.results.map((r) => r.account)
    : Array.from({ length: DEMO_ACCOUNT_COUNT }, (_, i) => `Account ${i + 1}`);

  if (DISPATCH_STYLE === 'independent') {
    await runIndependentDispatch(accountNames);
  } else {
    await runGroupedDispatch(accountNames);
  }

  await flashConfirm();

  $('order_idea').value = ideaId; // pyramid adds usually reuse the same Idea
  setOrderType(false);
  renderResults(data);
  setPositionStates(statusFromOrder(data, accountNames));
  startStatusPolling();
  dropCardInFromTop();

  submitBtn.disabled = false;
  submitBtn.textContent = 'Place Order';
});

// ============================================================================
// Account status dots
//   One dot per account holding a live position/order from an attempted
//   execution. States follow state_diagram:
//     Order:    sending · pending · filled · rejected · canceled · expired
//     Position: open · liquidating · closed
//   canceled / expired / closed → no dot. No Order Intent sent → no dots.
//   GET /positions/status -> [{ account, state }]   (polled while dots exist)
// ============================================================================

const statusDots = $('status-dots');
const STATUS_POLL_MS = 3000;
let statusTimer = null;

// account -> position/order state from the last Order Intent (absent = none).
const positionStates = {};
// Every connected account with its limits — from GET /accounts/risk.
let accountRisk = [];

function accountLevel(a) {
  return worstLevel([
    riskLevel(a.daily_loss_pct, a.daily_loss_limit_pct),
    riskLevel(a.max_dd_pct, a.max_dd_limit_pct),
  ]);
}

function renderStatusDots() {
  const selected = accountPanel.dataset.account;
  statusDots.innerHTML = '';
  for (const a of accountRisk) {
    const level = accountLevel(a);
    const pos = positionStates[a.account];
    const dot = document.createElement('button');
    dot.type = 'button';
    dot.className = 'status-dot ' + level;
    dot.dataset.account = a.account;
    dot.dataset.state = pos || 'none';
    dot.dataset.label = a.account + ' · ' + level + (pos ? ' · ' + pos : '');
    dot.setAttribute('aria-label', dot.dataset.label);
    if (a.account === selected) dot.classList.add('selected');
    dot.addEventListener('click', (ev) => {
      ev.stopPropagation();
      toggleAccountPanel(a.account);
    });
    statusDots.appendChild(dot);
  }
  if (selected) {
    if (!statusDots.querySelector('.selected')) closeAccountPanel();
    else renderAccountMeters(selected);
  }
}

function setPositionStates(rows) {
  for (const k of Object.keys(positionStates)) delete positionStates[k];
  for (const r of rows) if (DOT_STATES.includes(r.state)) positionStates[r.account] = r.state;
  renderStatusDots();
}

// Position states that still count as "holding something" from the last intent.
const DOT_STATES = ['sending', 'pending', 'filled', 'open', 'liquidating', 'rejected'];

async function pollStatus() {
  try {
    const res = await fetch('/positions/status');
    if (!res.ok) return;
    setPositionStates(await res.json());
  } catch (err) {
    // Preview mode — keep whatever the last order produced.
  }
}

function startStatusPolling() {
  if (!statusTimer) statusTimer = setInterval(pollStatus, STATUS_POLL_MS);
}

// Seed position states from an /order response (or demo data in preview mode).
function statusFromOrder(data, accountNames) {
  if (data.results && data.results.length) {
    return data.results.map((r) => ({ account: r.account, state: r.ok ? 'pending' : 'rejected' }));
  }
  const demo = ['open', 'open', 'pending', 'open', 'rejected', 'open', 'liquidating', 'pending'];
  return accountNames.map((account, i) => ({ account, state: demo[i % demo.length] }));
}

// ============================================================================
// Account detail panel — click a dot to see that account's positions
//   GET /accounts/{account}/positions ->
//     { order_status, position_state,
//       positions: [{ symbol, side, qty, entry_price, stop_loss_price,
//                     take_profit_price, unrealized_pnl, entry_time,
//                     broker_position_id }] }
// ============================================================================

const accountPanel = $('account-panel');
const positionList = $('position-list');

function labelled(tag, className, label, value) {
  const el = document.createElement(tag);
  if (className) el.className = className;
  const l = document.createElement('span');
  l.textContent = label;
  el.append(l, document.createTextNode(value ?? '—'));
  return el;
}

function demoPositions(account) {
  const dot = statusDots.querySelector(`[data-account="${account}"]`);
  const state = dot ? dot.dataset.state : 'open';
  if (state === 'none') return { order_status: '—', position_state: '—', positions: [] };
  if (state === 'rejected' || state === 'pending' || state === 'sending') {
    return { order_status: state, position_state: '—', positions: [] };
  }
  // Preview: an initial entry plus one pyramid add. The stop has been moved
  // into profit and the add shares that same stop (per the pyramiding rule).
  const now = Date.now();
  return {
    order_status: 'filled',
    position_state: state,
    positions: [
      {
        symbol: 'EURUSD', side: 'sell', qty: '1.20',
        entry_price: '1.13904', stop_loss_price: '1.13600', take_profit_price: '1.12757',
        unrealized_pnl: '664.80', entry_time: new Date(now - 5 * 3600e3).toISOString(),
        broker_position_id: '72109440' + account.replace(/\D/g, ''),
      },
      {
        symbol: 'EURUSD', side: 'sell', qty: '0.85',
        entry_price: '1.13352', stop_loss_price: '1.13600', take_profit_price: '1.12757',
        unrealized_pnl: '-22.10', entry_time: new Date(now - 20 * 60e3).toISOString(),
        broker_position_id: '72111873' + account.replace(/\D/g, ''),
      },
    ],
  };
}

function renderAccountMeters(account) {
  const a = accountRisk.find((r) => r.account === account);
  if (!a) return;
  renderMeter($('meter-daily'), a.daily_loss_pct, a.daily_loss_limit_pct);
  renderMeter($('meter-maxdd'), a.max_dd_pct, a.max_dd_limit_pct);
}

function renderAccountDetail(account, detail) {
  $('account-name').textContent = account;
  renderAccountMeters(account);

  const stateEl = $('account-state');
  stateEl.innerHTML = '';
  stateEl.append(
    labelled('div', 'state-chip', 'Order', detail.order_status),
    labelled('div', 'state-chip', 'Position', detail.position_state),
  );

  positionList.innerHTML = '';
  if (!detail.positions.length) {
    const p = document.createElement('p');
    p.className = 'notice';
    p.textContent = detail.order_status === 'rejected'
      ? 'Order was rejected — no position opened.'
      : detail.order_status === '—' || !detail.order_status
        ? 'No position from the last Order Intent.'
        : 'No filled position yet.';
    positionList.appendChild(p);
    return;
  }

  // Oldest first, so the initial entry is on top and adds follow in order.
  const positions = [...detail.positions].sort((a, b) => new Date(a.entry_time) - new Date(b.entry_time));

  // Totals across every position on the account.
  if (positions.length > 1) {
    const lots = positions.reduce((s, p) => s + Number(p.qty), 0);
    const total = positions.reduce((s, p) => s + Number(p.unrealized_pnl), 0);
    const summary = document.createElement('div');
    summary.className = 'position-summary';
    summary.append(
      span('', `${positions.length} positions · ${lots.toFixed(2)} lots`),
      span('position-pnl ' + (total >= 0 ? 'up' : 'down'), (total >= 0 ? '+' : '') + total.toFixed(2)),
    );
    positionList.appendChild(summary);
  }

  positions.forEach((pos, i) => {
    const el = document.createElement('div');
    el.className = 'position';

    const pnl = Number(pos.unrealized_pnl);
    const head = document.createElement('div');
    head.className = 'position-head';
    head.append(
      span('idea-symbol', pos.symbol),
      span('idea-side ' + pos.side, pos.side),
      span('position-tag', i === 0 ? 'Initial' : `Add ${i}`),
      span('position-pnl ' + (pnl >= 0 ? 'up' : 'down'), (pnl >= 0 ? '+' : '') + pnl.toFixed(2)),
    );

    const grid = document.createElement('div');
    grid.className = 'position-grid';
    grid.append(
      labelled('div', '', 'Qty', pos.qty),
      labelled('div', '', 'Entry', pos.entry_price),
      labelled('div', '', 'Opened', new Date(pos.entry_time).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })),
      labelled('div', '', 'Stop', pos.stop_loss_price),
      labelled('div', '', 'Target', pos.take_profit_price),
    );

    const meta = document.createElement('div');
    meta.className = 'position-meta';
    meta.textContent = 'Broker position ' + pos.broker_position_id;

    el.append(head, grid, meta);
    positionList.appendChild(el);
  });
}

async function openAccountPanel(account) {
  accountPanel.dataset.account = account;
  statusDots.querySelectorAll('.status-dot').forEach((d) =>
    d.classList.toggle('selected', d.dataset.account === account));

  let detail;
  try {
    const res = await fetch(`/accounts/${encodeURIComponent(account)}/positions`);
    if (!res.ok) throw new Error(res.status);
    detail = await res.json();
  } catch (e) {
    detail = demoPositions(account); // preview only
  }
  if (accountPanel.dataset.account !== account) return; // another dot was clicked meanwhile
  renderAccountDetail(account, detail);
  accountPanel.classList.add('open');
}

function closeAccountPanel() {
  delete accountPanel.dataset.account;
  accountPanel.classList.remove('open');
  statusDots.querySelectorAll('.status-dot').forEach((d) => d.classList.remove('selected'));
}

function toggleAccountPanel(account) {
  if (accountPanel.dataset.account === account) closeAccountPanel();
  else openAccountPanel(account);
}

document.addEventListener('keydown', (e) => { if (e.key === 'Escape') { closeAccountPanel(); closeRiskPanel(); } });

// ============================================================================
// Risk
//   GET /accounts/risk -> [{ account, daily_loss_pct, daily_loss_limit_pct,
//                            max_dd_pct, max_dd_limit_pct }]   (positive %)
//   Every connected account is listed, position or not. Daily loss resets at
//   the start of each trading day; max drawdown is lifetime.
//   Level per meter: excellent < CAUTION_AT × limit ≤ caution < limit ≤ violated
//   Account dot = its worst meter. Portfolio dot = worst account.
//   Violated is a safeguard only — the server blocks orders that would breach.
// ============================================================================

const CAUTION_AT = 0.7;       // caution once 70% of a limit is used
const RISK_POLL_MS = 5000;

const riskDot = $('risk-dot');
const riskPanel = $('risk-panel');
const RISK_ORDER = ['excellent', 'caution', 'violated'];

function riskLevel(value, limit) {
  const v = Math.abs(Number(value));
  if (v >= limit) return 'violated';
  if (v >= limit * CAUTION_AT) return 'caution';
  return 'excellent';
}

function worstLevel(levels) {
  return RISK_ORDER[Math.max(0, ...levels.map((l) => RISK_ORDER.indexOf(l)))];
}

function renderMeter(meter, value, limit) {
  const v = Math.abs(Number(value));
  const level = riskLevel(v, limit);
  meter.className = 'risk-meter ' + level;
  meter.querySelector('.risk-meter-value').textContent = `${v.toFixed(2)}% / ${Number(limit).toFixed(2)}%`;
  meter.querySelector('.risk-meter-fill').style.width = Math.min(100, (v / limit) * 100) + '%';
  return level;
}

const RISK_LABEL = { excellent: 'Excellent Risk', caution: 'Caution Risk', violated: 'Violated Risk' };

function renderPortfolioRisk() {
  const levels = accountRisk.map(accountLevel);
  const state = worstLevel(levels);

  riskDot.className = 'risk-dot ' + state + (riskPanel.classList.contains('open') ? ' selected' : '');
  riskDot.dataset.label = RISK_LABEL[state];
  riskDot.setAttribute('aria-label', RISK_LABEL[state]);
  $('risk-state').textContent = RISK_LABEL[state];
  $('risk-state').className = state;

  const counts = $('risk-counts');
  counts.innerHTML = '';
  for (const l of RISK_ORDER) {
    const chip = document.createElement('div');
    chip.className = 'risk-count ' + l;
    chip.append(span('risk-count-n', String(levels.filter((x) => x === l).length)));
    chip.dataset.label = l.charAt(0).toUpperCase() + l.slice(1);
    chip.setAttribute('aria-label', chip.dataset.label);
    counts.appendChild(chip);
  }

  // Accounts not at excellent, worst first, with the meter that put them there.
  const watch = $('risk-watch');
  watch.innerHTML = '';
  const flagged = accountRisk
    .map((a) => {
      const d = Math.abs(a.daily_loss_pct) / a.daily_loss_limit_pct;
      const m = Math.abs(a.max_dd_pct) / a.max_dd_limit_pct;
      return d >= m
        ? { a, level: accountLevel(a), use: d, text: `Daily ${Math.abs(a.daily_loss_pct).toFixed(2)}% / ${a.daily_loss_limit_pct.toFixed(2)}%` }
        : { a, level: accountLevel(a), use: m, text: `Max DD ${Math.abs(a.max_dd_pct).toFixed(2)}% / ${a.max_dd_limit_pct.toFixed(2)}%` };
    })
    .filter((f) => f.level !== 'excellent')
    .sort((x, y) => y.use - x.use);

  if (!flagged.length) {
    const p = document.createElement('p');
    p.className = 'notice';
    p.textContent = `All ${accountRisk.length} accounts well inside their limits.`;
    watch.appendChild(p);
    return;
  }
  for (const f of flagged) {
    const row = document.createElement('button');
    row.type = 'button';
    row.className = 'risk-watch-row ' + f.level;
    row.textContent = f.a.account;
    row.title = f.text;
    row.addEventListener('click', (ev) => {
      ev.stopPropagation();
      closeRiskPanel();
      openAccountPanel(f.a.account);
    });
    watch.appendChild(row);
  }
}

// Preview only: every connected account, a couple of them near a limit.
function demoAccountRisk() {
  const daily = [0.22, 0.41, 0.84, 0.10, 0.35, 0.18, 0.95, 0.27, 0.05, 0.48, 0.31, 0.60];
  const maxdd = [1.10, 2.05, 1.80, 0.40, 3.62, 0.90, 2.40, 1.25, 0.30, 2.10, 1.55, 2.95];
  return Array.from({ length: DEMO_ACCOUNT_COUNT }, (_, i) => ({
    account: `Account ${i + 1}`,
    daily_loss_pct: daily[i % daily.length], daily_loss_limit_pct: 1.2,
    max_dd_pct: maxdd[i % maxdd.length], max_dd_limit_pct: 5.0,
  }));
}

async function pollRisk() {
  try {
    const res = await fetch('/accounts/risk');
    if (!res.ok) throw new Error(res.status);
    accountRisk = (await res.json()).map((a) => ({
      ...a,
      daily_loss_pct: Number(a.daily_loss_pct), daily_loss_limit_pct: Number(a.daily_loss_limit_pct),
      max_dd_pct: Number(a.max_dd_pct), max_dd_limit_pct: Number(a.max_dd_limit_pct),
    }));
  } catch (err) {
    if (!accountRisk.length) accountRisk = demoAccountRisk();
  }
  renderStatusDots();
  renderPortfolioRisk();
}

function closeRiskPanel() {
  riskPanel.classList.remove('open');
  riskDot.classList.remove('selected');
}

riskDot.addEventListener('click', (e) => {
  e.stopPropagation();
  const open = !riskPanel.classList.contains('open');
  riskPanel.classList.toggle('open', open);
  riskDot.classList.toggle('selected', open);
});
// Click anywhere outside a panel (and its dot) to close it — no close button.
document.addEventListener('click', (e) => {
  if (!accountPanel.contains(e.target) && !e.target.closest('.status-dot')) closeAccountPanel();
  if (!riskPanel.contains(e.target) && !riskDot.contains(e.target)) closeRiskPanel();
});

pollRisk();
setInterval(pollRisk, RISK_POLL_MS);

// ============================================================================
// Intent rail — switches the card between Order / Ideas / Modify / Close
// ============================================================================

const rail = $('rail');
const railLabels = $('rail-labels');
const panelIds = { order: 'panel-order', ideas: 'panel-ideas', modify: 'panel-modify', close: 'panel-close' };
// Rail colour per panel — pulled from the theme variables in style.css.
const railTint = {
  order:  'var(--accent)',
  ideas:  'var(--idea)',
  modify: 'var(--warn)',
  close:  'var(--sell)',
};

function selectIntent(name) {
  Object.values(panelIds).forEach((id) => $(id).classList.remove('active'));
  $(panelIds[name]).classList.add('active');
  railLabels.querySelectorAll('button').forEach((b) =>
    b.classList.toggle('current', b.dataset.intent === name));
  card.style.setProperty('--rail-tint', railTint[name]);
  rail.classList.remove('open');
}

rail.addEventListener('click', (e) => {
  if (!e.target.closest('.rail-labels')) rail.classList.toggle('open');
});
railLabels.querySelectorAll('button').forEach((btn) =>
  btn.addEventListener('click', () => selectIntent(btn.dataset.intent)));
document.addEventListener('click', (e) => {
  if (!rail.contains(e.target)) rail.classList.remove('open');
});

// ============================================================================
// Modify / Close — both act on every position belonging to one Idea
//   POST /ideas/{id}/stop   <- { stop_loss }  -> [{ account, ok, message }]
//   POST /ideas/{id}/close                    -> [{ account, ok, message }]
// ============================================================================

async function ideaAction(btn, busyText, url, body, resultsEl, okText) {
  const idleText = btn.textContent;
  btn.disabled = true;
  btn.textContent = busyText;
  resultsEl.innerHTML = '';
  try {
    const res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: body ? JSON.stringify(body) : undefined,
    });
    const rows = await res.json();
    for (const r of rows) {
      resultsEl.appendChild(resultRow(r.account, r.ok, r.ok ? okText : 'Failed', r.message));
    }
  } catch (e) {
    resultsEl.textContent = 'Request failed — server unreachable.';
  }
  btn.disabled = false;
  btn.textContent = idleText;
}

$('modify-btn').addEventListener('click', () => {
  const id = $('modify_idea').value;
  const stop = $('new_stop').value.trim();
  if (!id || !stop) return;
  ideaAction($('modify-btn'), 'Updating…', `/ideas/${id}/stop`, { stop_loss: stop }, $('modify-results'), 'Moved');
});

$('close-btn').addEventListener('click', () => {
  const id = $('close_idea').value;
  if (!id) return;
  ideaAction($('close-btn'), 'Closing…', `/ideas/${id}/close`, null, $('close-results'), 'Closed');
});

// ============================================================================
// Ideas — mirrors the `ideas` table
//   GET  /ideas  -> [{ id, symbol, side, setup, timeframe, htf_bias, planned_entry,
//                      stop_loss, take_profit, idea_state, created_at }]
//   POST /ideas  <- { symbol, side, setup, timeframe, htf_bias, planned_entry,
//                     stop_loss, take_profit }
//   timeframe: M | W | D | 4H | H1 | 15m
//   htf_bias:  engulfing | shooting_star | hammer | flag | flat | channel
//                -> { id }
// Without a server, one example idea is shown and new ones live in memory.
// ============================================================================

const ideaList = $('idea-list');
const ideaForm = $('idea-form');
const ideaCreate = $('idea-create');
const ideaSideToggle = $('idea-side-toggle');
let ideaSide = 'buy';

const SETUP_LABELS = { jcp: 'JCP', dipndot: 'Dip N Dot', testsetup: 'Test Setup' };
const BIAS_LABELS = { engulfing: 'Engulfing', shooting_star: 'Shooting Star', hammer: 'Hammer',
  flag: 'Flag', flat: 'Flat', channel: 'Channel' };

const EXAMPLE_IDEAS = [
  { id: 'demo-1', symbol: 'EURUSD', side: 'sell', setup: 'jcp', timeframe: 'D', htf_bias: 'shooting_star', planned_entry: '1.13900',
    stop_loss: '1.14730', take_profit: '1.12757', idea_state: 'planned' },
];

function setIdeaSide(value) {
  ideaSide = value;
  ideaSideToggle.querySelectorAll('button').forEach((b) =>
    b.classList.toggle('active', b.dataset.side === value));
}
ideaSideToggle.addEventListener('click', (e) => {
  const btn = e.target.closest('button');
  if (btn) setIdeaSide(btn.dataset.side);
});

function span(className, text) {
  const el = document.createElement('span');
  el.className = className;
  el.textContent = text;
  return el;
}

function level(label, value) {
  const el = document.createElement('div');
  el.appendChild(span('', label));
  el.appendChild(document.createTextNode(value));
  return el;
}

function ideaLabel(idea) {
  const sideText = idea.side === 'buy' ? 'Buy' : 'Sell';
  return `${idea.symbol} · ${sideText} · ${SETUP_LABELS[idea.setup] || idea.setup}`;
}

// Order / Modify / Close all pick from the same list. Closed or missed
// Ideas can't take new orders, so they're left out.
function refreshIdeaSelects() {
  const usable = ideas.filter((i) => ['planned', 'active'].includes(i.idea_state || 'planned'));
  document.querySelectorAll('.idea-select').forEach((select) => {
    const current = select.value;
    select.innerHTML = '';
    const placeholder = document.createElement('option');
    placeholder.value = '';
    placeholder.textContent = usable.length ? 'Select an idea' : 'No ideas yet — create one first';
    select.appendChild(placeholder);
    for (const idea of usable) {
      const opt = document.createElement('option');
      opt.value = idea.id;
      opt.textContent = ideaLabel(idea);
      select.appendChild(opt);
    }
    select.value = usable.some((i) => i.id === current) ? current : '';
  });
}

// Picking an Idea refreshes the pending price from its planned entry.
function prefillOrderFromIdea() {
  if (!pendingBox.checked) return;
  const idea = ideas.find((i) => i.id === $('order_idea').value);
  priceInput.value = idea ? idea.planned_entry : '';
}

$('order_idea').addEventListener('change', prefillOrderFromIdea);

function addIdea(idea) {
  ideas.push(idea);
  renderIdea(idea);
  refreshIdeaSelects();
}

function renderIdea(idea) {
  const el = document.createElement('div');
  el.className = 'idea';

  const head = document.createElement('div');
  head.className = 'idea-head';
  head.append(
    span('idea-symbol', idea.symbol),
    span('idea-side ' + idea.side, idea.side),
    span('idea-setup', SETUP_LABELS[idea.setup] || idea.setup),
    ...(idea.timeframe ? [span('idea-setup', idea.timeframe)] : []),
    ...(idea.htf_bias ? [span('idea-setup', BIAS_LABELS[idea.htf_bias] || idea.htf_bias)] : []),
    span('idea-state', idea.idea_state || 'planned'),
  );

  const levels = document.createElement('div');
  levels.className = 'idea-levels';
  levels.append(
    level('Entry', idea.planned_entry),
    level('Stop', idea.stop_loss),
    level('Target', idea.take_profit),
  );

  el.append(head, levels);
  ideaList.prepend(el); // newest on top
}

function showIdeaForm(show) {
  ideaForm.classList.toggle('active', show);
  ideaCreate.style.display = show ? 'none' : '';
  if (show) {
    $('idea_symbol').focus();
  } else {
    ideaForm.reset();
    setIdeaSide('buy');
  }
}

async function loadIdeas() {
  try {
    const res = await fetch('/ideas');
    if (!res.ok) throw new Error(res.status);
    (await res.json()).forEach(addIdea);
  } catch (e) {
    EXAMPLE_IDEAS.forEach(addIdea); // preview only
  }
  refreshIdeaSelects();
}

$('idea-new-btn').addEventListener('click', () => showIdeaForm(true));
$('idea-cancel').addEventListener('click', () => showIdeaForm(false));

ideaForm.addEventListener('submit', async (e) => {
  e.preventDefault();
  const idea = {
    symbol: $('idea_symbol').value.trim().toUpperCase(),
    side: ideaSide,
    setup: $('idea_setup').value,
    timeframe: $('idea_timeframe').value,
    htf_bias: $('idea_htf_bias').value,
    planned_entry: $('idea_entry').value.trim(),
    stop_loss: $('idea_sl').value.trim(),
    take_profit: $('idea_tp').value.trim(),
  };
  let id = null;
  try {
    const res = await fetch('/ideas', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(idea),
    });
    id = (await res.json()).id; // server returns the new row's UUID
  } catch (err) {
    // Preview mode — keep it on screen anyway.
  }
  addIdea({ ...idea, id: id || crypto.randomUUID(), idea_state: 'planned' });
  showIdeaForm(false);
});

loadIdeas();

// ============================================================================
// Light / dark theme — remembered between visits
// ============================================================================

const themeToggle = $('theme-toggle');

function applyTheme(theme) {
  document.documentElement.dataset.theme = theme;
  themeToggle.textContent = theme === 'dark' ? 'Light' : 'Dark';
  localStorage.setItem('oms-theme', theme);
}

themeToggle.addEventListener('click', () =>
  applyTheme(document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark'));

applyTheme(localStorage.getItem('oms-theme') || 'dark');
selectIntent('ideas');

pollStatus().then(startStatusPolling);
