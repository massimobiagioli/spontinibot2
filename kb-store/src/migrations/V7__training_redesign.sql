ALTER TABLE training_session ADD COLUMN notes TEXT;

ALTER TABLE training_message ADD COLUMN expected_answer TEXT;
ALTER TABLE training_message ADD COLUMN execution_time_ms INTEGER;
ALTER TABLE training_message ADD COLUMN source TEXT NOT NULL DEFAULT 'chat';
