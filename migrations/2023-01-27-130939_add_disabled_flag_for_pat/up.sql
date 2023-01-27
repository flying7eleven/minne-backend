ALTER TABLE personal_access_tokens
    ADD COLUMN disabled boolean NOT NULL DEFAULT false;