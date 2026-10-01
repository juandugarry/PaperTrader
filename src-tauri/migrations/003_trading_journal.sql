CREATE TABLE journal_entries (
    execution_id INTEGER PRIMARY KEY REFERENCES executions(id),
    captured_with_fill INTEGER NOT NULL CHECK(captured_with_fill IN (0,1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE journal_revisions (
    id INTEGER PRIMARY KEY,
    execution_id INTEGER NOT NULL REFERENCES journal_entries(execution_id),
    version INTEGER NOT NULL CHECK(version > 0),
    thesis TEXT NOT NULL DEFAULT '', entry_trigger TEXT NOT NULL DEFAULT '',
    target_micros INTEGER CHECK(target_micros BETWEEN 1 AND 1000000000000000),
    stop_micros INTEGER CHECK(stop_micros BETWEEN 1 AND 1000000000000000),
    planned_risk_micros INTEGER CHECK(planned_risk_micros BETWEEN 0 AND 1000000000000000 AND planned_risk_micros % 10000=0),
    notes TEXT NOT NULL DEFAULT '', exit_reason TEXT NOT NULL DEFAULT '',
    followed_plan INTEGER CHECK(followed_plan IN (0,1)),
    went_well TEXT NOT NULL DEFAULT '', went_poorly TEXT NOT NULL DEFAULT '', would_change TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE(execution_id,version)
);
INSERT INTO journal_entries(execution_id,captured_with_fill) SELECT id,0 FROM executions;
INSERT INTO journal_revisions(execution_id,version) SELECT id,1 FROM executions;
CREATE TRIGGER journal_entries_no_update BEFORE UPDATE ON journal_entries
BEGIN SELECT RAISE(ABORT,'Journal links are immutable'); END;
CREATE TRIGGER journal_entries_no_delete BEFORE DELETE ON journal_entries
BEGIN SELECT RAISE(ABORT,'Journal links are immutable'); END;
CREATE TRIGGER journal_revisions_no_update BEFORE UPDATE ON journal_revisions
BEGIN SELECT RAISE(ABORT,'Journal revisions are immutable'); END;
CREATE TRIGGER journal_revisions_no_delete BEFORE DELETE ON journal_revisions
BEGIN SELECT RAISE(ABORT,'Journal revisions are immutable'); END;
