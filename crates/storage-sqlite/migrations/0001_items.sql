CREATE TABLE items (
    id         TEXT    PRIMARY KEY NOT NULL,
    name       TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE INDEX items_created_at ON items (created_at, id);
