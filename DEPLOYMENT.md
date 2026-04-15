# 本番デプロイガイド
最終更新: 2026-04-15

## 🎯 デプロイ構成

### バックエンド
- **プラットフォーム**: Railway
- **言語**: Rust (Axum)
- **認証**: Keycloak（将来実装予定）
- **URL**: https://your-app.railway.app

### フロントエンド
- **プラットフォーム**: Expo Application Services (EAS)
- **ターゲット**: Android
- **配信**: Google Play Store（Internal Testing）

---

## 📋 デプロイ前チェックリスト

### 共通
- [ ] ソースコードがGitHubにプッシュ済み
- [ ] 機能テストが全てパス
- [ ] ログレベルを本番用に設定（`INFO`または`WARN`）

### バックエンド
- [ ] OpenAI APIキーが本番用で有効
- [ ] Qdrant Cloudクラスタが作成済み
- [ ] Neo4j Auraデータベースが作成済み
- [ ] 全てのAPIエンドポイントが正常動作確認済み

### フロントエンド
- [ ] Expo アカウント作成済み
- [ ] EAS CLIインストール済み（`npm install -g eas-cli`）
- [ ] `app.json`の`owner`と`extra.eas.projectId`を更新
- [ ] 本番APIのURLを`eas.json`に設定

---

## 🚀 バックエンド デプロイ手順（Railway）

### 1. Railwayアカウント設定
```bash
# Railway CLIインストール（オプション）
npm i -g @railway/cli

# ログイン
railway login
```

### 2. プロジェクト作成
1. [Railway Dashboard](https://railway.app/dashboard) にアクセス
2. 「New Project」→ 「Deploy from GitHub repo」
3. リポジトリ選択: `project_mirror`
4. Root Directory: `/backend`

### 3. 環境変数設定
Railway Dashboard → Variables タブで以下を設定:

```env
# OpenAI
OPENAI_API_KEY=sk-proj-xxxxx

# Qdrant
QDRANT_URL=https://xxxxx.qdrant.tech
QDRANT_API_KEY=xxxxx

# Neo4j
NEO4J_URI=neo4j+s://xxxxx.databases.neo4j.io
NEO4J_USER=neo4j
NEO4J_PASSWORD=xxxxx

# ログレベル
RUST_LOG=info

# サーバー設定
PORT=8080

# 【将来】Keycloak設定（未実装）
# KEYCLOAK_URL=https://keycloak.example.com
# KEYCLOAK_REALM=project-mirror
# KEYCLOAK_CLIENT_ID=backend-client
# KEYCLOAK_CLIENT_SECRET=xxxxx
# POSTGRES_URL=postgres://user:password@host:port/keycloak_db
```

### 4. デプロイ実行
- Railwayが自動的にDockerfileを検出してビルド
- デプロイ完了後、URLが発行される（例: `https://project-mirror-backend-production.up.railway.app`）

### 5. 動作確認
```bash
# ヘルスチェック
curl https://your-app.railway.app/health

# Chat API確認
curl -X POST https://your-app.railway.app/api/v1/chat/message \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test","text":"テストメッセージ"}'
```

---

## 📱 フロントエンド デプロイ手順（Expo EAS）

### 1. EAS CLIセットアップ
```bash
cd frontend

# EAS CLIインストール
npm install -g eas-cli

# Expoログイン
eas login

# プロジェクト初期化（初回のみ）
eas build:configure
```

### 2. app.jsonの更新
```bash
# EASプロジェクトID取得後、app.jsonを更新
# extra.eas.projectId: "your-project-id"
# owner: "your-expo-username"
```

### 3. バックエンドURLの設定
`eas.json`の`production`と`preview`の`EXPO_PUBLIC_API_BASE_URL`を更新:
```json
"env": {
  "EXPO_PUBLIC_API_BASE_URL": "https://your-app.railway.app/api/v1"
}
```

### 4. Androidビルド実行

#### Preview版（APK - 内部テスト用）
```bash
eas build --platform android --profile preview
```

#### Production版（AAB - Google Play Store用）
```bash
eas build --platform android --profile production
```

ビルド完了後、EAS Dashboardからダウンロード可能。

### 5. Google Play Storeへの提出（オプション）

#### 前提条件
1. [Google Play Console](https://play.google.com/console)でアプリ作成
2. サービスアカウント作成してJSONキーをダウンロード
3. `google-service-account.json`としてfrontend/に保存

#### 提出実行
```bash
eas submit --platform android --profile production
```

### 6. 配信方法

#### A. Internal Testing（推奨・最速）
- Google Play Console → Internal Testing track
- テスターのメールアドレスを登録
- 承認待ち時間: 数時間〜1日

#### B. APK直接配布
```bash
# Preview版APKをダウンロード
eas build:list --platform android --profile preview

# 端末に直接インストール（開発者向け）
```

---

## 🔐 Keycloak統合（将来実装）

### 予定アーキテクチャ
```
┌─────────────────┐
│  Mobile App     │
└────────┬────────┘
         │ OAuth2/OIDC
┌────────▼────────┐
│   Keycloak      │ ← ユーザー認証管理
└────────┬────────┘
         │ JWT Token
┌────────▼────────┐
│   Backend API   │ ← トークン検証
└─────────────────┘
```

### 実装予定タスク
- [ ] RailwayにKeycloakサービス追加
- [ ] PostgreSQL追加（Keycloak用DB）
- [ ] Rustバックエンドにトークン検証ミドルウェア追加
- [ ] フロントエンドにOAuth2ログインフロー実装
- [ ] user_id生成をKeycloak UUIDに変更

### 参考リンク
- [Keycloak Docker](https://www.keycloak.org/getting-started/getting-started-docker)
- [Railway Keycloak Template](https://railway.app/template/keycloak)
- [React Native AppAuth](https://github.com/FormidableLabs/react-native-app-auth)

---

## 🔄 CI/CD（オプション）

### GitHub Actionsワークフロー例
- PRマージ時に自動デプロイ
- Railwayは自動デプロイ対応
- EASビルドはトリガー可能

```yaml
# .github/workflows/deploy.yml（例）
name: Deploy Production

on:
  push:
    branches: [main]

jobs:
  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Deploy to Railway
        run: railway up
        env:
          RAILWAY_TOKEN: ${{ secrets.RAILWAY_TOKEN }}

  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Node
        uses: actions/setup-node@v3
        with:
          node-version: 18
      - name: Install EAS CLI
        run: npm install -g eas-cli
      - name: Build Android
        run: eas build --platform android --non-interactive --no-wait
        env:
          EXPO_TOKEN: ${{ secrets.EXPO_TOKEN }}
```

---

## 📊 監視とログ

### Railway（バックエンド）
- **メトリクス**: CPU, メモリ, ネットワーク使用量
- **ログ**: Dashboard → Deployments → Logs
- **アラート**: Railway Observability（有料プラン）

### Expo EAS（フロントエンド）
- **クラッシュレポート**: Sentry連携推奨
- **アナリティクス**: Firebase Analytics推奨

---

## 🆘 トラブルシューティング

### Railwayビルド失敗
```bash
# ローカルでDockerビルド確認
cd backend
docker build -t test-backend .

# ログ確認
railway logs
```

### EASビルド失敗
```bash
# ローカル確認
cd frontend
expo doctor

# キャッシュクリア
eas build:cancel
eas build --platform android --profile preview --clear-cache
```

### 接続エラー（API到達不可）
- CORSヘッダー確認（backend/src/main.rs）
- Railway URLが正しく設定されているか
- ファイアウォール・VPN設定確認

---

## 📞 サポート

- **Railway**: [docs.railway.app](https://docs.railway.app/)
- **Expo EAS**: [docs.expo.dev/eas](https://docs.expo.dev/eas/)
- **Keycloak**: [www.keycloak.org/documentation](https://www.keycloak.org/documentation)
