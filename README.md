# Project Mirror

**デジタル分身（Digital Twin）- ユーザーの価値観をブレずに保持する対話システム**

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
2. Embedding化（OpenAI text-embedding-3-small）
   ↓
3. 並列検索
   ├─→ [Qdrant] セマンティック検索（類似度 > 0.3）
   │   └─→ 関連parent_id取得（意味的類似性ベース）
   └─→ [Neo4j] コアバリュー取得（重み付き上位5件）
   ↓
4. [PostgreSQL] 現在のアクティブセッション取得
   └─→ 現在セッションを除外（自己参照防止）
   ↓
5. [PostgreSQL] セッション内容取得（全ターン）
   └─→ Qdrantから取得したparent_idに基づく
   ↓
6. 動的プロンプト構築
   ├─→ システムプロンプト（ベース）
   ├─→ コアバリュー注入（重要度順）
   └─→ 過去セッション注入（関連度高い上位5件）
   ↓
7. [OpenAI] レスポンス生成（gpt-4o-mini, max_tokens=1000）
   ├─→ finish_reason監視（truncation検出）
   └─→ 応答の完全性を保証
   ↓
8. ユーザーに返答
```

**最適化ポイント（v1.1）:**
- エンティティ抽出処理を削除し、Qdrantのセマンティック検索に一本化
- 応答生成前の処理時間を約30-40%短縮（1.8-4.8秒 → 0.7-2.0秒）
- コアバリューは引き続き動的注入し、AIの分身としての品質を維持

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

### 環境変数の設定

#### Backend (.env for local development)
```env
OPENAI_API_KEY=sk-xxxx...
QDRANT_URL=https://xxxx.qdrant.tech
QDRANT_API_KEY=xxxx...
NEO4J_URI=neo4j+s://xxxx.databases.neo4j.io
NEO4J_USER=neo4j
NEO4J_PASSWORD=xxxx...
NEO4J_DATABASE=neo4j
DATABASE_URL=postgresql://user:pass@localhost:5432/mirror
HOST=0.0.0.0
PORT=8080
RUST_LOG=info
```

**Railway本番環境:**
環境変数は`.env`ではなく、**Railway Dashboardから設定**してください。
`DATABASE_URL`には`${{Postgres.DATABASE_PRIVATE_URL}}`を使用することで、プライベートネットワーク接続となりコストを削減できます。

#### Frontend (.env)
```env
EXPO_PUBLIC_API_BASE_URL=http://localhost:8080/api/v1
```

### ローカル開発

#### バックエンドの起動
```bash
cd backend
cargo run または
RUST_LOG=debug cargo run
```

#### フロントエンドの起動
```bash
cd frontend
npm install
npm start または
npx expo start
```

## 本番デプロイ

### デプロイ構成

#### バックエンド
- **プラットフォーム**: [Railway](https://railway.app)
- **リージョン**: us-east4-eqdc4a
- **ビルド**: Nixpacks（Rust）
- **デプロイ方法**: GitHub連携による自動デプロイ
- **ヘルスチェック**: `/health` (timeout: 300s)
- **再起動ポリシー**: ON_FAILURE (max 10 retries)
- **設定ファイル**: `backend/railway.toml`

**重要な設定:**
- 環境変数はすべてRailway Dashboardで管理
- `DATABASE_URL`は`${{Postgres.DATABASE_PRIVATE_URL}}`を使用（コスト削減）
- `railway.toml`にはビルド設定のみを含める

#### フロントエンド
- **プラットフォーム**: [Expo Application Services (EAS)](https://expo.dev)
- **ターゲット**: Android APK / AAB
- **配信**: Google Play Console（Internal/Production）
- **デプロイ方法**: 
（プレビュービルド Android APK）
eas build --profile preview --platform android
（本番ビルド AAB for Google Play）
eas build --profile production --platform android
- **設定ファイル**: `frontend/eas.json`, `frontend/app.json`

## 将来の機能

### Keycloak マルチユーザー認証（予定）
現在、シングルユーザー向けですが、将来的にKeycloakを使ったマルチユーザー対応を予定しています。

- OAuth2/OIDC認証フロー
- JWT トークンベースのAPI認証
- ユーザー管理とロールベースアクセス制御

実装スケルトン:
- Backend: `backend/src/auth_keycloak.rs`
- Frontend: `frontend/src/auth/keycloak.ts`

## ドキュメント

### 開発ガイド
- [バックエンド実装ガイドライン](./docs/バックエンド実装ガイドライン.md)
- [フロントエンド実装ガイドライン](./docs/フロントエンド実装ガイドライン.md)

## ライセンス

Private Project
