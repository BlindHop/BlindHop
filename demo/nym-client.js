// BlindHop — Nym Browser Client (Wasm SDK Wrapper)
//
// Wraps @nymproject/sdk-full-fat to provide a simple interface for
// sending JSON-RPC requests through the Nym mixnet from the browser.
//
// Wire format matches blindhop-exit's MixnetMessage:
//   { "payload": [u8 array], "msg_type": "Request" | "Response" }

import { createNymMixnetClient } from '@nymproject/sdk-full-fat';

const NYM_API_URL = 'https://validator.nymtech.net/api';

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

        // Subscribe to incoming messages (SURB replies from exit service)
        this._nym.events.subscribeToTextMessageReceivedEvent((event) => {
            this._handleIncoming(event.args.payload);
        });

        this._setStatus('connecting', 'Connecting to Nym gateway...');

        await this._nym.client.start({
            clientId: `blindhop-browser-${Date.now()}`,
            nymApiUrl: NYM_API_URL,
        });

        this._selfAddress = this._nym.client.selfAddress();
        this._connected = true;
        this._setStatus('connected', `Nym connected (${this._selfAddress.slice(0, 16)}...)`);

        return this._selfAddress;
    }

    /**
     * Send a JSON-RPC request through the Nym mixnet to the exit service.
     * The request is wrapped in a MixnetMessage envelope matching the Rust struct.
     * @param {string} jsonRpcString - Serialized JSON-RPC request.
     */
    async sendRequest(jsonRpcString) {
        if (!this._connected || !this._nym) {
            throw new Error('Nym client not connected');
        }

        // Build MixnetMessage envelope matching blindhop-exit's serde format:
        //   { "payload": [byte array], "msg_type": "Request" }
        const payloadBytes = Array.from(new TextEncoder().encode(jsonRpcString));
        const envelope = JSON.stringify({
            payload: payloadBytes,
            msg_type: 'Request',
        });

        await this._nym.client.send({
            payload: envelope,
            recipient: this._exitAddress,
        });
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
     * Unwraps the MixnetMessage envelope and fires the response callback.
     */
    _handleIncoming(rawPayload) {
        try {
            const envelope = JSON.parse(rawPayload);

            // Validate it's a response
            if (envelope.msg_type !== 'Response') {
                console.warn('[NymBrowser] Unexpected msg_type:', envelope.msg_type);
                return;
            }

            // Decode payload bytes back to JSON-RPC string
            const jsonRpcString = new TextDecoder().decode(new Uint8Array(envelope.payload));

            if (this._onResponse) {
                this._onResponse(jsonRpcString);
            }
        } catch (e) {
            console.error('[NymBrowser] Failed to parse incoming message:', e);
        }
    }

    _setStatus(status, message) {
        console.log(`[NymBrowser] ${status}: ${message}`);
        if (this._onStatusChange) {
            this._onStatusChange(status, message);
        }
    }
}
