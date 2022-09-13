CREATE TABLE IF NOT EXISTS namespaces
(
    namespace_id          INTEGER PRIMARY KEY NOT NULL,
    publisher_id          INTEGER             NOT NULL,
    created_at            TEXT                NOT NULL,
    name                  TEXT                NOT NULL UNIQUE,

    FOREIGN KEY (publisher_id) REFERENCES publishers (publisher_id)
);

CREATE TABLE IF NOT EXISTS publishers
(
    publisher_id          INTEGER PRIMARY KEY NOT NULL,
    created_at            TEXT                NOT NULL,
    address               BLOB(20)            NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS namespace_publishers
(
    namespace_id          INTEGER             NOT NULL,
    publisher_id          INTEGER             NOT NULL,

    FOREIGN KEY (namespace_id) REFERENCES namespaces (namespace_id),
    FOREIGN KEY (publisher_id) REFERENCES publishers (publisher_id)
);

CREATE TABLE IF NOT EXISTS packages
(
    package_id            INTEGER PRIMARY KEY NOT NULL,
    namespace_id          INTEGER             NOT NULL,
    created_at            TEXT                NOT NULL,
    name                  TEXT                NOT NULL,

    FOREIGN KEY (namespace_id) REFERENCES namespaces (namespace_id)
);

CREATE TABLE IF NOT EXISTS versions
(
    version_id            INTEGER PRIMARY KEY NOT NULL,
    publisher_id          INTEGER             NOT NULL,
    package_id            INTEGER             NOT NULL,
    created_at            TEXT                NOT NULL,
    -- Semver
    major                 INTEGER             NOT NULL,
    minor                 INTEGER             NOT NULL,
    patch                 INTEGER             NOT NULL,
    pre                   TEXT,
    build                 TEXT,

    -- IPFS content identifier
    content_id            TEXT,
    -- Pointer identifier
    pointer_id            TEXT                NOT NULL,
    -- Signature using the publisher's private key
    signature             BLOB(65)            NOT NULL,
    -- SHA-256 checksum or the package archive
    checksum              BLOB(32)            NOT NULL,
    -- Package meta data as JSON (eg: package.json)
    package               TEXT                NOT NULL,

    -- Yanked message when not NULL
    yanked                TEXT,

    FOREIGN KEY (publisher_id) REFERENCES publishers (publisher_id),
    FOREIGN KEY (package_id) REFERENCES packages (package_id)
);
