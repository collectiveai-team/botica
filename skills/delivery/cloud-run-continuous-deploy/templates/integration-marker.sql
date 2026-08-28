-- One-time, idempotent initializer for the dedicated integration database.
--
-- Run by an operator, never by a workflow: an automated marker write would let a
-- misconfigured deploy mark production as disposable and then reset it.
--
-- The marker lives outside the schema the reset drops. Placed inside it, the
-- reset would destroy its own authorisation on the first run.
CREATE SCHEMA IF NOT EXISTS review_control;

CREATE TABLE IF NOT EXISTS review_control.environment_marker (
    environment text PRIMARY KEY
);

INSERT INTO review_control.environment_marker (environment)
VALUES ('integration')
ON CONFLICT DO NOTHING;
