# Install

V0.1 ships GitHub Release binaries (Linux musl, macOS) and source builds. Package registries are not required.

## From source

Requires Rust 1.75+.

```bash
git clone https://github.com/devcy0922/coexistgate
cd coexistgate
cargo install --path crates/coexistgate-cli
coexistgate --version
```

## GitHub Release

Download `coexistgate-<os>-<arch>` and `SHA256SUMS` from the release. Verify:

```bash
shasum -a 256 -c SHA256SUMS
chmod +x coexistgate-*-*
sudo mv coexistgate-*-* /usr/local/bin/coexistgate
```

[`scripts/install.sh`](../scripts/install.sh) downloads the latest release for your OS. **Read the script first.** It checks SHA-256 when `SHA256SUMS` is present.

## Docker

```bash
# after building a musl binary named coexistgate
docker build -t coexistgate:0.1.0 .
docker run --rm -v "$PWD":/repo -w /repo coexistgate:0.1.0 analyze .
```

## CI

See [`examples/github-action.yml`](../examples/github-action.yml).
