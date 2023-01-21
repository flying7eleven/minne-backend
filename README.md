# Minne (Backend for the app)

## Environment Variables
- `MINNE_LOGGING_LEVEL` - The verbosity of the logging. Default: `info` (options: `trace`, `debug`, `info`, `warn`, `error`)
- `MINNE_DB_CONNECTION` - The connection string for the database (e.g. `postgres://postgres:postgres@localhost:5432/minne`)
- `MINNE_TOKEN_SIGNATURE_PSK` - The PSK used to sign the JWT tokens for the authentication process
- `MINNE_ACCESS_TOKEN_LIFETIME_IN_SECONDS` - The lifetime of the access token in seconds. Default: `300`
- `MINNE_REFRESH_TOKEN_LIFETIME_IN_SECONDS` - The lifetime of the refresh token in seconds. Default: `3600`
- `MINNE_ENABLE_USER_REGISTRATION` - Whether or not to enable user registration. Default: `false`