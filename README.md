# IPFS Registry

Signed package registry backed by IPFS for storage.

## Preqrequsites

* [ipfs][]
* [rust][]

Minimum supported rust version (MSRV) is 1.63.0.

## Getting Started

Ensure a local IPFS node is running:

```
ipfs daemon
```

Start the server:

```
cd workspace/server
cargo run -- -c ../../sandbox/config.toml
```

## Upload a package

```
curl -vXPUT http://localhost:9060/api/package \
  -H "Content-Type: application/gzip" \
  --data-binary "@fixtures/mock-package-1.0.0.tgz"
```

[ipfs]: https://ipfs.io/
[rust]: https://www.rust-lang.org/
