# Install Romit

Pre-built binaries ship on [GitHub Releases](https://github.com/romit07/romitlang/releases) for **Linux** `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, **macOS** `x86_64-apple-darwin` and `aarch64-apple-darwin`, and **Windows** `x86_64-pc-windows-msvc`. Linux builds use **glibc** dynamic linking on the official GNU triples.

## One-line install

### macOS and Linux

```bash
curl -fsSL https://romitlang.org/install.sh | sh
```

### Windows

```powershell
irm https://romitlang.org/install.ps1 | iex
```

Set `ROMIT_MODIFY_PATH=1` before piping if you want the installer to append `~/.romit/bin` to your shell config or Windows user `PATH` automatically.

## macOS

Download the `.tar.gz` for your CPU:

| Hardware | Asset triple |
|----------|----------------|
| Apple Silicon | `aarch64-apple-darwin` |
| Intel Mac | `x86_64-apple-darwin` |

```bash
export ROMIT_VERSION=0.0.3
curl -fsSL -O "https://github.com/romit07/romitlang/releases/download/v${ROMIT_VERSION}/romit-${ROMIT_VERSION}-aarch64-apple-darwin.tar.gz"
curl -fsSL -O "https://github.com/romit07/romitlang/releases/download/v${ROMIT_VERSION}/SHA256SUMS"
sha256sum -c SHA256SUMS --ignore-missing
tar -xzf "romit-${ROMIT_VERSION}-aarch64-apple-darwin.tar.gz"
cp romit-*/romit ~/.romit/bin/romit
chmod +x ~/.romit/bin/romit
```

**PATH — bash / zsh** — add to `~/.zshrc` or `~/.bash_profile`:

```bash
export PATH="$HOME/.romit/bin:$PATH"
```

**PATH — fish** — in `~/.config/fish/config.fish`:

```fish
fish_add_path $HOME/.romit/bin
```

## Linux

| Architecture | Asset triple |
|--------------|----------------|
| amd64 | `x86_64-unknown-linux-gnu` |
| arm64 | `aarch64-unknown-linux-gnu` |

```bash
export ROMIT_VERSION=0.0.3
curl -fsSL -O "https://github.com/romit07/romitlang/releases/download/v${ROMIT_VERSION}/romit-${ROMIT_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
curl -fsSL -O "https://github.com/romit07/romitlang/releases/download/v${ROMIT_VERSION}/SHA256SUMS"
sha256sum -c SHA256SUMS --ignore-missing
mkdir -p ~/.romit/bin
tar -xzf "romit-${ROMIT_VERSION}-x86_64-unknown-linux-gnu.tar.gz" -C /tmp
cp /tmp/romit-*/romit ~/.romit/bin/
chmod +x ~/.romit/bin/romit
```

Use the same PATH snippets as [macOS](#macos).

## Windows

Download `romit-${ROMIT_VERSION}-x86_64-pc-windows-msvc.zip` from Releases, verify against `SHA256SUMS`, and extract `romit.exe` to `%USERPROFILE%\.romit\bin`.

Add that folder to the **User** `PATH` in Settings → System → Environment variables, or:

```powershell
$env:ROMIT_MODIFY_PATH = '1'
irm https://romitlang.org/install.ps1 | iex
```

## Verify checksums

Every release includes `SHA256SUMS`. After downloading an archive and the sums file:

```bash
sha256sum -c SHA256SUMS --ignore-missing
```

On Windows PowerShell:

```powershell
Get-FileHash .\romit-0.0.3-x86_64-pc-windows-msvc.zip -Algorithm SHA256
# Compare with the matching line in SHA256SUMS
```

## Update

Re-run the one-line installer, or download the newer release asset and replace the binary in `~/.romit/bin`.

Pin a version:

```bash
ROMIT_VERSION=0.0.3 curl -fsSL https://romitlang.org/install.sh | sh
```

## Uninstall

```bash
curl -fsSL https://romitlang.org/install.sh | sh -s -- --uninstall
```

Remove any `PATH` lines you added manually. On Windows:

```powershell
irm https://romitlang.org/install.ps1 | iex; install.ps1 -Uninstall
```

Or delete `%USERPROFILE%\.romit\bin\romit.exe` and remove the directory from user `PATH`.

## Build from source

Requires a Rust toolchain:

```bash
git clone https://github.com/romit07/romitlang.git
cd romitlang
cargo build --release
./target/release/romit --version
```

Copy `target/release/romit` anywhere on your `PATH`.

## Troubleshooting

| Problem | What to try |
|---------|-------------|
| `curl: 404` on install | No GitHub Release yet, or set `ROMIT_VERSION` to a published tag. |
| Checksum mismatch | Re-download both the archive and `SHA256SUMS`; check for partial downloads. |
| `romit: command not found` | Ensure `~/.romit/bin` is on `PATH` and start a new shell. |
| Wrong Linux arch | Pick the GNU triple that matches `uname -m`. |
| Corporate proxy | Set `HTTPS_PROXY` for `curl` / `Invoke-WebRequest`. |

For local installer development, point at a directory served over HTTP:

```bash
ROMIT_BASE_URL=http://127.0.0.1:8765 ROMIT_VERSION=0.0.1-test sh install.sh
```
