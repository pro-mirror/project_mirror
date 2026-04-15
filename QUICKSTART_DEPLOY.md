# 🚀 デプロイ クイックスタート

## バックエンド（Railway） - 5分で完了

### 1. Railway準備
```bash
# Railway CLIインストール（オプション）  
npm i -g @railway/cli
railway login
```

### 2. デプロイ
1. https://railway.app/new にアクセス
2. **Deploy from GitHub repo** → リポジトリ選択
3. **Root Directory**: `/backend` に設定
4. **Variables**タブで環境変数設定:
   ```
   OPENAI_API_KEY=sk-proj-xxxxx
   QDRANT_URL=https://xxxxx.qdrant.tech
   QDRANT_API_KEY=xxxxx
   NEO4J_URI=neo4j+s://xxxxx.databases.neo4j.io
   NEO4J_USER=neo4j
   NEO4J_PASSWORD=xxxxx
   RUST_LOG=info
   PORT=8080
   ```
5. **Deploy** ボタンをクリック

### 3. 確認
```bash
curl https://your-app.railway.app/health
# 期待レスポンス: {"status":"ok","timestamp":"..."}
```

---

## フロントエンド（Expo EAS） - 10分で完了

### 1. EAS準備
```bash
cd frontend
npm install -g eas-cli
eas login
eas build:configure  # 初回のみ
```

### 2. app.json更新
```json
{
  "expo": {
    "extra": {
      "eas": {
        "projectId": "eas build後に自動生成される"
      }
    },
    "owner": "your-expo-username"
  }
}
```

### 3. eas.json更新
RailwayのURLを設定:
```json
{
  "build": {
    "preview": {
      "env": {
        "EXPO_PUBLIC_API_BASE_URL": "https://your-app.railway.app/api/v1"
      }
    },
    "production": {
      "env": {
        "EXPO_PUBLIC_API_BASE_URL": "https://your-app.railway.app/api/v1"
      }
    }
  }
}
```

### 4. ビルド実行
```bash
# Preview版（APK - すぐテスト可能）
eas build --platform android --profile preview

# Production版（AAB - Google Play用）
eas build --platform android --profile production
```

ビルド完了後、QRコードまたはダウンロードリンクが表示されます。

### 5. インストール
- **Preview版**: ダウンロードしてAndroid端末に直接インストール
- **Production版**: Google Play Consoleにアップロード

---

## 動作確認

### バックエンド
```bash
curl -X POST https://your-app.railway.app/api/v1/chat/message \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test-user","text":"今日も良い一日だった"}'
```

### フロントエンド
1. APKをインストール
2. ホーム画面でメッセージ送信
3. Constellationでグラフ可視化確認

---

## トラブルシューティング

### Railwayビルドエラー
```bash
# ローカルでDockerビルドテスト
cd backend
docker build -t test .
```

### EASビルドエラー
```bash
# キャッシュクリア
eas build --platform android --profile preview --clear-cache
```

### API接続エラー
- `eas.json`のURLが正しいか確認
- RailwayでCORS設定確認（backend/src/main.rs）
- ファイアウォール確認

---

## 次のステップ

✅ **現在完了**
- バックエンド本番デプロイ
- Androidアプリビルド

🔜 **将来実装**
- Keycloak認証（マルチユーザー対応）
- Google Play Storeリリース
- CI/CD自動化
- 監視・ログ（Sentry, Firebase）

詳細は [DEPLOYMENT.md](DEPLOYMENT.md) を参照してください。
