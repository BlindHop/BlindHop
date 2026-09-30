// BlindHop v2 — Canvas-based real-time latency chart
// Plots latency over time with color-coded lines per privacy mode

const ChartColors = {
    none: '#ff4d6a',
    fast: '#ffb84d',
    full: '#00e599',
    grid: 'rgba(255, 255, 255, 0.04)',
    gridLabel: '#7a7a95',
    bg: 'transparent',
};

const MAX_POINTS = 60;

class LatencyChart {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.ctx = this.canvas.getContext('2d');
        this.data = {
            none: [],
            fast: [],
            full: [],
        };
        this.dpr = window.devicePixelRatio || 1;
        this.resize();
        window.addEventListener('resize', () => this.resize());
    }

    resize() {
        const rect = this.canvas.parentElement.getBoundingClientRect();
        const w = rect.width - 32; // Account for padding
        const h = 220;
        this.canvas.width = w * this.dpr;
        this.canvas.height = h * this.dpr;
        this.canvas.style.width = w + 'px';
        this.canvas.style.height = h + 'px';
        this.ctx.scale(this.dpr, this.dpr);
        this.W = w;
        this.H = h;
        this.draw();
    }

    addPoint(mode, latencyMs) {
        if (!this.data[mode]) return;
        this.data[mode].push({
            time: Date.now(),
            latency: latencyMs,
        });
        if (this.data[mode].length > MAX_POINTS) {
            this.data[mode].shift();
        }
        this.draw();
    }

    clear() {
        this.data = { none: [], fast: [], full: [] };
        this.draw();
    }

    draw() {
        const ctx = this.ctx;
        const W = this.W;
        const H = this.H;
        const pad = { top: 20, right: 15, bottom: 25, left: 55 };
        const plotW = W - pad.left - pad.right;
        const plotH = H - pad.top - pad.bottom;

        // Clear
        ctx.clearRect(0, 0, W, H);

        // Find max latency across all series for scaling
        let allLatencies = [];
        for (const key of Object.keys(this.data)) {
            allLatencies = allLatencies.concat(this.data[key].map(d => d.latency));
        }
        const maxLatency = Math.max(100, ...allLatencies) * 1.15;

        // Draw grid lines
        ctx.strokeStyle = ChartColors.grid;
        ctx.lineWidth = 1;
        const gridLines = 5;
        for (let i = 0; i <= gridLines; i++) {
            const y = pad.top + (plotH * i) / gridLines;
            ctx.beginPath();
            ctx.moveTo(pad.left, y);
            ctx.lineTo(W - pad.right, y);
            ctx.stroke();

            // Y-axis labels
            const val = maxLatency * (1 - i / gridLines);
            ctx.fillStyle = ChartColors.gridLabel;
            ctx.font = '10px "JetBrains Mono", monospace';
            ctx.textAlign = 'right';
            ctx.fillText(
                val >= 1000 ? `${(val / 1000).toFixed(1)}s` : `${val.toFixed(0)}ms`,
                pad.left - 8,
                y + 3
            );
        }

        // Draw each series
        for (const [mode, color] of [
            ['none', ChartColors.none],
            ['fast', ChartColors.fast],
            ['full', ChartColors.full],
        ]) {
            const points = this.data[mode];
            if (points.length < 2) continue;

            // Line
            ctx.strokeStyle = color;
            ctx.lineWidth = 2;
            ctx.lineJoin = 'round';
            ctx.beginPath();

            for (let i = 0; i < points.length; i++) {
                const x = pad.left + (i / (MAX_POINTS - 1)) * plotW;
                const y = pad.top + plotH - (points[i].latency / maxLatency) * plotH;
                if (i === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            }
            ctx.stroke();

            // Glow effect
            ctx.save();
            ctx.globalAlpha = 0.08;
            ctx.strokeStyle = color;
            ctx.lineWidth = 8;
            ctx.beginPath();
            for (let i = 0; i < points.length; i++) {
                const x = pad.left + (i / (MAX_POINTS - 1)) * plotW;
                const y = pad.top + plotH - (points[i].latency / maxLatency) * plotH;
                if (i === 0) ctx.moveTo(x, y);
                else ctx.lineTo(x, y);
            }
            ctx.stroke();
            ctx.restore();

            // Dots (last 5 points)
            const dotStart = Math.max(0, points.length - 5);
            ctx.fillStyle = color;
            for (let i = dotStart; i < points.length; i++) {
                const x = pad.left + (i / (MAX_POINTS - 1)) * plotW;
                const y = pad.top + plotH - (points[i].latency / maxLatency) * plotH;
                const r = i === points.length - 1 ? 4 : 2.5;
                ctx.beginPath();
                ctx.arc(x, y, r, 0, Math.PI * 2);
                ctx.fill();
            }
        }
    }
}

export { LatencyChart };
