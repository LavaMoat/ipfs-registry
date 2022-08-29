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

## API

### Upload a package

```
PUT /api/package
```

The default mime type the server respects for packages is `application/gzip` so you should ensure the `content-type` header is set correctly.

To upload a package it MUST be signed and the signature given in the `x-signature` header.

The `x-signature` header MUST be a base64 encoded string of a 65-byte Ethereum-style ECDSA recoverable signature.

The server will compute the address from the public key recovered from the signature and use that as the namespace for packages.

If a file already exists for the given package a 409 CONFLICT response is returned.

If the address of the publisher has been denied based on the server configuration's `allow` and `deny` sets then a 401 UNAUTHORIZED response is returned.

The default configuration limits requests to 16MiB so if the package is too large a 413 PAYLOAD TOO LARGE response is returned.

### Download a package

```
GET /api/package/:address/:name/:version
```

To download a package construct a URL containing the Ethereum-style address that was used when the package was uploaded along with the package name and semver.

## Configuration

This section describes the server configuration; after making changes to the configuration you must restart the server for changes to take effect.

### IPFS

If the IPFS node is running on a different port to the default you can change the URL to use:

```toml
[ipfs]
url = "http://localhost:7007"
```

Note that currently HTTPS connections to IPFS are not supported.

### Registry

#### Body Limit

If you need to allow packages larger than the default 16MiB use `body-limit`:

```toml
[registry]
body-limit = 33554432   # 32MiB
```

#### Allow

To restrict access to an allowed list of publishers specify addresses in the `allow` set:

```toml
[registry]
allow = [
  "0x1fc770ac21067a04f83101ebf19a670db9e3eb21"
]
```

#### Deny

To deny publish access use the `deny` set:

```toml
[registry]
deny = [
  "0x1fc770ac21067a04f83101ebf19a670db9e3eb21"
]
```

### TLS

To run the server over HTTPS specify certificate and key files:

```toml
[tls]
cert = "cert.pem"
key = "key.pem"
```

Relative paths are resolved from the directory containing the configuration file.

## Bugs

The package meta data is not immutable and theoretically the meta data could be modified to point to a different `cid` which could allow an attacker to replace the file pointer. This could be mitigated by storing the package meta data in a blockchain.

[ipfs]: https://ipfs.io/
[rust]: https://www.rust-lang.org/
