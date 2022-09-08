CREATE TABLE IF NOT EXISTS namespaces
(
    namespace_id          INTEGER PRIMARY KEY NOT NULL,
    name                  TEXT                NOT NULL UNIQUE,
    owner                 BLOB(20)            NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS publishers
(
    id          INTEGER PRIMARY KEY NOT NULL,
    address     BLOB(20)            NOT NULL UNIQUE
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
    name                  TEXT                NOT NULL UNIQUE,

    FOREIGN KEY (namespace_id) REFERENCES namespaces (namespace_id)
);

CREATE TABLE IF NOT EXISTS versions
(
    version_id            INTEGER PRIMARY KEY NOT NULL,
    package_id            INTEGER             NOT NULL,
    version               TEXT                NOT NULL UNIQUE,

    FOREIGN KEY (package_id) REFERENCES packages (package_id)
);
