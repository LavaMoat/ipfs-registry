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

Signup so the public key is registered for publishing:

```
ipkg signup -k ./sandbox/<addr>.json
```

Replace `<addr>` with the address of the public key and enter the password for the keystore when prompted.

Register a namespace for published packages:

```
ipkg register -k ./sandbox/<addr>.json mock-namespace
```

Publish a package:

```
ipkg publish -k ./sandbox/<addr>.json -n mock-namespace fixtures/mock-package-1.0.0.tgz
```

Download the package to a file:

```
ipkg fetch <addr>/mock-package/1.0.0 sandbox/package.tgz
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
GET /api/package?id=<package-id>
```

To download a package construct a URL containing the package identifier; the identifier may be an explicit IPFS reference such as:

```
/ipfs/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi
```

Or a package pointer:

```
example.com/mock-package/1.0.0
```

When a package pointer is used to fetch a package then the checksum and signature are verified.

## Configuration

This section describes the server configuration; after making changes to the configuration you must restart the server for changes to take effect.

### Storage

Storage for packages and pointers can be defined as an ordered set of layers.

You must define at least one layer; to define an IPFS layer specify an object with a `url` field that points to the node URL.

```toml
[storage]
layers = [
  { url = "https://ipfs-node1.example.com" }
]
```

For example, to mirror to multiple IPFS nodes:

```toml
[storage]
layers = [
  { url = "https://ipfs-node1.example.com" },
  { url = "https://ipfs-node2.example.com" },
  { url = "https://ipfs-node3.example.com" },
]
```

To define a storage layer backed by an AWS S3 bucket you must specify the `profile`, `region` and `bucket`; the `profile` must be a valid profile in `~/.aws/credentials` with read and write permissions for the bucket.

```toml
[storage]
layers = [
  { region = "ap-southeast-1", profile = "example", bucket = "registry.example.com" }
]
```

When using an AWS S3 bucket as a storage layer in production it is ***strongly recommended*** that the bucket has [versioning][] and [object locks][] enabled. 
Mixing layers is encouraged for redundancy:

```toml
[storage]
layers = [
  { url = "https://ipfs-node1.example.com" },
  { region = "ap-southeast-1", profile = "example", bucket = "registry.example.com" },
]
```

Note that all the downstream storage layers must be available for the service to work as intended; ie, requests must succeeed across all storage layers for the server to return a success response.

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

### CORS

The default CORS configuration is very permissive, if you wish to restrict to certain origins:

```toml
[cors]
origins = [
  "https://example.com"
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

## Developers

Install `sqlx` and `cargo make`:

```
cargo install sqlx-cli
cargo install cargo-make
```

Then create a `.env` file from `.env.example`. Afterwards, create a database and run the migrations:

```
sqlx database create
sqlx migrate run
```

## Bugs

The package meta data is not immutable and theoretically the meta data could be modified to point to a different `cid` which could allow an attacker to replace the file pointer. This could be mitigated by storing the package meta data in a blockchain.

## License

MIT or Apache-2.0

[ipfs]: https://ipfs.io/
[rust]: https://www.rust-lang.org/
[object locks]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-lock.html
[versioning]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/Versioning.html
