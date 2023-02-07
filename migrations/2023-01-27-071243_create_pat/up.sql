CREATE TABLE IF NOT EXISTS personal_access_tokens
(
    id         serial PRIMARY KEY,
    name       varchar(255) NOT NULL, -- name which describes the token
    token      varchar(36)  NOT NULL, -- will ne an UUID
    secret     varchar(36)  NOT NULL,-- will ne an UUID
    user_id    int          NOT NULL,
    created_at timestamp    NOT NULL DEFAULT NOW(),
    updated_at timestamp    NOT NULL DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users (id)
);