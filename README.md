# Project Mirror

**デジタル分身（Digital Twin）- あなたの価値観をブレずに保持する対話システム**

## 🎯 プロジェクト概要

Project Mirrorは、ユーザーの「他人への思いやりと感謝」という価値観を保持し、感情の波を受け止めながら本来の姿へ導くデジタル分身です。

### Genesis Data（初期記憶）
「真っ直ぐに愛を伝え続けてくれる伴侶への感謝」

## 🏗 アーキテクチャ

```
┌─────────────────┐
│   Frontend      │  Expo / React Native
│   (Mobile App)  │  - Home (対話)
└────────┬────────┘  - Constellation (グラフ可視化)
         │           - Chronicle (アーカイブ)
         │ REST API
┌────────▼────────┐
│   Backend       │  Rust / Axum
│   (Railway)     │  - Chat API
└────┬───────┬────┘  - Insights API
     │       │
     │       └─────────────┐
     │                     │
┌────▼─────┐         ┌────▼─────┐
│  Neo4j   │         │  Qdrant  │
│ (Graph)  │         │ (Vector) │
└──────────┘         └──────────┘
     │                     │
     └──────────┬──────────┘
                │
         ┌──────▼──────┐
         │   OpenAI    │
         │  gpt-4o-mini│
         └─────────────┘
```

## 🛠 技術スタック

### Backend
- **言語**: Rust
- **フレームワーク**: Axum
- **Graph DB**: Neo4j
- **Vector DB**: Qdrant
- **LLM**: OpenAI gpt-4o-mini
- **デプロイ**: Railway

### Frontend
- **フレームワーク**: Expo / React Native
- **ナビゲーション**: React Navigation
- **アニメーション**: react-native-reanimated
- **状態管理**: React Query + Context API

## 📁 プロジェクト構造

```
project_mirror/
├── backend/              # Rustバックエンド
│   ├── src/
│   │   ├── main.rs
│   │   ├── api/         # APIエンドポイント
│   │   ├── db/          # DB接続層
│   │   ├── llm/         # LLM統合
│   │   └── models/      # データモデル
│   ├── Cargo.toml
│   └── .env.example
├── frontend/             # Expo フロントエンド
│   ├── src/
│   │   ├── screens/     # Home, Constellation, Chronicle
│   │   ├── components/  # 再利用可能なコンポーネント
│   │   ├── navigation/  # ナビゲーション設定
│   │   └── api/         # APIクライアント
│   ├── app.json
│   ├── package.json
│   └── .env.example
└── docs/                # ドキュメント
    ├── バックエンド実装ガイドライン.md
    └── フロントエンド実装ガイドライン.md
```

## 🚀 セットアップ

### 環境変数の設定

#### Backend (.env)
```env
OPENAI_API_KEY=sk-xxxx...
QDRANT_URL=https://xxxx.qdrant.tech
QDRANT_API_KEY=xxxx...
NEO4J_URI=neo4j+s://xxxx.databases.neo4j.io
NEO4J_USER=neo4j
NEO4J_PASSWORD=xxxx...
```

#### Frontend (.env)
```env
EXPO_PUBLIC_API_BASE_URL=http://localhost:8080/api/v1
```

### バックエンドの起動

```bash
cd backend
cargo run
```

### フロントエンドの起動

```bash
cd frontend
npm install
npm start
```

## 📚 ドキュメント

- [バックエンド実装ガイドライン](./docs/バックエンド実装ガイドライン.md)
- [フロントエンド実装ガイドライン](./docs/フロントエンド実装ガイドライン.md)

## 🎨 デザイン原則

1. **静寂 (Stillness)**: 情報を詰め込まず、余白を贅沢に使う
2. **共鳴 (Resonance)**: ユーザーの感情に同調し、光の揺らぎや振動で応える
3. **鏡 (Mirror)**: ユーザーの過去の「善性（感謝）」を美しく、誇らしく映し出す

## 📄 ライセンス

Private Project
