FROM osrf/ros:rolling-desktop

# Install env
RUN apt update && apt install -y build-essential curl pkg-config libssl-dev protobuf-compiler clang
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy project source code
COPY . .

# Build the project
RUN cargo build --release
