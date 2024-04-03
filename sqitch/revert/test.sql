-- Revert astronauth:test from pg

BEGIN;

DROP EXTENSION IF EXISTS pgcrypto;
DROP TABLE IF EXISTS "test";

COMMIT;
