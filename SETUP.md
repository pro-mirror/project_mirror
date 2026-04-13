# Project Mirror - セットアップガイド

## 📋 前提条件

### 必要なツール
- **Rust**: v1.70以上 ([rustup.rs](https://rustup.rs/))
- **Node.js**: v18以上 ([nodejs.org](https://nodejs.org/))
- **Expo CLI**: `npm install -g expo-cli`
- **Docker** (オプション): ローカル開発用

### 必要なアカウント & APIキー
1. **OpenAI**: [platform.openai.com](https://platform.openai.com/)
2. **Qdrant Cloud**: [cloud.qdrant.io](https://cloud.qdrant.io/)
3. **Neo4j Aura**: [neo4j.com/cloud/aura](https://neo4j.com/cloud/aura/)

## 🚀 バックエンドのセットアップ

### 1. 環境変数の設定
```bash
cd backend
cp .env.example .env
# .envファイルを編集して、APIキーとDB接続情報を入力
```

### 2. 依存関係のインストールとビルド
```bash
cargo build
```

### 3. データベースの初期化
Neo4jとQdrantのコレクション/スキーマは、初回起動時に自動的に作成されます。

### 4. サーバーの起動
```bash
cargo run
```

サーバーは `http://localhost:8080` で起動します。

### ヘルスチェック
```bash
curl http://localhost:8080/health
```

## 📱 フロントエンドのセットアップ

### 1. 環境変数の設定
```bash
cd frontend
cp .env.example .env
# EXPO_PUBLIC_API_BASE_URL を編集（開発時は http://localhost:8080/api/v1）
```

### 2. 依存関係のインストール
```bash
npm install
```

### 3. 開発サーバーの起動
```bash
npm start
```

Expoメニューが表示されます:
- **i**: iOS Simulatorで開く
- **a**: Android Emulatorで開く
- **w**: Webブラウザで開く

### 実機での動作確認
1. **Expo Go** アプリをスマートフォンにインストール
2. QRコードをスキャンしてアプリを起動

## 🧪 動作確認

### 1. バックエンド
```bash
# Health Check
curl http://localhost:8080/health

# Chat API
curl -X POST http://localhost:8080/api/v1/chat/message \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test","text":"今日も妻が美味しいご飯を作ってくれた"}'

# Graph API
curl http://localhost:8080/api/v1/insights/graph
```

### 2. フロントエンド
1. Home画面でメッセージを送信
2. Constellation画面でグラフを確認
3. Chronicle画面でアーカイブを確認

## 📂 プロジェクト構造

```
project_mirror/
├── backend/              # Rustバックエンド
│   ├── src/
│   │   ├── main.rs      # エントリーポイント
│   │   ├── config.rs    # 環境変数設定
│   │   ├── models.rs    # データモデル
│   │   ├── api/         # APIエンドポイント
│   │   ├── db/          # Neo4j & Qdrant
│   │   └── llm/         # OpenAI統合
│   ├── Cargo.toml
│   └── .env
├── frontend/             # Expo フロントエンド
│   ├── src/
│   │   ├── screens/     # 3つのメイン画面
│   │   ├── components/  # 再利用可能なコンポーネント
│   │   ├── api/         # APIクライアント
│   │   └── theme.ts     # デザインテーマ
│   ├── App.tsx
│   ├── package.json
│   └── .env
└── docs/                # ドキュメント
```

## 🔧 トラブルシューティング

### Rust ビルドエラー
```bash
# 最新版にアップデート
rustup update

# キャッシュをクリア
cargo clean
cargo build
```

### Expo エラー
```bash
# キャッシュをクリア
npm start --clear

# node_modulesを再インストール
rm -rf node_modules
npm install
```

### データベース接続エラー
- Neo4j/Qdrantの接続情報が正しいか確認
- ファイアウォール設定を確認
- APIキーの有効期限を確認

## 📚 次のステップ

1. **データベーススキーマの完成**: Neo4jの制約とQdrantのコレクション設定
2. **LLM統合の強化**: Structured Outputs、RAG、感情分析
3. **フロントエンドの洗練**: アニメーション、音声入力、Haptics
4. **デプロイ**: Railway (Backend) + Expo EAS (Frontend)

## 🤝 コントリビューション

このプロジェクトはプライベートプロジェクトです。

## 📄 ライセンス

Private Project
