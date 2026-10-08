// BlindHop Demo - Browser-Embedded Nym Client (+ opt-in local proxy)
//
// Connection mode:
//   - Default: the Nym Wasm client runs in the browser (Web Worker); the page
//     never contacts localhost.
//   - Opt-in ("Use my local BlindHop proxy"): connect to blindhop-proxy at
//     ws://127.0.0.1:9500 (started with --allowed-origin for this site).
// If the Nym client fails, querying stops: there is no silent fallback to a
// direct connection.

import { LatencyChart } from './chart.js';
import { NymBrowserClient } from './nym-client.js';

// ——— Configuration ———

const CONFIG = {
    proxyUrl: 'ws://127.0.0.1:9500',
    directTarget: 'wss://sys.turboflakes.io/asset-hub-paseo',
    queryInterval: 5000,
    proxyDetectTimeout: 2000,
    defaultExitAddress: 'DQ4uyTm1HyWmagAWtWcp3dzRvy7L6tjTgLECg4NiuEHN.HoGQui9XA7x6bFDTpifUDTvniWk3VNwCAeBQg1KCymyu@B6docD8mkjfBWh9vuYw63BbzDfgEzCxARokK4jWfqN3K', // Deployed exit service node address
    // Earlier defaults. A saved address equal to one of these was the default
    // at the time, not the user's own exit, so it's replaced by the current one.
    retiredExitAddresses: [
        'DQ4uyTm1HyWmagAWtWcp3dzRvy7L6tjTgLECg4NiuEHN.HoGQui9XA7x6bFDTpifUDTvniWk3VNwCAeBQg1KCymyu@ES962rrdsZNhE15dKYnkCtMSPJZZz2B411GUmpyyxkp7',
    ],
};

// Privacy modes mapped to slider positions
const MODES = ['none', 'fast', 'full'];
const MODE_INFO = {
    none: {
        emoji: '🔴',
        label: 'Direct — IP Exposed',
        route: 'Direct WebSocket (no mixnet)',
        hops: 0,
        cover: false,
        ipLabel: 'YOUR IP ADDRESS',
        ipExposed: true,
    },
    fast: {
        emoji: '🟡',
        label: '2-hop dVPN — IP Hidden',
        route: 'Nym dVPN (2 hops)',
        hops: 2,
        cover: false,
        ipLabel: 'NYM EXIT NODE IP',
        ipExposed: false,
    },
    full: {
        emoji: '🟢',
        label: '5-hop Mixnet — Metadata Private',
        route: 'Nym Mixnet (5 hops + cover traffic)',
        hops: 5,
        cover: true,
        ipLabel: 'NYM EXIT NODE IP',
        ipExposed: false,
    },
};

// ——— State ———

let currentMode = 'full';
let connectionMode = null; // 'native' | 'browser' | null
let queryTimer = null;
let proxyWs = null;
let directWs = null;
let nymClient = null;
let requestId = 1;
let pendingRequests = {};
// Native proxy control requests awaiting a reply (JSON-RPC ids).
let pendingModeSwitch = null; // { id, mode }
let pendingModeQuery = null;  // id of the blindhop_getMetrics sent on connect
let chart = null;

// Metrics per mode
let metrics = {
    none: { latencies: [], sent: 0, requests: 0, lastBlock: null, chain: null },
    fast: { latencies: [], sent: 0, requests: 0, lastBlock: null, chain: null },
    full: { latencies: [], sent: 0, requests: 0, lastBlock: null, chain: null },
};

// Requests unanswered after this long are treated as lost (matches the
// timeouts in nym-client.js and blindhop-proxy).
const REQUEST_TIMEOUT_MS = 60_000;

// Browsers throttle timers in hidden tabs (Chrome: about once a minute after
// 5 minutes hidden), which makes measured latencies meaningless. Polling
// pauses while the tab is hidden, and replies to requests that were in flight
// while it was hidden aren't counted as latency samples.
let lastHiddenAt = -Infinity;
document.addEventListener('visibilitychange', () => {
    if (document.hidden) lastHiddenAt = performance.now();
});

let directLatencies = [];
// Also query the RPC node directly, for the latency comparison. Off by
// default: it sends the same queries from the user's own IP address.
let compareDirect = false;

// ——— Auto-Detection ———

/**
 * Test if a WebSocket endpoint is reachable within a timeout.
 */
function testWebSocket(url, timeoutMs) {
    return new Promise((resolve, reject) => {
        const ws = new WebSocket(url);
        const timer = setTimeout(() => {
            ws.close();
            reject(new Error('Timeout'));
        }, timeoutMs);

        ws.onopen = () => {
            clearTimeout(timer);
            ws.close();
            resolve(true);
        };
        ws.onerror = () => {
            clearTimeout(timer);
            reject(new Error('Connection failed'));
        };
    });
}

/**
 * Detect whether native blindhop-proxy is running.
 * Returns 'native' if proxy found, 'browser' otherwise.
 */
/**
 * Use the native proxy only if the user opted in. Probing 127.0.0.1 from a
 * public page triggers the browser's "access other apps and services on this
 * device" prompt and can be used for fingerprinting, so by default the demo
 * never touches localhost and uses the in-browser Nym client.
 */
async function detectConnectionMode() {
    if (!useLocalProxy()) {
        return 'browser';
    }
    updateConnectionMode('detecting');
    try {
        await testWebSocket(CONFIG.proxyUrl, CONFIG.proxyDetectTimeout);
        console.log('[Detection] Native proxy found at', CONFIG.proxyUrl);
        return 'native';
    } catch {
        console.log('[Detection] Native proxy not found, using browser Nym client');
        return 'browser';
    }
}

// ——— Privacy Slider ———

function initSlider() {
    const slider = document.getElementById('privacy-slider');
    slider.addEventListener('input', () => {
        const val = parseInt(slider.value);
        const mode = MODES[val];

        // In browser mode, skip 'fast' (jump to 'full' instead)
        if (connectionMode === 'browser' && mode === 'fast') {
            slider.value = 2;
            setPrivacyMode('full');
            return;
        }

        setPrivacyMode(mode);
    });

    // Click on marks to jump
    document.querySelectorAll('.mark').forEach(mark => {
        mark.addEventListener('click', () => {
            const val = parseInt(mark.dataset.value);
            const mode = MODES[val];

            if (connectionMode === 'browser' && mode === 'fast') return;

            slider.value = val;
            setPrivacyMode(mode);
        });
    });

    setPrivacyMode('full');
}

/**
 * Request a privacy mode. With a native proxy, the new mode is shown only
 * after the proxy confirms it, so the page never claims a privacy level
 * the proxy isn't providing.
 */
function setPrivacyMode(mode) {
    if (connectionMode === 'native' && proxyWs && proxyWs.readyState === WebSocket.OPEN) {
        const id = sendToProxy('blindhop_setPrivacyMode', [mode]);
        pendingModeSwitch = { id, mode };
        document.getElementById('indicator-text').textContent =
            `Switching to ${MODE_INFO[mode].label}…`;
        return;
    }
    renderPrivacyMode(mode);
}

/** Show `mode` as the active privacy mode (UI only). */
function renderPrivacyMode(mode) {
    currentMode = mode;
    const info = MODE_INFO[mode];
    const slider = document.getElementById('privacy-slider');
    slider.value = MODES.indexOf(mode);

    // Update slider fill
    const fillPct = (MODES.indexOf(mode) / 2) * 100;
    document.getElementById('slider-fill').style.width = fillPct + '%';

    // Update indicator
    const indicator = document.getElementById('privacy-indicator');
    indicator.className = 'privacy-indicator mode-' + mode;
    document.getElementById('indicator-emoji').textContent = info.emoji;
    document.getElementById('indicator-text').textContent = info.label;

    // Update connection card
    document.getElementById('route-info').textContent = info.route;
    document.getElementById('hop-count').textContent =
        info.hops === 0 ? 'None (direct)' : `${info.hops} hops`;
    document.getElementById('cover-status').textContent =
        info.cover ? '✓ Active (Loopix)' : '✗ Disabled';

    const ipRow = document.getElementById('ip-row');
    const ipValue = document.getElementById('ip-visibility');
    ipRow.className = 'stat-row ip-row ' + (info.ipExposed ? 'exposed' : 'hidden');
    ipValue.textContent = info.ipLabel;

    syncDirectConnection();
}

/**
 * Handle replies to the proxy control requests sent by this page.
 * @returns {boolean} true if `data` was such a reply.
 */
function handleProxyControlReply(data) {
    let response;
    try {
        response = JSON.parse(data);
    } catch {
        return false;
    }

    if (pendingModeSwitch && response.id === pendingModeSwitch.id) {
        const requested = pendingModeSwitch.mode;
        pendingModeSwitch = null;
        if (response.result && MODE_INFO[response.result.mode_id]) {
            renderPrivacyMode(response.result.mode_id);
        } else {
            // Show the mode the proxy is actually in, not the one requested.
            const actual = response.error?.data?.mode_id;
            renderPrivacyMode(MODE_INFO[actual] ? actual : currentMode);
            const message = response.error?.message || 'unknown error';
            console.error(`[Proxy] Switch to ${requested} failed:`, message);
            const nymStatus = document.getElementById('nym-status');
            if (nymStatus) nymStatus.textContent = `Mode switch failed: ${message}`;
        }
        return true;
    }

    if (pendingModeQuery !== null && response.id === pendingModeQuery) {
        pendingModeQuery = null;
        const actual = response.result?.mode_id;
        if (MODE_INFO[actual]) renderPrivacyMode(actual);
        return true;
    }

    return false;
}

function updateSliderForMode(connMode) {
    const fastMark = document.querySelector('.mark[data-value="1"]');
    if (connMode === 'browser') {
        fastMark.classList.add('disabled');
        fastMark.title = 'Install native proxy for 2-hop Fast mode';
    } else {
        fastMark.classList.remove('disabled');
        fastMark.title = '';
    }
}

// ——— Connection Mode UI ———

function updateConnectionMode(mode) {
    const badge = document.getElementById('connection-mode-badge');
    if (!badge) return;

    switch (mode) {
        case 'detecting':
            badge.className = 'connection-mode-badge detecting';
            badge.textContent = '🔍 Detecting...';
            break;
        case 'native':
            badge.className = 'connection-mode-badge native';
            badge.textContent = '🖥️ Native Proxy';
            break;
        case 'browser':
            badge.className = 'connection-mode-badge browser';
            badge.textContent = '🌐 Browser (Nym Wasm)';
            break;
    }
}

// ——— WebSocket Connections ———

function connectProxy() {
    return new Promise((resolve, reject) => {
        try {
            proxyWs = new WebSocket(CONFIG.proxyUrl);
            proxyWs.onopen = () => {
                console.log('[Proxy] Connected');
                resolve();
            };
            proxyWs.onmessage = (event) => {
                if (!handleProxyControlReply(event.data)) handleResponse('proxy', event.data);
            };
            proxyWs.onerror = (e) => {
                console.error('[Proxy] Error:', e);
                reject(e);
            };
            proxyWs.onclose = () => {
                console.log('[Proxy] Disconnected');
                updateStatus('disconnected');
            };
        } catch (e) {
            reject(e);
        }
    });
}

function connectDirect() {
    return new Promise((resolve, reject) => {
        try {
            const ws = new WebSocket(CONFIG.directTarget);
            directWs = ws;
            ws.onopen = () => {
                console.log('[Direct] Connected');
                resolve();
            };
            ws.onmessage = (event) => handleResponse('direct', event.data);
            ws.onerror = (e) => {
                console.error('[Direct] Error:', e);
                reject(e);
            };
            ws.onclose = () => {
                console.log('[Direct] Disconnected');
                if (directWs === ws) directWs = null;
            };
        } catch (e) {
            reject(e);
        }
    });
}

async function connectNymBrowser() {
    const exitInput = document.getElementById('exit-address');
    const exitAddress = exitInput?.value.trim() || CONFIG.defaultExitAddress;

    if (!exitAddress) {
        throw new Error('Exit service Nym address is required. Enter it in the Exit Address field.');
    }

    nymClient = new NymBrowserClient();

    nymClient.onStatusChange((status, message) => {
        updateStatus(status === 'connected' ? 'connected' : 'connecting');
        const nymStatus = document.getElementById('nym-status');
        if (nymStatus) nymStatus.textContent = message;
    });

    nymClient.onResponse((jsonRpcString) => {
        handleResponse('nym', jsonRpcString);
    });

    await nymClient.connect(exitAddress);
}

// ——— JSON-RPC ———

function sendRpc(ws, method, params, source) {
    const id = requestId++;
    const request = JSON.stringify({
        jsonrpc: '2.0',
        id,
        method,
        params: params || [],
    });

    // Credit the reply to the mode it was sent in, even if the mode changes.
    const mode = source === 'direct' ? 'none' : currentMode;
    pendingRequests[`${source}-${id}`] = {
        sentAt: performance.now(),
        source,
        method,
        mode,
    };
    metrics[mode].sent++;

    ws.send(request);
    return id;
}

function sendToProxy(method, params) {
    if (proxyWs && proxyWs.readyState === WebSocket.OPEN) {
        const id = requestId++;
        const msg = JSON.stringify({ jsonrpc: '2.0', id, method, params: params || [] });
        proxyWs.send(msg);
        return id;
    }
    return null;
}

async function sendViaNym(method, params) {
    if (!nymClient || !nymClient.isConnected) return null;

    const id = requestId++;
    const request = JSON.stringify({
        jsonrpc: '2.0',
        id,
        method,
        params: params || [],
    });

    pendingRequests[`nym-${id}`] = {
        sentAt: performance.now(),
        source: 'nym',
        method,
        mode: currentMode,
    };
    metrics[currentMode].sent++;

    await nymClient.sendRequest(request);
    return id;
}

function handleResponse(source, data) {
    try {
        const response = typeof data === 'string' ? JSON.parse(data) : data;
        const key = `${source}-${response.id}`;
        const pending = pendingRequests[key];

        if (!pending) return;

        const latency = performance.now() - pending.sentAt;
        delete pendingRequests[key];

        const modeMetrics = metrics[pending.mode];
        modeMetrics.requests++;

        // A request in flight while the tab was hidden has a throttled,
        // meaningless timing: count the reply, but don't chart it.
        const timingValid = !document.hidden && pending.sentAt > lastHiddenAt;
        if (timingValid) {
            if (source === 'direct') {
                directLatencies.push(latency);
                if (directLatencies.length > 100) directLatencies.shift();
            }
            modeMetrics.latencies.push(latency);
            if (modeMetrics.latencies.length > 100) modeMetrics.latencies.shift();
            if (chart) chart.addPoint(pending.mode, latency);
        }

        // Parse chain data
        if (pending.method === 'chain_getHeader' && response.result) {
            const blockNum = parseInt(response.result.number, 16);
            if (source === 'proxy' || source === 'nym') {
                modeMetrics.lastBlock = blockNum;
            } else {
                metrics.none.lastBlock = blockNum;
            }
        }

        if (pending.method === 'system_chain' && response.result) {
            if (source === 'proxy' || source === 'nym') {
                modeMetrics.chain = response.result;
            } else {
                metrics.none.chain = response.result;
            }
        }

        updateUI();
    } catch (e) {
        console.error(`[${source}] Parse error:`, e);
    }
}

// ——— Query Loop ———

/**
 * Open or close the direct connection to the RPC node so it exists only
 * while it's needed: in Direct mode, or when the user opted into the
 * latency comparison. Otherwise the node never sees the user's IP.
 */
function syncDirectConnection() {
    const running = queryTimer !== null;
    const needed = running && (currentMode === 'none' || compareDirect);
    if (needed && !directWs) {
        connectDirect().catch((e) => console.warn('[Direct] Could not connect:', e?.message || e));
    } else if (!needed && directWs) {
        directWs.close();
        directWs = null;
    }
}

function toggleCompareDirect(enabled) {
    compareDirect = enabled;
    syncDirectConnection();
}

/** Forget requests unanswered for longer than REQUEST_TIMEOUT_MS (lost). */
function dropLostRequests() {
    const cutoff = performance.now() - REQUEST_TIMEOUT_MS;
    for (const [key, pending] of Object.entries(pendingRequests)) {
        if (pending.sentAt < cutoff) delete pendingRequests[key];
    }
}

function queryAll() {
    dropLostRequests();
    // Don't poll from a hidden tab (see lastHiddenAt).
    if (document.hidden) return;

    if (currentMode === 'none') {
        // Direct mode — query Substrate directly
        if (directWs && directWs.readyState === WebSocket.OPEN) {
            sendRpc(directWs, 'chain_getHeader', [], 'direct');
        }
    } else if (connectionMode === 'native') {
        // Native proxy mode
        if (proxyWs && proxyWs.readyState === WebSocket.OPEN) {
            sendRpc(proxyWs, 'chain_getHeader', [], 'proxy');
        }
    } else if (connectionMode === 'browser') {
        // Browser Nym mode
        sendViaNym('chain_getHeader', []);
    }

    // Direct comparison only if the user opted in (it reveals their IP).
    if (compareDirect && currentMode !== 'none' && directWs && directWs.readyState === WebSocket.OPEN) {
        sendRpc(directWs, 'chain_getHeader', [], 'direct');
    }
}

async function startQuerying() {
    const startBtn = document.getElementById('btn-start');
    const stopBtn = document.getElementById('btn-stop');
    startBtn.disabled = true;
    startBtn.textContent = '⏳ Detecting...';
    startBtn.classList.add('connecting');
    updateStatus('connecting');

    try {
        // Step 1: Auto-detect connection mode
        connectionMode = await detectConnectionMode();
        updateConnectionMode(connectionMode);
        updateSliderForMode(connectionMode);

        // Step 2: Connect direct only when needed: Direct mode, or the
        // user opted into the comparison. It reveals the user's IP.
        if (currentMode === 'none' || compareDirect) {
            startBtn.textContent = '⏳ Connecting direct...';
            try {
                await connectDirect();
                sendRpc(directWs, 'system_chain', [], 'direct');
            } catch (e) {
                console.warn('[Direct] Could not connect:', e?.message || e);
            }
        }

        // Step 3: Connect via detected mode
        if (connectionMode === 'native') {
            startBtn.textContent = '⏳ Connecting to proxy...';
            await connectProxy();
            // Show the proxy's actual mode (it may have been started in another mode).
            pendingModeQuery = sendToProxy('blindhop_getMetrics', []);
            sendRpc(proxyWs, 'system_chain', [], 'proxy');
            updateStatus('connected');
        } else {
            startBtn.textContent = '⏳ Starting Nym client...';
            try {
                await connectNymBrowser();
                await sendViaNym('system_chain', []);
                updateStatus('connected');
            } catch (e) {
                console.error('[NymBrowser] Connection failed:', e);
                updateStatus('error');
                const nymStatus = document.getElementById('nym-status');
                if (nymStatus) nymStatus.textContent = `Error: ${e.message}`;

                // Fail closed: never fall back to direct, which would expose
                // the user's IP without them choosing it.
                if (directWs) { directWs.close(); directWs = null; }
                startBtn.disabled = false;
                startBtn.textContent = '▶ Start Querying';
                startBtn.classList.remove('connecting');
                return;
            }
        }

        startBtn.textContent = '● Running';
        startBtn.classList.remove('connecting');
        startBtn.classList.add('running');
        stopBtn.disabled = false;

        queryTimer = setInterval(queryAll, CONFIG.queryInterval);
        syncDirectConnection();
        queryAll();
    } catch (e) {
        startBtn.disabled = false;
        startBtn.textContent = '▶ Start Querying';
        startBtn.classList.remove('connecting');
        updateStatus('error');
        console.error('Connection failed:', e);
    }
}

function stopQuerying() {
    if (queryTimer) clearInterval(queryTimer);
    queryTimer = null;

    if (proxyWs) { proxyWs.close(); proxyWs = null; }
    if (directWs) { directWs.close(); directWs = null; }
    if (nymClient) { nymClient.disconnect(); nymClient = null; }

    connectionMode = null;

    document.getElementById('btn-start').disabled = false;
    document.getElementById('btn-start').textContent = '▶ Start Querying';
    document.getElementById('btn-start').classList.remove('running');
    document.getElementById('btn-stop').disabled = true;
    updateStatus('disconnected');
    updateConnectionMode('detecting');

    // Reset fast mark
    const fastMark = document.querySelector('.mark[data-value="1"]');
    if (fastMark) {
        fastMark.classList.remove('disabled');
        fastMark.title = '';
    }
}

function updateInterval() {
    CONFIG.queryInterval = parseInt(document.getElementById('interval-select').value);
    if (queryTimer) {
        clearInterval(queryTimer);
        queryTimer = setInterval(queryAll, CONFIG.queryInterval);
    }
}

// ——— UI Updates ———

function percentile(arr, p) {
    if (arr.length === 0) return 0;
    const sorted = [...arr].sort((a, b) => a - b);
    const idx = Math.ceil(sorted.length * p / 100) - 1;
    return sorted[Math.max(0, idx)];
}

function formatLatency(ms) {
    if (ms === 0) return '—';
    if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
    return `${ms.toFixed(0)}ms`;
}

function updateStatus(status) {
    const badge = document.getElementById('status-badge');
    badge.className = 'status-badge ' + status;
    badge.textContent = status === 'connected' ? 'Connected'
        : status === 'connecting' ? 'Connecting...'
            : status === 'error' ? 'Connection Error'
                : 'Disconnected';
}

function updateUI() {
    const m = metrics[currentMode];
    const p50 = percentile(m.latencies, 50);
    const p95 = percentile(m.latencies, 95);

    const chain = m.chain || metrics.none.chain || '—';
    const block = m.lastBlock || metrics.none.lastBlock;

    document.getElementById('chain-name').textContent = chain;
    document.getElementById('latest-block').textContent =
        block ? `#${block.toLocaleString()}` : '—';

    document.getElementById('latency-p50').textContent = formatLatency(p50);
    document.getElementById('latency-p95').textContent = formatLatency(p95);
    document.getElementById('messages-sent').textContent = m.sent;
    document.getElementById('messages-recv').textContent = m.requests;

    const overheadEl = document.getElementById('overhead-value');
    if (!compareDirect) {
        // Measured only when the user opts into the direct comparison.
        overheadEl.textContent = 'Off';
        overheadEl.classList.add('muted');
    } else {
        overheadEl.classList.remove('muted');
        const directP50 = percentile(directLatencies, 50);
        if (directP50 > 0 && p50 > 0) {
            const overhead = p50 - directP50;
            overheadEl.textContent =
                overhead >= 0 ? `+${formatLatency(overhead)}` : formatLatency(overhead);
        }
    }
}

// ——— Exit Address Persistence ———

// Per-browser preferences. Storage can be unavailable (private windows,
// blocked site data), so failures fall back to the defaults.
function loadPref(key) {
    try {
        return localStorage.getItem(key);
    } catch {
        return null;
    }
}

function savePref(key, value) {
    try {
        localStorage.setItem(key, value);
    } catch {
        // Not persisted; the page still works.
    }
}

function initExitAddress() {
    const input = document.getElementById('exit-address');
    if (!input) return;

    // A saved address (e.g. a user's own exit) wins; otherwise the default.
    let saved = loadPref('blindhop_exit_address');
    if (CONFIG.retiredExitAddresses.includes(saved)) {
        saved = null;
        savePref('blindhop_exit_address', '');
    }
    input.value = saved || CONFIG.defaultExitAddress;

    input.addEventListener('change', () => {
        savePref('blindhop_exit_address', input.value.trim());
    });
}

function useLocalProxy() {
    return document.getElementById('use-local-proxy')?.checked === true;
}

function initLocalProxyOption() {
    const checkbox = document.getElementById('use-local-proxy');
    if (!checkbox) return;
    checkbox.checked = loadPref('blindhop_use_local_proxy') === '1';
    checkbox.addEventListener('change', () => {
        savePref('blindhop_use_local_proxy', checkbox.checked ? '1' : '0');
    });
}

// ——— Init ———

// Make functions available to onclick handlers in HTML
window.startQuerying = startQuerying;
window.stopQuerying = stopQuerying;
window.updateInterval = updateInterval;
window.toggleCompareDirect = toggleCompareDirect;

document.addEventListener('DOMContentLoaded', () => {
    chart = new LatencyChart('latency-chart');
    initSlider();
    initExitAddress();
    initLocalProxyOption();
});
