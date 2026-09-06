// BlindHop MVP Demo — Client-side Logic
// Queries a Substrate chain via WebSocket in two modes:
// 1. Native: direct WebSocket to the full node
// 2. BlindHop: WebSocket through the local blindhop-proxy (Sphinx relay path)

const CONFIG = {
    nativeTarget: 'wss://sys.turboflakes.io/asset-hub-paseo',
    blindhopProxy: 'ws://127.0.0.1:9500',
    queryInterval: 5000,
};

// State
let mode = 'native';
let queryTimer = null;
let nativeWs = null;
let blindhopWs = null;
let nativeMetrics = { latencies: [], requests: 0, lastBlock: null, chain: null };
let blindhopMetrics = { latencies: [], requests: 0, lastBlock: null, chain: null };
let requestId = 1;
let pendingRequests = {};

// Chart data
const chartData = { native: [], blindhop: [] };
const MAX_CHART_POINTS = 50;

// ——— Mode Toggle ———

function setMode(newMode) {
    mode = newMode;
    document.querySelectorAll('.toggle-btn').forEach(b => b.classList.remove('active'));
    document.getElementById(`btn-${newMode}`).classList.add('active');

    const panels = document.getElementById('panels');
    const nativePanel = document.getElementById('panel-native');
    const bhPanel = document.getElementById('panel-blindhop');
    const overheadBar = document.getElementById('overhead-bar');
    const chartContainer = document.getElementById('chart-container');

    if (newMode === 'native') {
        nativePanel.style.display = 'block';
        bhPanel.style.display = 'none';
        panels.classList.remove('side-by-side');
        overheadBar.style.display = 'none';
        chartContainer.style.display = 'none';
    } else if (newMode === 'blindhop') {
        nativePanel.style.display = 'none';
        bhPanel.style.display = 'block';
        panels.classList.remove('side-by-side');
        overheadBar.style.display = 'none';
        chartContainer.style.display = 'none';
    } else {
        nativePanel.style.display = 'block';
        bhPanel.style.display = 'block';
        panels.classList.add('side-by-side');
        overheadBar.style.display = 'flex';
        chartContainer.style.display = 'block';
    }
}

// ——— WebSocket Connections ———

function connectNative() {
    return new Promise((resolve, reject) => {
        try {
            nativeWs = new WebSocket(CONFIG.nativeTarget);
            nativeWs.onopen = () => {
                console.log('[Native] Connected');
                resolve();
            };
            nativeWs.onmessage = (event) => handleResponse('native', event.data);
            nativeWs.onerror = (e) => {
                console.error('[Native] Error:', e);
                reject(e);
            };
            nativeWs.onclose = () => console.log('[Native] Disconnected');
        } catch (e) {
            reject(e);
        }
    });
}

function connectBlindHop() {
    return new Promise((resolve, reject) => {
        try {
            blindhopWs = new WebSocket(CONFIG.blindhopProxy);
            blindhopWs.onopen = () => {
                console.log('[BlindHop] Connected');
                resolve();
            };
            blindhopWs.onmessage = (event) => handleResponse('blindhop', event.data);
            blindhopWs.onerror = (e) => {
                console.error('[BlindHop] Error:', e);
                reject(e);
            };
            blindhopWs.onclose = () => console.log('[BlindHop] Disconnected');
        } catch (e) {
            reject(e);
        }
    });
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

function handleResponse(source, data) {
    try {
        const response = JSON.parse(data);
        const key = `${source}-${response.id}`;
        const pending = pendingRequests[key];

        if (pending) {
            const latency = performance.now() - pending.sentAt;
            delete pendingRequests[key];

            const metrics = source === 'native' ? nativeMetrics : blindhopMetrics;
            metrics.latencies.push(latency);
            metrics.requests++;

            // Keep only last 100 latencies
            if (metrics.latencies.length > 100) metrics.latencies.shift();

            // Update chart data
            chartData[source].push({ time: Date.now(), latency });
            if (chartData[source].length > MAX_CHART_POINTS) chartData[source].shift();

            // Parse block number from chain_getHeader response
            if (pending.method === 'chain_getHeader' && response.result) {
                const blockNum = parseInt(response.result.number, 16);
                metrics.lastBlock = blockNum;
            }

            // Parse chain name
            if (pending.method === 'system_chain' && response.result) {
                metrics.chain = response.result;
            }

            updateUI();
        }
    } catch (e) {
        console.error(`[${source}] Parse error:`, e);
    }
}

// ——— Querying ———

function queryAll() {
    if (nativeWs && nativeWs.readyState === WebSocket.OPEN) {
        sendRpc(nativeWs, 'chain_getHeader', [], 'native');
    }
    if (blindhopWs && blindhopWs.readyState === WebSocket.OPEN) {
        sendRpc(blindhopWs, 'chain_getHeader', [], 'blindhop');
    }
}

async function startQuerying() {
    const startBtn = document.getElementById('btn-start');
    const stopBtn = document.getElementById('btn-stop');
    startBtn.disabled = true;
    startBtn.textContent = '⏳ Connecting...';

    try {
        // Connect based on mode
        if (mode === 'native' || mode === 'both') {
            await connectNative();
            sendRpc(nativeWs, 'system_chain', [], 'native');
        }
        if (mode === 'blindhop' || mode === 'both') {
            try {
                await connectBlindHop();
                sendRpc(blindhopWs, 'system_chain', [], 'blindhop');
            } catch (e) {
                console.warn('[BlindHop] Proxy not running. Start it with: scripts/run_demo.sh');
                document.getElementById('bh-chain').textContent = '⚠ Proxy not running';
            }
        }

        startBtn.textContent = '● Running';
        stopBtn.disabled = false;

        // Start periodic queries
        queryAll();
        queryTimer = setInterval(queryAll, CONFIG.queryInterval);
    } catch (e) {
        startBtn.disabled = false;
        startBtn.textContent = '▶ Start Querying';
        console.error('Connection failed:', e);
    }
}

function stopQuerying() {
    if (queryTimer) clearInterval(queryTimer);
    queryTimer = null;

    if (nativeWs) { nativeWs.close(); nativeWs = null; }
    if (blindhopWs) { blindhopWs.close(); blindhopWs = null; }

    document.getElementById('btn-start').disabled = false;
    document.getElementById('btn-start').textContent = '▶ Start Querying';
    document.getElementById('btn-stop').disabled = true;
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

function updateUI() {
    // Native panel
    const np50 = percentile(nativeMetrics.latencies, 50);
    document.getElementById('native-chain').textContent = nativeMetrics.chain || '—';
    document.getElementById('native-block').textContent = nativeMetrics.lastBlock
        ? `#${nativeMetrics.lastBlock.toLocaleString()}`
        : '—';
    document.getElementById('native-latency').textContent = np50 > 0 ? `${np50.toFixed(0)}ms` : '—';
    document.getElementById('native-requests').textContent = nativeMetrics.requests;

    // BlindHop panel
    const bp50 = percentile(blindhopMetrics.latencies, 50);
    document.getElementById('bh-chain').textContent = blindhopMetrics.chain || '—';
    document.getElementById('bh-block').textContent = blindhopMetrics.lastBlock
        ? `#${blindhopMetrics.lastBlock.toLocaleString()}`
        : '—';
    document.getElementById('bh-latency').textContent = bp50 > 0 ? `${bp50.toFixed(0)}ms` : '—';
    document.getElementById('bh-requests').textContent = blindhopMetrics.requests;

    // Overhead
    if (np50 > 0 && bp50 > 0) {
        const diff = bp50 - np50;
        document.getElementById('overhead-value').textContent = `+${diff.toFixed(0)}ms`;
        document.getElementById('overhead-detail').textContent =
            `Native p50: ${np50.toFixed(0)}ms · BlindHop p50: ${bp50.toFixed(0)}ms`;
    }

    // Redraw chart
    if (mode === 'both') drawChart();
}

// ——— Canvas Chart ———

function drawChart() {
    const canvas = document.getElementById('latency-chart');
    const ctx = canvas.getContext('2d');
    const W = canvas.width;
    const H = canvas.height;

    ctx.clearRect(0, 0, W, H);

    // Background grid
    ctx.strokeStyle = '#1a1a25';
    ctx.lineWidth = 1;
    for (let y = 0; y <= H; y += 40) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(W, y);
        ctx.stroke();
    }

    // Find max latency for scaling
    const allLatencies = [
        ...chartData.native.map(d => d.latency),
        ...chartData.blindhop.map(d => d.latency),
    ];
    const maxLatency = Math.max(100, ...allLatencies) * 1.1;

    // Draw lines
    function drawLine(data, color) {
        if (data.length < 2) return;
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        for (let i = 0; i < data.length; i++) {
            const x = (i / (MAX_CHART_POINTS - 1)) * W;
            const y = H - (data[i].latency / maxLatency) * H;
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.stroke();

        // Dots
        ctx.fillStyle = color;
        for (let i = 0; i < data.length; i++) {
            const x = (i / (MAX_CHART_POINTS - 1)) * W;
            const y = H - (data[i].latency / maxLatency) * H;
            ctx.beginPath();
            ctx.arc(x, y, 3, 0, Math.PI * 2);
            ctx.fill();
        }
    }

    drawLine(chartData.native, '#ff4d6a');
    drawLine(chartData.blindhop, '#00e599');

    // Y-axis labels
    ctx.fillStyle = '#8888a0';
    ctx.font = '10px JetBrains Mono';
    ctx.fillText(`${maxLatency.toFixed(0)}ms`, 4, 12);
    ctx.fillText('0ms', 4, H - 4);
}

// Init
setMode('both');
