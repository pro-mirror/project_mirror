// ===================================================================
// 【将来実装予定】Keycloak マルチユーザー認証
// ===================================================================
// 
// このファイルは、将来のKeycloak統合時に実装される認証機能の
// スケルトンです。現在は未実装ですが、構造を示すために残しています。
//
// 実装タイムライン: TBD（To Be Determined）
// ===================================================================

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone)]
pub struct KeycloakConfig {
    pub url: String,
    pub realm: String,
    pub client_id: String,
    pub client_secret: String,
}

impl KeycloakConfig {
    /// 環境変数からKeycloak設定をロード
    /// 
    /// 必要な環境変数:
    /// - KEYCLOAK_URL
    /// - KEYCLOAK_REALM
    /// - KEYCLOAK_CLIENT_ID
    /// - KEYCLOAK_CLIENT_SECRET
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            url: env::var("KEYCLOAK_URL")
                .map_err(|_| "KEYCLOAK_URL not set".to_string())?,
            realm: env::var("KEYCLOAK_REALM")
                .map_err(|_| "KEYCLOAK_REALM not set".to_string())?,
            client_id: env::var("KEYCLOAK_CLIENT_ID")
                .map_err(|_| "KEYCLOAK_CLIENT_ID not set".to_string())?,
            client_secret: env::var("KEYCLOAK_CLIENT_SECRET")
                .map_err(|_| "KEYCLOAK_CLIENT_SECRET not set".to_string())?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Keycloak User ID（UUIDv4）
    pub sub: String,
    /// Email
    pub email: Option<String>,
    /// Preferred username
    pub preferred_username: Option<String>,
    /// Token expiration
    pub exp: usize,
    /// Issued at
    pub iat: usize,
}

// ===================================================================
// 【将来実装予定】JWT検証ミドルウェア
// ===================================================================
//
// 実装例:
// ```rust
// use axum::{
//     extract::Request,
//     http::StatusCode,
//     middleware::Next,
//     response::Response,
// };
//
// pub async fn verify_jwt_middleware(
//     request: Request,
//     next: Next,
// ) -> Result<Response, StatusCode> {
//     // 1. Authorizationヘッダーからトークン取得
//     let auth_header = request
//         .headers()
//         .get("Authorization")
//         .and_then(|h| h.to_str().ok())
//         .ok_or(StatusCode::UNAUTHORIZED)?;
//
//     // 2. "Bearer <token>" 形式を確認
//     let token = auth_header
//         .strip_prefix("Bearer ")
//         .ok_or(StatusCode::UNAUTHORIZED)?;
//
//     // 3. JWT検証（jsonwebtokenクレート使用）
//     // TODO: Keycloak公開鍵で署名検証
//
//     // 4. Claimsをリクエスト拡張に保存
//     // request.extensions_mut().insert(claims);
//
//     Ok(next.run(request).await)
// }
// ```
// ===================================================================

// ===================================================================
// 【将来実装予定】統合手順
// ===================================================================
//
// 1. Railway上でKeycloakサービス追加
//    - Keycloak Dockerイメージ使用
//    - PostgreSQLサービスをKeycloak用DBとして追加
//
// 2. Cargo.tomlに依存関係追加
//    ```toml
//    jsonwebtoken = "9.2"
//    reqwest = { version = "0.11", features = ["json"] }
//    ```
//
// 3. main.rsでミドルウェア適用
//    ```rust
//    use tower::ServiceBuilder;
//    use axum::middleware;
//    
//    let app = Router::new()
//        .route("/api/v1/chat/message", post(chat_handler))
//        .layer(
//            ServiceBuilder::new()
//                .layer(middleware::from_fn(verify_jwt_middleware))
//        );
//    ```
//
// 4. フロントエンドにOAuth2フロー追加
//    - react-native-app-auth使用
//    - ログイン画面追加
//    - トークンストレージ（SecureStore）
//
// 5. user_id生成ロジック変更
//    - 現在: フロントエンドで生成
//    - 将来: Keycloak Claims.sub（UUID）を使用
//
// ===================================================================
