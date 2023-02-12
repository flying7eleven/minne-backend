# Minne (Backend for the app)
TODO

## Development
## Building a container
Build the backend container by going into the root directory of the repository and typing the following command:
`docker buildx build -f Dockerfile -t minne-backend:local .`

### Create a new user
`curl --verbose http://127.0.0.1:5842/v1/user/create -H "Content-Type: application/json" -d @example_payloads/create_user.json`

### Get an authentication token (and store it in `access_token.tmp`)
`echo -n "Authorization: Bearer " > access_token.tmp && curl --silent http://127.0.0.1:5842/v1/auth/login -H "Content-Type: application/json" -d @example_payloads/login.json | grep -oP '(?<=accessToken":")[^"]*' >> access_token.tmp`

### Use the stored access token and create a new task for the user who is logged in (simple)
`curl --verbose http://127.0.0.1:5842/v1/task -H "Content-Type: application/json" -H @access_token.tmp --data "{\"title\": \"Some new task\"}"`

### Use the stored access token and create a new task for the user who is logged in (extended)
`curl --verbose http://127.0.0.1:5842/v1/task -H "Content-Type: application/json" -H @access_token.tmp --data "{\"title\": \"Some new task\",\"created_at\":\"2023-02-09T14:58:02Z\",\"updated_at\":\"2023-02-09T14:58:02Z\"}"`

### Use the stored access token to delete an own task (with the id 1)
`curl --verbose -XDELETE http://127.0.0.1:5842/v1/task/1 -H "Content-Type: application/json" -H @access_token.tmp`

### Use the stored access token to fetch all tasks-ids of the logged-in user
`curl --verbose http://127.0.0.1:5842/v1/task/list -H @access_token.tmp`

### Create a new Personal Access Token (PAT) for the logged-in user
`echo -n "Authorization: PAT " > pat_token.tmp && curl --verbose http://127.0.0.1:5842/v1/auth/pat -H "Content-Type: application/json" -H @access_token.tmp --data "{\"name\": \"A descriptive name for the PAT\"}" | grep -oP '(?<=token":")[^"]*|(?<=secret":")[^"]*' | sed -z 's/\n/:/' >> pat_token.tmp`

### Permanently disable a Personal Access Token (PAT)
`curl --verbose http://127.0.0.1:5842/v1/auth/pat -H "Content-Type: application/json" -H @pat_token.tmp -XDELETE`

**Note**: To be able to use this call, you have to use the PAT you want to disable as the access token.

### Get a task by its id with a users Personal Access Token (PAT)
`curl --verbose http://127.0.0.1:5842/v1/task/4 -H "Content-Type: application/json" -H @pat_token.tmp`

## Environment Variables
- `MINNE_LOGGING_LEVEL` - The verbosity of the logging. Default: `info` (options: `trace`, `debug`, `info`, `warn`, `error`)
- `MINNE_DB_CONNECTION` - The connection string for the database (e.g. `postgres://postgres:postgres@localhost:5432/minne`)
- `MINNE_TOKEN_SIGNATURE_PSK` - The PSK used to sign the JWT tokens for the authentication process
- `MINNE_ACCESS_TOKEN_LIFETIME_IN_SECONDS` - The lifetime of the access token in seconds. Default: `300`
- `MINNE_REFRESH_TOKEN_LIFETIME_IN_SECONDS` - The lifetime of the refresh token in seconds. Default: `3600`
- `MINNE_ENABLE_USER_REGISTRATION` - Whether to enable user registration or leave it disabled. Default: `false`