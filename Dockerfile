# use the same platform for building as we use for running the server
FROM debian:bullseye AS build

# ensure we have rust installed in the appropiate version
RUN apt update && \
    apt install -y curl build-essential libpq-dev && \
    curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain stable -y
ENV PATH=/root/.cargo/bin:$PATH

# copy the source files and build the backend in release-mode
COPY . /usr/src/app
WORKDIR /usr/src/app
RUN git config --global --add safe.directory /usr/src/app
RUN cargo build --release

# configure the acutal container for running the backend
FROM debian:bullseye

# install the required dependencies
RUN apt update && \
    apt install -y libpq5 curl

# copy the files for running the container
COPY --from=build --chown=1001 /usr/src/app/target/release/minne-backend /usr/local/bin/minne-backend

# set the work dir for the backend
WORKDIR /usr/local/bin

# configure the user(-id) for the running process
USER 1001

# configure the default values for the possible environment variables
ENV MINNE_LOGGING_LEVEL=info
ENV MINNE_DB_CONNECTION=postgres://minne:debuguser@minne_database:5432/minne
ENV MINNE_TOKEN_SIGNATURE_PSK=default_psk
ENV MINNE_ACCESS_TOKEN_LIFETIME_IN_SECONDS=300
ENV MINNE_REFRESH_TOKEN_LIFETIME_IN_SECONDS=3600
ENV MINNE_ENABLE_USER_REGISTRATION=false

# expose the backend port
EXPOSE 5842/tcp

# startup the backend
CMD exec /usr/local/bin/minne-backend