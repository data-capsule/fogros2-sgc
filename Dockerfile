# first build an image with rust, ros and cargo chef
FROM osrf/ros:humble-desktop AS chef
RUN apt update && apt install -y build-essential curl pkg-config libssl-dev protobuf-compiler clang
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cargo-chef --locked

RUN apt-get update && apt-get install -y build-essential curl pkg-config libssl-dev protobuf-compiler clang libssl-dev wget ros-humble-rmw-cyclonedds-cpp
# https://stackoverflow.com/questions/72378647/spl-token-error-while-loading-shared-libraries-libssl-so-1-1-cannot-open-shar
# Check here for latest file: http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/
RUN wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.20_amd64.deb 
RUN dpkg -i libssl1.1_1.1.1f-1ubuntu2.20_amd64.deb

WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder 

WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN . /opt/ros/humble/setup.sh && cargo chef cook --recipe-path recipe.json
# to run with release mode, uncomment the following line
# RUN . /opt/ros/humble/setup.sh cargo chef cook --release --recipe-path recipe.json

COPY . .
# generate crypto keys
WORKDIR /app/scripts
RUN bash ./generate_crypto.sh
# build app
WORKDIR /app
RUN cd /app/signaling && cargo build --release
RUN . /opt/ros/humble/setup.sh && cargo build 

# build the final image
FROM chef
WORKDIR /
COPY --from=builder  /app/target/release/sgc_signaling_server /signaling_server
COPY --from=builder /app/bench /fog_ws
COPY --from=builder /app/src /src 
COPY --from=builder /app/scripts /scripts
COPY --from=builder /app/target/debug/gdp-router /

CMD [ "source /opt/ros/rolling/setup.bash; cargo run", "router" ]
