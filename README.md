# IPFS Registry

Signed package registry backed by IPFS for storage.

## Prerequisites

* [ipfs][]
* [rust][]

Minimum supported rust version (MSRV) is 1.63.0.

## Abstract

Content addressing used by the IPFS network is a good fit for a package registry as it prevents packages from being tampered with and provides decentralized storage of the package archives.

However, there is a tension between using opaque identifiers and exposing human-friendly references to packages. To resolve this we support both types of references so that callers can choose from *tamper proof* in the case of an opaque Content Identifier (CID) or from *tamper protected* in the case of a human-readable package reference.

For example, to fetch a package from the registry using a CID such as:

```
/ipfs/QmSYVWjXh5GCZpxhCSHMa89X9VHnPpaxafkBAR9rjfCenb
```

Can be said to be *tamper proof* as changing the package contents would also change the CID.

This kind of package reference does not tell us anything about the package name or version which may not be useful depending upon the use case. The registry will also accept a *pointer* to a package such as:

```
mock-namespace/mock-package/1.0.0
```

In this case the registry will look up the object key in a database before returning the package file. Using a *pointer* reference is said to be *tamper protected* because it is possible for the registry operator to change the object key.

In the future support can be added to mitigate this by storing object references in a smart contract ensuring that both kinds of package references are *tamper proof*.

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

For API calls that require authentication the `x-signature` header MUST be a base64 encoded string of a 65-byte Ethereum-style ECDSA recoverable signature.

### Signup

```
POST /api/publisher
```

Register a signing key for publishing.

#### Headers

* `x-signature`: Signature of the well known value `.ipfs-registry`.

#### Response

```json
{
  "address": "0x1fc770ac21067a04f83101ebf19a670db9e3eb21",
  "created_at": "2022-09-11T08:28:17Z"
}
```

### Register

```
POST /api/namespace/:namespace
```

Register a namespace; if the namespace already exists a 409 CONFLICT response is returned.

#### Headers

* `x-signature`: Signature of the bytes for `:namespace`.

#### Response

```json
{
  "name": "mock-namespace",
  "owner": "0x1fc770ac21067a04f83101ebf19a670db9e3eb21",
  "publishers": [],
  "created_at": "2022-09-11T08:29:27Z"
}
```

### Upload a package

```
PUT /api/package/:namespace
```

If the package already exists a 409 CONFLICT response is returned.

If the address of the publisher has been denied based on the server configuration's `allow` and `deny` sets then a 401 UNAUTHORIZED response is returned.

The default configuration limits requests to 16MiB so if the package is too large a 413 PAYLOAD TOO LARGE response is returned.

#### Parameters

* `:namespace`: The package namespace.

#### Headers

* `x-signature`: Signature of the bytes for the request body.
* `content-type`: Should match the MIME type for the registry (default: `application/gzip`)

#### Response

```json
{
  "id": "mock-namespace/mock-package/1.0.0",
  "artifact": {
    "namespace": "mock-namespace",
    "package": {
      "name": "mock-package",
      "version": "1.0.0"
    }
  },
  "key": "/ipfs/QmSYVWjXh5GCZpxhCSHMa89X9VHnPpaxafkBAR9rjfCenb"
}
```

### Download a package

```
GET /api/package?id=<package-id>
```

To download a package construct a URL containing the package identifier; the identifier may be an IPFS reference such as:

```
/ipfs/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi
```

Or a package pointer:

```
mock-namespace/mock-package/1.0.0
```

#### Query

* `id`: Package identifier.

### List packages

```
GET /api/package/:namespace
```

List the packages for a namespace.

#### Query

* `versions`: Fetch versions for each package, either `none` or  `latest`. Default is `none`.
* `limit`: Limit per page.
* `offset`: Offset for pagination.
* `sort`: Sort order, either `asc` or `desc`.

#### Response

```json
{
  "records": [
    {
      "name": "mock-package",
      "created_at": "2022-09-11T08:30:27Z"
    }
  ],
  "count": 1
}
```

### List versions

```
GET /api/package/:namespace/:package
```

List the versions of a package.

#### Query

* `limit`: Limit per page.
* `offset`: Offset for pagination.
* `sort`: Sort order, either `asc` or `desc`.

#### Response

```json
{
  "records": [
    {
      "version": "1.0.0",
      "content_id": "QmSYVWjXh5GCZpxhCSHMa89X9VHnPpaxafkBAR9rjfCenb",
      "signature": "9a0b6450d1f42380f826a86f2d8106d6c9db041c912d90a6063e6bf8a28989301551d0458fb4a6f49b467334d7d1d9368e3b411f4c1b2ce7052167ac422c150301",
      "checksum": "4ad90a2c2e08374f8ccec2b604915a0ab7e97fcca983b12a6857d20df3fca9c0",
      "created_at": "2022-09-11T08:30:27Z"
    },
    {
      "version": "1.0.1",
      "content_id": "QmQfiqgpEL7gWavVJ5r2JK17N516q9wWoL8eHjwq8zKozZ",
      "signature": "dad0482f1096ad9f0caef5d422e742ff4052c497fcfa0a2bead256886c713770357bf810297aed168e3e0888e7e00b73f2e50d432addcd9b46ebeff03ee3d06c00",
      "checksum": "6fb6f92379c52eeb7f18d56c6fc745755588ebbccd5db0e157c9938daaf5e359",
      "created_at": "2022-09-12T01:34:05Z"
    },
    {
      "version": "2.0.0-alpha.1",
      "content_id": "QmbptdWzd7pzNbmTkGwtYRdQWYCmXYjQ6tJV9CkWkjD2V8",
      "signature": "92b39f8aacaa109e136194c98a47e741d058c6a1b042fe0453f24aaedeb878b5499c388162a48e55f1f88756e9735821ac26f84b7db75060e0ac2cad1dce37b300",
      "checksum": "58313c4525d2253048a7b7342bb63b4a914bd5ae2ee5eab9e22f35c8897b5db5",
      "created_at": "2022-09-12T02:22:16Z"
    }
  ],
  "count": 3
}
```

### Latest version

```
GET /api/package/:namespace/:package/latest
```

Get the latest version of a package.

#### Query

* `prerelease`: When `true` include prerelease versions.

#### Response

Response with `?prerelease=true` query string:

```json
{
  "version": "2.0.0-alpha.1",
  "package": {
    "author": "",
    "description": "Mock package to test NPM registry support",
    "license": "ISC",
    "main": "index.js",
    "name": "mock-package",
    "scripts": {
      "test": "echo \"Error: no test specified\" && exit 1"
    },
    "version": "2.0.0-alpha.1"
  },
  "content_id": "QmbptdWzd7pzNbmTkGwtYRdQWYCmXYjQ6tJV9CkWkjD2V8",
  "signature": "92b39f8aacaa109e136194c98a47e741d058c6a1b042fe0453f24aaedeb878b5499c388162a48e55f1f88756e9735821ac26f84b7db75060e0ac2cad1dce37b300",
  "checksum": "58313c4525d2253048a7b7342bb63b4a914bd5ae2ee5eab9e22f35c8897b5db5",
  "created_at": "2022-09-12T02:22:16Z"
}
```

### Package version

```
GET /api/package/:namespace/:package/:version
```

Get a specific version of a package.

#### Response

See example response for latest version above.

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
cargo make dev-db
```

Typical workflow is to run the test suite and format the code:

```
cargo make dev
```

Starting a local server (requires an IPFS node running locally):

```
cargo make dev-server
```

### TLS Support

To test TLS support for IPFS nodes, set up CORS for `https://localhost`:

```
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin '["https://localhost", "http://localhost:3000", "http://127.0.0.1:5001", "https://webui.ipfs.io"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "POST"]'
```

Then install and run [caddy][] as a reverse proxy:

```
caddy reverse-proxy --to 127.0.0.1:5001
```

Make sure you can view `https://localhost/webui` and then create a configuration that connects to IPFS over HTTPS:

```toml
[database]
url = "sqlite:ipfs_registry.db"

[storage]
layers = [
  { url = "https://localhost" }
]
```

And start the server:

```
cargo run -- server -c sandbox/ipfs-tls.toml
```

## License

MIT or Apache-2.0

[ipfs]: https://ipfs.io/
[rust]: https://www.rust-lang.org/
[object locks]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-lock.html
[versioning]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/Versioning.html
[caddy]: https://caddyserver.com/
