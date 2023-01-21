# Minne (Backend for the app)

## Environment Variables
- `MINNE_LOGGING_LEVEL` - The verbosity of the logging. Default: `info` (options: `trace`, `debug`, `info`, `warn`, `error`)
- `MINNE_DB_CONNECTION` - The connection string for the database (e.g. `postgres://postgres:postgres@localhost:5432/minne`)
- `MINNE_TOKEN_SIGNATURE_PSK` - The PSK used to sign the JWT tokens for the authentication process
- `MINNE_ACCESS_TOKEN_LIFETIME_IN_SECONDS` - The lifetime of the access token in seconds. Default: `300`
- `MINNE_REFRESH_TOKEN_LIFETIME_IN_SECONDS` - The lifetime of the refresh token in seconds. Default: `3600`
- `MINNE_ENABLE_USER_REGISTRATION` - Whether to enable user registration or leave it disabled. Default: `false`

## Create a new user
`curl --verbose http://127.0.0.1:5645/v1/user/create -H "Content-Type: application/json" -d @example_payloads/create_user.json`

## Get an authentication token
`curl --verbose http://127.0.0.1:5645/v1/auth/login -H "Content-Type: application/json" -d @example_payloads/login.json`