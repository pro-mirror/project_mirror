# Project Mirror

**デジタル分身（Digital Twin）- あなたの価値観をブレずに保持する対話システム**

## プロジェクト概要

Project Mirrorは、AIとユーザーが対話するシンプルなアプリです。AIは会話内容からユーザーが重要視している価値観を抽出します。同じ価値観を持った分身として、ブレることのないAIとの会話を通して、ユーザーが内なる価値観や本来の姿に気づくことを目的としています。

## アーキテクチャ

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
└─┬──┬───┬───┬───┘  - Insights API
  │  │   │   │
  │  │   │   └─────────────────┐
  │  │   └─────────┐           │
  │  │             │           │
┌─▼──▼───┐   ┌────▼─────┐ ┌──▼──────┐
│Postgres│   │  Qdrant  │ │  Neo4j  │
│(Session│   │ (Vector) │ │ (Graph) │
│ & Turn)│   └──────────┘ └─────────┘
└────────┘         │            │
                   │            │
                   └─────┬──────┘
                         │
                  ┌──────▼──────┐
                  │   OpenAI    │
                  │  gpt-4o-mini│
                  └─────────────┘
```

## 技術スタック

### Backend
- **言語**: Rust
- **フレームワーク**: Axum
- **RDB**: PostgreSQL（セッション・ターン管理）
- **Graph DB**: Neo4j（コアバリューと関係性）
- **Vector DB**: Qdrant（セマンティック検索）
- **LLM**: OpenAI gpt-4o-mini
- **デプロイ**: Railway

### Frontend
- **フレームワーク**: Expo / React Native
- **ナビゲーション**: React Navigation
- **アニメーション**: react-native-reanimated
- **状態管理**: React Query + Context API

## 処理フロー

### ユーザー入力からレスポンス生成まで

```
1. ユーザー入力
   ↓
2. Embedding化（OpenAI）
   ↓
3. 並列検索
   ├─→ [Qdrant] セマンティック検索（類似度 > 0.3）
   │   └─→ 関連parent_id取得
   ├─→ [Neo4j] コアバリュー取得（重み付き上位5件）
   └─→ [LLM] エンティティ抽出（人名・キーワード）
       └─→ [Neo4j] エンティティベースでparent_id検索
   ↓
4. parent_id統合・重複排除
   ├─→ [PostgreSQL] 現在のアクティブセッション取得
   └─→ 現在セッションを除外（自己参照防止）
   ↓
5. [PostgreSQL] セッション内容取得（全ターン）
   ↓
6. 動的プロンプト構築
   ├─→ システムプロンプト（ベース）
   ├─→ コアバリュー注入（重要度順）
   └─→ 過去セッション注入（関連度高い上位5件）
   ↓
7. [OpenAI] レスポンス生成
   ↓
8. ユーザーに返答
```

### 記憶の保存フロー（バックグラウンド処理）

```
1. [PostgreSQL] アクティブセッション取得/作成
   └─→ 10分以内に更新されたセッションを再利用
   └─→ なければ新規セッション作成
   ↓
2. [PostgreSQL] ターン追加
   └─→ 会話の1往復（user_text + reply_text）を保存
   └─→ turn_count自動インクリメント
   ↓
3. [Qdrant] sub-chunk embedding保存
   └─→ parent_id（セッションID）と紐付け
   ↓
4. [LLM] コアバリュー抽出
   └─→ 会話からユーザーの価値観を抽出
   ↓
5. [Neo4j] グラフ保存
   ├─→ User ノード
   ├─→ Episode ノード（= セッション）
   ├─→ CoreValue ノード（価値観）
   ├─→ Person ノード（登場人物）
   └─→ 関係性（HAS, HOLDS, RELATED_TO）
```

### データベースの役割分担

| データベース | 主な役割 | 保存データ |
|------------|---------|----------|
| **PostgreSQL** | セッション・ターン管理 | - parent_episodes（セッション単位）<br>- sub_chunks（ターン単位）<br>- 会話全文テキスト |
| **Qdrant** | セマンティック検索 | - sub-chunk embedding（768次元）<br>- parent_id（セッションID）とのマッピング |
| **Neo4j** | 関係性グラフ | - User, Episode, CoreValue, Person<br>- 重み付き関係性（HAS, HOLDS, RELATED_TO）<br>- 時系列メタデータ |

### 動的プロンプト注入の仕組み

1. **ベースプロンプト**: ユーザーの分身としての役割定義
2. **コアバリュー注入**: Neo4jから取得した重要な価値観を動的に追加
   ```
   【現在焦点を当てているコアバリュー】
   - **感謝の気持ち** (重要度: 0.95)
     背景: 妻への日々の感謝を大切にしている
   ```
3. **過去セッション注入**: PostgreSQLから取得した関連会話を追加
   ```
   【現在の話題の対象に関する過去の記憶】
   - セッション（3ターン）:
     User: 今日も妻が美味しいご飯を作ってくれた
     AI: それは素敵ですね...
   ```

この動的注入により、AIは常に最新のコンテキストで応答します。

## セットアップ

詳細なセットアップ手順は [SETUP.md](./SETUP.md) を参照してください。

### 環境変数の設定

#### Backend (.env)
```env
OPENAI_API_KEY=sk-xxxx...
QDRANT_URL=https://xxxx.qdrant.tech
QDRANT_API_KEY=xxxx...
NEO4J_URI=neo4j+s://xxxx.databases.neo4j.io
NEO4J_USER=neo4j
NEO4J_PASSWORD=xxxx...
DATABASE_PUBLIC_URL=postgres://user:pass@host:port/db
```

#### Frontend (.env)
```env
EXPO_PUBLIC_API_BASE_URL=http://localhost:8080/api/v1
```

### ローカル開発

#### バックエンドの起動
```bash
cd backend
cargo run
```

#### フロントエンドの起動
```bash
cd frontend
npm install
npm start
```

## 本番デプロイ

### クイックスタート
5-10分で本番環境にデプロイ可能です。[QUICKSTART_DEPLOY.md](./QUICKSTART_DEPLOY.md) を参照してください。

### デプロイ構成

#### バックエンド
- **プラットフォーム**: [Railway](https://railway.app)
- **コンテナ**: Docker（マルチステージビルド）
- **デプロイ方法**: GitHub連携による自動デプロイ
- **設定ファイル**: `backend/Dockerfile`, `backend/railway.toml`

#### フロントエンド
- **プラットフォーム**: [Expo Application Services (EAS)](https://expo.dev)
- **ターゲット**: Android APK / AAB
- **配信**: Google Play Console（Internal/Production）
- **設定ファイル**: `frontend/eas.json`, `frontend/app.json`

### 詳細ドキュメント
- [DEPLOYMENT.md](./DEPLOYMENT.md) - 詳細なデプロイ手順と構成
- [QUICKSTART_DEPLOY.md](./QUICKSTART_DEPLOY.md) - 5分で始めるデプロイガイド

## 将来の機能

### Keycloak マルチユーザー認証（予定）
現在、シングルユーザー向けですが、将来的にKeycloakを使ったマルチユーザー対応を予定しています。

- OAuth2/OIDC認証フロー
- JWT トークンベースのAPI認証
- ユーザー管理とロールベースアクセス制御

実装スケルトン:
- Backend: `backend/src/auth_keycloak.rs`
- Frontend: `frontend/src/auth/keycloak.ts`

詳細は [DEPLOYMENT.md](./DEPLOYMENT.md) の「Keycloak統合」セクションを参照してください。

## ドキュメント

### 開発ガイド
- [SETUP.md](./SETUP.md) - ローカル開発環境のセットアップ
- [バックエンド実装ガイドライン](./docs/バックエンド実装ガイドライン.md)
- [フロントエンド実装ガイドライン](./docs/フロントエンド実装ガイドライン.md)

### デプロイガイド
- [QUICKSTART_DEPLOY.md](./QUICKSTART_DEPLOY.md) - 5分で始めるデプロイ
- [DEPLOYMENT.md](./DEPLOYMENT.md) - 詳細なデプロイ手順とKeycloak統合

## ライセンス

Private Project
