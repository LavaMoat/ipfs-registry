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
    version               TEXT                NOT NULL UNIQUE,
    -- Package meta data JSON
    package               TEXT                NOT NULL,
    -- IPFS content identifier or key path
    content_id            TEXT                NOT NULL,

    FOREIGN KEY (publisher_id) REFERENCES publishers (publisher_id),
    FOREIGN KEY (package_id) REFERENCES packages (package_id)
);
