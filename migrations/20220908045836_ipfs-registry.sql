CREATE TABLE IF NOT EXISTS namespaces
(
    namespace_id          INTEGER PRIMARY KEY NOT NULL,
    publisher_id          INTEGER             NOT NULL,
    created_at            TEXT                NOT NULL,
    name                  TEXT                NOT NULL UNIQUE,
    skeleton              TEXT                NOT NULL UNIQUE,

    FOREIGN KEY (publisher_id) REFERENCES publishers (publisher_id)
);
CREATE INDEX IF NOT EXISTS namespaces_name_idx ON namespaces (name);

CREATE TABLE IF NOT EXISTS publishers
(
    publisher_id          INTEGER PRIMARY KEY NOT NULL,
    created_at            TEXT                NOT NULL,
    address               BLOB(20)            NOT NULL UNIQUE
);
CREATE INDEX IF NOT EXISTS publishers_address_idx ON publishers (address);

CREATE TABLE IF NOT EXISTS namespace_publishers
(
    namespace_id          INTEGER             NOT NULL,
    publisher_id          INTEGER             NOT NULL,
    administrator         BOOLEAN             NOT NULL
                                                CHECK (administrator IN (0, 1))
                                                DEFAULT 0,

    FOREIGN KEY (namespace_id) REFERENCES namespaces (namespace_id),
    FOREIGN KEY (publisher_id) REFERENCES publishers (publisher_id)
);

CREATE TABLE IF NOT EXISTS publisher_restrictions
(
    publisher_id          INTEGER             NOT NULL,
    package_id            INTEGER             NOT NULL,

    FOREIGN KEY (publisher_id) REFERENCES publishers (publisher_id),
    FOREIGN KEY (package_id) REFERENCES packages (package_id)
);

CREATE TABLE IF NOT EXISTS packages
(
    package_id            INTEGER PRIMARY KEY NOT NULL,
    namespace_id          INTEGER             NOT NULL,
    created_at            TEXT                NOT NULL,
    name                  TEXT                NOT NULL,
    skeleton              TEXT                NOT NULL,

    FOREIGN KEY (namespace_id) REFERENCES namespaces (namespace_id)
);
CREATE INDEX IF NOT EXISTS packages_name_idx ON packages(name);

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
CREATE INDEX IF NOT EXISTS versions_semver_idx
  ON versions(major, minor, patch, pre, build);
CREATE INDEX IF NOT EXISTS versions_content_id_idx ON versions(content_id);
CREATE INDEX IF NOT EXISTS versions_pointer_id_idx ON versions(pointer_id);
