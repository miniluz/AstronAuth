-- Deploy astronauth:test to pg
-- requires: initial_migration

BEGIN;

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE TABLE "test" (
    "uuid" uuid DEFAULT gen_random_uuid() NOT NULL PRIMARY KEY,
    "created" TIMESTAMP WITH TIME ZONE NOT NULL default NOW()
);

COMMIT;
