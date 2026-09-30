// BlindHop — Nym Browser Client (Wasm SDK Wrapper)
//
// Wraps @nymproject/sdk-full-fat to provide a simple interface for
// sending JSON-RPC requests through the Nym mixnet from the browser.
//
// Requests and replies are BlindHop frames (see frame.js). They are sent
// with rawSend (no SDK mime-type framing) and read from the raw message
// event, since the exit replies with raw bytes.

import { createNymMixnetClient } from '@nymproject/sdk-full-fat';
import {
    FRAME_TYPE_REQUEST,
    FRAME_TYPE_RESPONSE,
    SUPPORTS_COMPRESSION,
    decodeFrame,
    encodeFrame,
    inflateRaw,
} from './frame.js';

const NYM_API_URL = 'https://validator.nymtech.net/api';

// Reply SURBs attached to each request. Large replies need more; the exit
// requests top-ups as needed, so this only has to cover small replies.
const REPLY_SURBS = 10;

// Give up on a reply after this long, matching blindhop-proxy.
const REQUEST_TIMEOUT_MS = 60_000;

// Give up on connecting to the mixnet after this long.
const CONNECT_TIMEOUT_MS = 60_000;

/**
 * Browser-based Nym mixnet client for BlindHop.
 *
 * Usage:
 *   const client = new NymBrowserClient();
 *   await client.connect(exitNymAddress);
 *   client.onResponse((jsonRpcResponse) => { ... });
 *   client.sendRequest('{"jsonrpc":"2.0","id":1,"method":"chain_getHeader","params":[]}');
 *   client.disconnect();
 */
export class NymBrowserClient {
    constructor() {
        this._nym = null;
        this._exitAddress = null;
        this._selfAddress = null;
        this._onResponse = null;
        this._onStatusChange = null;
        this._connected = false;
        this._unsubscribe = null;
        this._nextId = 1;
        // correlation ID -> timeout handle, for requests awaiting a reply
        this._pending = new Map();
    }

    /**
     * Connect to the Nym mixnet via a browser Wasm client.
     * @param {string} exitAddress - The Nym address of the blindhop-exit service.
     * @returns {Promise<string>} This client's Nym address.
     */
    async connect(exitAddress) {
        this._exitAddress = exitAddress;

        this._setStatus('connecting', 'Initializing Nym Wasm client...');

        this._nym = await createNymMixnetClient();

        // Raw event: the exit's replies carry no SDK mime-type framing, so the
        // text/binary events never fire for them.
        this._unsubscribe = this._nym.events.subscribeToRawMessageReceivedEvent((event) => {
            this._handleIncoming(event.args.payload);
        });

        this._setStatus('connecting', 'Connecting to Nym gateway...');

        // start() resolves before the client is connected; anything sent
        // before the Connected event is silently dropped.
        let unsubscribeConnected;
        let timer;
        const connected = new Promise((resolve, reject) => {
            unsubscribeConnected = this._nym.events.subscribeToConnected((event) => {
                resolve(event.args.address);
            });
            timer = setTimeout(
                () => reject(new Error(`Nym client did not connect within ${CONNECT_TIMEOUT_MS / 1000}s`)),
                CONNECT_TIMEOUT_MS,
            );
        });

        try {
            await this._nym.client.start({
                clientId: `blindhop-browser-${Date.now()}`,
                nymApiUrl: NYM_API_URL,
            });
            this._selfAddress = await connected;
        } finally {
            clearTimeout(timer);
            unsubscribeConnected();
        }
        this._connected = true;
        this._setStatus('connected', `Nym connected (${String(this._selfAddress).slice(0, 16)}...)`);

        return this._selfAddress;
    }

    /**
     * Send a JSON-RPC request through the Nym mixnet to the exit service.
     * The reply is delivered to the onResponse callback.
     * @param {string} jsonRpcString - Serialized JSON-RPC request.
     */
    async sendRequest(jsonRpcString) {
        if (!this._connected || !this._nym) {
            throw new Error('Nym client not connected');
        }

        const id = this._nextId++;
        // Ask for compressed replies if this browser can decompress them.
        const frame = encodeFrame(
            FRAME_TYPE_REQUEST, id, new TextEncoder().encode(jsonRpcString), SUPPORTS_COMPRESSION,
        );

        this._pending.set(id, setTimeout(() => {
            this._pending.delete(id);
            console.warn(`[NymBrowser] No reply for request ${id} within ${REQUEST_TIMEOUT_MS / 1000}s`);
        }, REQUEST_TIMEOUT_MS));

        try {
            await this._nym.client.rawSend({
                payload: frame,
                recipient: this._exitAddress,
                replySurbs: REPLY_SURBS,
            });
        } catch (e) {
            clearTimeout(this._pending.get(id));
            this._pending.delete(id);
            throw e;
        }
    }

    /**
     * Register a callback for JSON-RPC responses from the exit service.
     * @param {function(string): void} callback - Called with the raw JSON-RPC response string.
     */
    onResponse(callback) {
        this._onResponse = callback;
    }

    /**
     * Register a callback for connection status changes.
     * @param {function(string, string): void} callback - Called with (status, message).
     *   status: 'connecting' | 'connected' | 'disconnected' | 'error'
     */
    onStatusChange(callback) {
        this._onStatusChange = callback;
    }

    /**
     * Disconnect from the Nym mixnet.
     */
    async disconnect() {
        for (const timer of this._pending.values()) clearTimeout(timer);
        this._pending.clear();
        if (this._unsubscribe) {
            this._unsubscribe();
            this._unsubscribe = null;
        }
        if (this._nym) {
            try {
                await this._nym.client.stop();
            } catch (e) {
                console.warn('[NymBrowser] Error stopping client:', e);
            }
            this._nym = null;
        }
        this._connected = false;
        this._selfAddress = null;
        this._setStatus('disconnected', 'Nym disconnected');
    }

    /** Whether the client is currently connected. */
    get isConnected() {
        return this._connected;
    }

    /** This client's Nym address (null if not connected). */
    get selfAddress() {
        return this._selfAddress;
    }

    // --- Private ---

    /**
     * Handle an incoming message from the Nym mixnet (SURB reply from exit).
     * Decodes the frame, matches it to a pending request, decompresses it if
     * needed, and fires the response callback.
     */
    async _handleIncoming(bytes) {
        const frame = decodeFrame(bytes);
        if (!frame) {
            console.warn('[NymBrowser] Ignoring message that is not a BlindHop frame');
            return;
        }
        if (frame.type !== FRAME_TYPE_RESPONSE) {
            console.warn('[NymBrowser] Ignoring non-response frame');
            return;
        }

        const timer = this._pending.get(frame.correlationId);
        if (timer === undefined) {
            console.warn(`[NymBrowser] Dropping reply with unknown correlation ID ${frame.correlationId} (timed out or duplicate)`);
            return;
        }
        clearTimeout(timer);
        this._pending.delete(frame.correlationId);

        // Decompress only replies we asked for (bounded by MAX_DECOMPRESSED_BYTES).
        let payload = frame.payload;
        if (frame.compressed) {
            try {
                payload = await inflateRaw(payload);
            } catch (e) {
                console.error(`[NymBrowser] Could not decompress reply ${frame.correlationId}:`, e);
                return;
            }
        }

        if (this._onResponse) {
            this._onResponse(new TextDecoder().decode(payload));
        }
    }

    _setStatus(status, message) {
        console.log(`[NymBrowser] ${status}: ${message}`);
        if (this._onStatusChange) {
            this._onStatusChange(status, message);
        }
    }
}
