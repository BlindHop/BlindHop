// BlindHop Demo - Browser-Embedded Nym Client + Native Proxy Fallback
//
// Auto-detects connection mode:
//   1. Try native blindhop-proxy at ws://localhost:9500
//   2. If not found, fall back to browser Nym Wasm client
//
// In browser mode, the Nym SDK runs directly in a Web Worker — no local setup needed.

import { LatencyChart } from './chart.js';
import { NymBrowserClient } from './nym-client.js';

// ——— Configuration ———

const CONFIG = {
    proxyUrl: 'ws://127.0.0.1:9500',
    directTarget: 'wss://sys.turboflakes.io/asset-hub-paseo',
    queryInterval: 5000,
    proxyDetectTimeout: 2000,
    defaultExitAddress: '', // Set after deploying exit service
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
    none: { latencies: [], requests: 0, lastBlock: null, chain: null },
    fast: { latencies: [], requests: 0, lastBlock: null, chain: null },
    full: { latencies: [], requests: 0, lastBlock: null, chain: null },
};

let directLatencies = [];

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
async function detectConnectionMode() {
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
            directWs = new WebSocket(CONFIG.directTarget);
            directWs.onopen = () => {
                console.log('[Direct] Connected');
                resolve();
            };
            directWs.onmessage = (event) => handleResponse('direct', event.data);
            directWs.onerror = (e) => {
                console.error('[Direct] Error:', e);
                reject(e);
            };
            directWs.onclose = () => console.log('[Direct] Disconnected');
        } catch (e) {
            reject(e);
        }
    });
}

async function connectNymBrowser() {
    const exitInput = document.getElementById('exit-address');
    const exitAddress = exitInput ? exitInput.value.trim() : CONFIG.defaultExitAddress;

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

    pendingRequests[`${source}-${id}`] = {
        sentAt: performance.now(),
        source,
        method,
    };

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
    };

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

        const modeMetrics = metrics[currentMode];

        if (source === 'proxy' || source === 'nym') {
            modeMetrics.latencies.push(latency);
            modeMetrics.requests++;
            if (modeMetrics.latencies.length > 100) modeMetrics.latencies.shift();
            if (chart) chart.addPoint(currentMode, latency);
        } else if (source === 'direct') {
            directLatencies.push(latency);
            if (directLatencies.length > 100) directLatencies.shift();
            metrics.none.latencies.push(latency);
            metrics.none.requests++;
            if (chart) chart.addPoint('none', latency);
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

function queryAll() {
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

    // Always query direct for overhead comparison (if not in none mode)
    if (currentMode !== 'none' && directWs && directWs.readyState === WebSocket.OPEN) {
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

        // Step 2: Connect direct (for baseline)
        startBtn.textContent = '⏳ Connecting direct...';
        try {
            await connectDirect();
            sendRpc(directWs, 'system_chain', [], 'direct');
        } catch (e) {
            console.warn('[Direct] Could not connect for baseline:', e.message);
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

                // Fall back to direct-only
                if (!directWs || directWs.readyState !== WebSocket.OPEN) {
                    startBtn.disabled = false;
                    startBtn.textContent = '▶ Start Querying';
                    startBtn.classList.remove('connecting');
                    return;
                }
                setPrivacyMode('none');
            }
        }

        startBtn.textContent = '● Running';
        startBtn.classList.remove('connecting');
        stopBtn.disabled = false;

        queryAll();
        queryTimer = setInterval(queryAll, CONFIG.queryInterval);
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
    document.getElementById('messages-sent').textContent = m.requests;
    document.getElementById('messages-recv').textContent = m.requests;

    const directP50 = percentile(directLatencies, 50);
    if (directP50 > 0 && p50 > 0) {
        const overhead = p50 - directP50;
        document.getElementById('overhead-value').textContent =
            overhead >= 0 ? `+${formatLatency(overhead)}` : formatLatency(overhead);
    }
}

// ——— Exit Address Persistence ———

function initExitAddress() {
    const input = document.getElementById('exit-address');
    if (!input) return;

    const saved = localStorage.getItem('blindhop_exit_address');
    if (saved) {
        input.value = saved;
    }

    input.addEventListener('change', () => {
        localStorage.setItem('blindhop_exit_address', input.value.trim());
    });
}

// ——— Init ———

// Make functions available to onclick handlers in HTML
window.startQuerying = startQuerying;
window.stopQuerying = stopQuerying;
window.updateInterval = updateInterval;

document.addEventListener('DOMContentLoaded', () => {
    chart = new LatencyChart('latency-chart');
    initSlider();
    initExitAddress();
});
