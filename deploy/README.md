# Deploying the BlindHop exit

Only the **exit** runs on a server. It connects out to the Nym mixnet and to one Substrate full node; it opens **no inbound ports**. The proxy runs on each user's machine, and the demo is a static site.

Tested on Ubuntu 24.04 (x86_64) with systemd 255. Oracle's Ampere VMs are ARM (`aarch64`); the build is done on the server, so it works on either.

## 1. Prepare the server (once)

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev git curl

# Rust (official installer; the build needs Rust >= 1.88)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"

git clone https://github.com/BlindHop/BlindHop.git
cd BlindHop
git checkout nym
```

Building `nym-sdk` needs a few GB of RAM. `build-exit.sh` warns if RAM + swap is under 4 GB and prints the commands to add swap.

## 2. Build (as your normal user)

```bash
./deploy/build-exit.sh
```

This uses `--locked`, so it builds exactly the dependency versions in `Cargo.lock`. Expect several minutes on a small ARM VM.

## 3. Install and start (as root)

```bash
sudo ./deploy/install-exit.sh --target-rpc wss://your-full-node
```

`--target-rpc` is the Substrate node the exit forwards to. That node sees every query the exit forwards (not who sent it), so choose one you trust. The script:

- creates the `blindhop` system user (no login, no home);
- installs `/usr/local/bin/blindhop-exit`, `/etc/blindhop-exit.env` and the systemd unit (`deploy/blindhop-exit.service`);
- starts the service, waits for it to join the Nym network, and prints the exit's **Nym address**.

Put that address in `demo/app.js` (`CONFIG.defaultExitAddress`) and give it to proxy users (`--exit-address`).

## 4. Back up the keys

```bash
sudo ./deploy/backup-exit-keys.sh
```

The keys in `/var/lib/blindhop-exit/keys` **are** the exit's Nym address. If they're lost, the address changes and every user must be reconfigured. The script stops the exit for a few seconds, writes a `0600` archive, and restarts it. Copy the archive off the server.

To restore on a new server: install once (step 3), then `sudo systemctl stop blindhop-exit`, extract the archive into `/var/lib/blindhop-exit` (`sudo tar -xzf <archive> -C /var/lib/blindhop-exit && sudo chown -R blindhop:blindhop /var/lib/blindhop-exit`), and `sudo systemctl start blindhop-exit`.

## Upgrading

```bash
git pull
./deploy/build-exit.sh
sudo ./deploy/install-exit.sh      # keeps the keys, so the address stays the same
```

The running exit gets SIGTERM and disconnects from Nym cleanly before the new binary starts.

## Operating

| | |
|---|---|
| Status | `systemctl status blindhop-exit` |
| Logs | `journalctl -u blindhop-exit -f` |
| Address | `sudo cat /var/lib/blindhop-exit/.exit_nym_address` |
| Change full node | `sudo ./deploy/install-exit.sh --target-rpc wss://...` |
| Change Nym gateway | `sudo ./deploy/install-exit.sh --gateway <identity key>` (changes the address after `@`; update the demo) |
| Remove (keep keys) | `sudo ./deploy/uninstall-exit.sh` |
| Remove everything | `sudo ./deploy/uninstall-exit.sh --purge` |

Keep logging at `info` (the default in the unit); debug logs record request sizes and timings.

### Limits

The unit caps the exit at `MemoryMax=320M` and one CPU core (`CPUQuota=100%`), with `CPUWeight=50` so other services on the server get twice its CPU share when they compete. The exit itself handles at most 16 requests at once (8 per client); a request that finds no free slot waits up to 5 s for one (at most 32 waiting) before being answered "busy" (`-32005`). It forwards only an allowlist of read-only methods plus `author_submitExtrinsic` (no batches or subscriptions), and caps requests at 1 MiB and responses at 4 MiB. In load testing, peak memory was ~100 MB; the worst case at these limits is estimated at ~240 MB.

To change a unit setting without editing the file: `sudo systemctl edit blindhop-exit`, then add e.g.

```ini
[Service]
MemoryMax=512M
```

### Self-healing

The exit exits with an error (so systemd restarts it) if its Nym client shuts down unexpectedly, or if a small probe it sends itself through the mixnet every 60 s hasn't come back for 180 s. The second case covers a gateway that silently stops delivering messages. If restarts repeat, check `journalctl -u blindhop-exit` for gateway errors and consider `--gateway` to move to another gateway.

### Hardening

The unit runs the exit as an unprivileged user with no capabilities, a read-only filesystem except its state directory, private `/tmp` and devices, only IPv4/IPv6/Unix sockets, and the `@system-service` syscall set. If the service fails to start after an OS upgrade, check `journalctl -u blindhop-exit` for a blocked syscall (`SIGSYS`) or permission error.
