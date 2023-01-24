FROM osrf/ros:rolling-desktop

# Install env
RUN apt update && apt install -y build-essential curl pkg-config libssl-dev protobuf-compiler clang
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

# Copy project source code
COPY . .

# Build the project
RUN cargo build --release
