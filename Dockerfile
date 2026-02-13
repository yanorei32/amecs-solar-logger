FROM rust:1.93.1-bookworm AS build-env
LABEL maintainer="yanorei32"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

WORKDIR /usr/src
COPY . /usr/src/amecs-solar-logger/
WORKDIR /usr/src/amecs-solar-logger
RUN cargo build --release && cargo install cargo-license && cargo license \
	--authors \
	--do-not-bundle \
	--avoid-dev-deps \
	--avoid-build-deps \
	--filter-platform "$(rustc -vV | sed -n 's|host: ||p')" \
	> CREDITS

FROM debian:bookworm-slim@sha256:98f4b71de414932439ac6ac690d7060df1f27161073c5036a7553723881bffbe

RUN apt-get update; \
	apt-get install -y --no-install-recommends \
		libssl3 ca-certificates; \
	apt-get clean;

WORKDIR /

COPY --chown=root:root --from=build-env \
	/usr/src/amecs-solar-logger/CREDITS \
	/usr/src/amecs-solar-logger/LICENSE \
	/usr/share/licenses/amecs-solar-logger/

COPY --chown=root:root --from=build-env \
	/usr/src/amecs-solar-logger/target/release/amecs-solar-logger \
	/usr/bin/amecs-solar-logger

CMD ["/usr/bin/amecs-solar-logger"]
