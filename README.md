# IPFS Registry

Signed package registry backed by the Inter-Planetary File System for storage.

## Prerequisites

* [ipfs][]
* [rust][]

Minimum supported rust version (MSRV) is 1.63.0.

## Abstract

Content addressing used by the Inter-Planetary File System (IPFS) network is a good fit for a package registry as it prevents packages from being tampered with and provides decentralized storage of the package archives.

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

### Identity

Using ECDSA recoverable signatures to identify publishers avoids storing any Personally Identifiable Information (PII) and allows us to add Self-Sovereign Identity (SSI) support enabling publishers to verify their identity using Distributed Identifiers (DID) and Verifiable Credentials (VC).

All packages must be signed so we establish irrefutable proof of which identity published a package.

Clients could in the future support Multi-Party Computation (MPC) for package publishing which would allow organizations to ensure multiple parties are involved in signing off a package helping increase the security of the supply chain.

### Discoverability

A key feature of any package registry is the ability to discover packages; meta data about the published packages is stored in a database and exposed via a public API.

### Redundancy

The package registry supports multiple storage layers so it can be configured to automatically mirror published packages; see [storage configuration](#storage) for more information.

### Namespaces

Namespaces are useful as a means to establish trust for a collection of packages and to allow publishers to name their packages without collisions.

They don't prevent name-squatting as that problem just moves from the package name level to the namespace level; but they do help to make it easier to identify the author(s) of a package so we designed the registry with namespaces baked in.

### Generic Archives

The only thing the registry needs to extract from a package archive is the package *name* and [semver][] so it can easily support different kinds of packages.

Currently support is provided for [npm][] packages (the default) as well as [crates][] generated by `cargo package`; let us know if you have a package archive format that you would like to support.

### Unicode Security

To mitigate identifier based attacks all namespace and package names are subject to the [unicode security mechanisms][]; mixed script and confusable detection is thanks to the [unicode security crate][].

* Identifier MUST be at least three characters in length
* Identifier MUST have an alphabetic first character
* Identifier MUST NOT contain ASCII control characters
* Identifier MUST NOT contain ASCII punctuation (except for the hyphen)
* Identifier MUST NOT contain emojis
* Identifier MUST NOT contain invisible characters
* Identifier MUST conform to the general security profile, see [general security profile][]
* Identifier MUST be a single script, see [single script][]

#### Confusables

Namespaces and packages store a confusable skeleton in the database and comparison is performed on the skeleton when retrieving namespaces and packages by identifier which provides some protection for registering identifiers that are confusable, see [confusables][].

### Access Control

Organizations need to manage multiple signing keys and possibly restrict access to certain packages as well as support publishing in Continuous Integration / Continuous Deployment (CI/CD) pipelines.

The owner of a namespace has complete control and can create administrators and users.

Administrators can publish to all packages as well as add and remove other non-administrator users.

Users with no access restrictions can publish to all packages; if package access restrictions have been applied then publishing is restricted to the allowed list of packages.

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
ipkg fetch mock-namespace/mock-package/1.0.0 sandbox/package.tgz
```

Get information about the package version:

```
ipkg get mock-namespace/mock-package/1.0.0
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
  "created_at": "2022-09-11T08:29:27Z"
}
```

### Add a user

```
POST /api/namespace/:namespace/user/:address
```

Add a user to a namespace.

If the user already has access to the namespace a 409 CONFLICT response is returned.

If the address of the signer has been denied then a 401 UNAUTHORIZED response is returned.

#### Query

* `admin`: Boolean indicating the user is an administrator (default: `false`).
* `package`: Optional name of a package restriction for the new user.

#### Headers

* `x-signature`: Signature of the bytes for `:address`.

#### Response

200 if successful.

### Remove a user

```
DELETE /api/namespace/:namespace/user/:address
```

Remove a user from a namespace.

If the address of the signer has been denied then a 401 UNAUTHORIZED response is returned.

#### Headers

* `x-signature`: Signature of the bytes for `:address`.

#### Response

200 if successful.

### Upload a package

```
POST /api/package/:namespace
```

If the package already exists or is not ahead of the latest version a 409 CONFLICT response is returned.

If the address of the signer has been denied then a 401 UNAUTHORIZED response is returned.

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
  "key": "/ipfs/QmSYVWjXh5GCZpxhCSHMa89X9VHnPpaxafkBAR9rjfCenb",
  "checksum": "4ad90a2c2e08374f8ccec2b604915a0ab7e97fcca983b12a6857d20df3fca9c0"
}
```

### Download a package

```
GET /api/package?id=<package-id>
```

To download a package construct a URL containing the package identifier; the identifier may be an IPFS reference such as:

```
/ipfs/QmSYVWjXh5GCZpxhCSHMa89X9VHnPpaxafkBAR9rjfCenb
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

#### Parameters

* `:namespace`: The package namespace.

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

#### Parameters

* `:namespace`: The package namespace.
* `:package`: The package name.

#### Query

* `range`: Version range query, see [semver crate][] for details.
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
      "pointer_id": "230e83dd43123aa0f3d8bc337b0f63440a6128ae8491ee70f42d02594c087d49",
      "signature": "mgtkUNH0I4D4JqhvLYEG1snbBByRLZCmBj5r+KKJiTAVUdBFj7Sm9JtGczTX0dk2jjtBH0wbLOcFIWesQiwVAwE=",
      "checksum": "4ad90a2c2e08374f8ccec2b604915a0ab7e97fcca983b12a6857d20df3fca9c0",
      "created_at": "2022-09-14T01:19:12Z"
    },
    {
      "version": "1.0.1",
      "content_id": "QmQfiqgpEL7gWavVJ5r2JK17N516q9wWoL8eHjwq8zKozZ",
      "pointer_id": "f52b51ea3b48652b6c01892695b92c76c404a5efe8270a331e981a3b1f772b47",
      "signature": "2tBILxCWrZ8MrvXUIudC/0BSxJf8+gor6tJWiGxxN3A1e/gQKXrtFo4+CIjn4Atz8uUNQyrdzZtG6+/wPuPQbAA=",
      "checksum": "6fb6f92379c52eeb7f18d56c6fc745755588ebbccd5db0e157c9938daaf5e359",
      "created_at": "2022-09-14T01:19:17Z"
    },
    {
      "version": "2.0.0-alpha.1",
      "content_id": "QmbptdWzd7pzNbmTkGwtYRdQWYCmXYjQ6tJV9CkWkjD2V8",
      "pointer_id": "ed7cfb288b5b7dedaa4dd2e189e921d839cc832d39d13d8a2be87c6b340809fb",
      "signature": "krOfiqyqEJ4TYZTJikfnQdBYxqGwQv4EU/JKrt64eLVJnDiBYqSOVfH4h1bpc1ghrCb4S323UGDgrCytHc43swA=",
      "checksum": "58313c4525d2253048a7b7342bb63b4a914bd5ae2ee5eab9e22f35c8897b5db5",
      "created_at": "2022-09-14T01:19:30Z"
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

#### Parameters

* `:namespace`: The package namespace.
* `:package`: The package name.

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
  "pointer_id": "ed7cfb288b5b7dedaa4dd2e189e921d839cc832d39d13d8a2be87c6b340809fb",
  "signature": "krOfiqyqEJ4TYZTJikfnQdBYxqGwQv4EU/JKrt64eLVJnDiBYqSOVfH4h1bpc1ghrCb4S323UGDgrCytHc43swA=",
  "checksum": "58313c4525d2253048a7b7342bb63b4a914bd5ae2ee5eab9e22f35c8897b5db5",
  "created_at": "2022-09-14T01:19:30Z"
}
```

### Package version

```
GET /api/package/version?id=<package-id>
```

Get a specific version of a package.

#### Query

* `id`: Package identifier.

#### Response

See example response for latest version above.

### Yank version

```
POST /api/package/yank?id=<package-id>
```

Mark a specific version of a package as yanked.

The body should be a UTF-8 encoded string of the reason why the version was yanked; it may be the empty string.

If the version is already yanked a 409 CONFLICT response is returned.

#### Query

* `id`: Package identifier.

#### Headers

* `x-signature`: Signature of the bytes for the request body.

#### Response

200 if successful.

## Configuration

This section describes the server configuration; after making changes to the configuration you must restart the server for changes to take effect.

### Database

The default database is an in-memory [sqlite][] database; to configure a file on disc for the database:

```toml
[database]
url = "sqlite:ipfs_registry.db"
```

In the future we intend to support a postgres database driver too.

### Storage

Storage for packages is defined as an ordered set of layers.

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

Local filesystem storage can be configured using a file layer:

```toml
[storage]
layers = [
  { directory = "./objects" },
]
```

Relative paths are resolved from the directory containing the configuration file; the path must be a directory.

Note that all the downstream storage layers must be available for the service to work as intended; ie, requests must succeed across all storage layers for the server to return a success response.

### Registry

### Kind

Set the registry `kind` to determine how package data is extracted from package archives when they are published.

```toml
[registry]
kind = "cargo"
```

Supported registry kinds are:

* `npm`: Packages generated by [npm][] (default)
* `cargo`: [Crates][crates] generated by `cargo`.

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
[sqlite]: https://www.sqlite.org/
[semver]: https://semver.org/
[semver crate]: https://docs.rs/semver/
[npm]: https://www.npmjs.com/
[crates]: https://crates.io/
[unicode security mechanisms]: http://www.unicode.org/reports/tr39/
[unicode security crate]: https://docs.rs/unicode-security/
[confusables]: https://util.unicode.org/UnicodeJsps/confusables.jsp
[general security profile]: https://www.unicode.org/reports/tr39/#General_Security_Profile
[single script]: https://www.unicode.org/reports/tr39/#def-single-script
