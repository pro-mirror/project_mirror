# ビルドステージ
FROM rust:slim as builder

WORKDIR /app

# 依存関係のキャッシュ最適化のため、まずCargo.tomlとCargo.lockをコピー
COPY Cargo.toml Cargo.lock* ./

# ダミーソースでビルドして依存関係をキャッシュ
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 実際のソースコードをコピー
COPY src ./src

# リリースビルド（依存関係は既にキャッシュ済み）
RUN cargo build --release

# 実行ステージ（軽量なイメージ）
FROM debian:bookworm-slim

WORKDIR /app

# 必要なランタイム依存関係のインストール
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# ビルドステージからバイナリをコピー
COPY --from=builder /app/target/release/project-mirror-backend /app/project-mirror-backend

# 実行権限を付与し、バイナリの存在を確認
RUN chmod +x /app/project-mirror-backend && \
    ls -la /app/project-mirror-backend

# ポート公開
EXPOSE 8080

# アプリケーション実行
CMD ["/app/project-mirror-backend"]
