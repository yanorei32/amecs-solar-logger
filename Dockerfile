FROM rust:1.86.0-bookworm AS build-env
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

FROM debian:bookworm-slim@sha256:4b44499bc2a6c78d726f3b281e6798009c0ae1f034b0bfaf6a227147dcff928b

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
