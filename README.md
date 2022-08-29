# IPFS Registry

Signed package registry backed by IPFS for storage.

## Prerequisites

* [ipfs][]
* [rust][]

Minimum supported rust version (MSRV) is 1.63.0.

## Getting Started

Install the binary:

```
cargo install --path .
```

Ensure a local IPFS node is running:

```
ipfs daemon
```

Start the server:

```
ipkg server -c ./sandbox/config.toml
```

Generate a signing key; you will be prompted to choose a password for the keystore:

```
ipkg keygen ./sandbox
```

Publish a package:

```
ipkg publish -k ./sandbox/<addr>.json fixtures/mock-package-1.0.0.tgz
```

Replace `<addr>` with the address for the key and enter the password for the keystore when prompted.

Download the package to a file:

```
ipkg fetch -a <addr> -n mock-package -v 1.0.0 sandbox/package.tgz
```

## Upload a package

```
PUT /api/package
```

The default mime type the server respects for packages is `application/gzip` so you should ensure the `content-type` header is set correctly.

To upload a package it MUST be signed and the signature given in the `x-signature` header.

The `x-signature` header MUST be a base64 encoded string of a 65-byte Ethereum-style ECDSA recoverable signature.

The server will compute the address from the public key recovered from the signature and use that as the namespace for packages.

If a file already exists for the given package a 409 CONFLICT response is returned.

## Download a package

```
GET /api/package/:address/:name/:version
```

To download a package construct a URL containing the Ethereum-style address that was used when the package was uploaded along with the package name and semver.

[ipfs]: https://ipfs.io/
[rust]: https://www.rust-lang.org/
