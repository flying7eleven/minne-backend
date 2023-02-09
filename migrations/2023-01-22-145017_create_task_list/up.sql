CREATE TABLE tasks
(
    id         SERIAL PRIMARY KEY,
    title      VARCHAR(255) NOT NULL,
    owner      SERIAL       NOT NULL,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

ALTER TABLE tasks
    ADD CONSTRAINT tasks_owner_fkey FOREIGN KEY (owner) REFERENCES users (id);