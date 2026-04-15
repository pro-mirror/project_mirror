/**
 * ===================================================================
 * 【将来実装予定】Keycloak OAuth2 認証フロー
 * ===================================================================
 * 
 * このファイルは、将来のKeycloak統合時に実装される
 * フロントエンド認証ロジックのスケルトンです。
 * 
 * 実装タイムライン: TBD（To Be Determined）
 * ===================================================================
 */

import { authorize, refresh, revoke } from 'react-native-app-auth';
import * as SecureStore from 'expo-secure-store';

// ===================================================================
// Keycloak設定
// ===================================================================
const keycloakConfig = {
  // 【要更新】本番Keycloak URLに変更
  issuer: 'https://keycloak.your-domain.com/realms/project-mirror',
  clientId: 'mobile-app',
  // リダイレクトURI（カスタムURLスキーム）
  redirectUrl: 'com.projectmirror.app://oauth/callback',
  scopes: ['openid', 'profile', 'email'],
  
  // 追加設定
  serviceConfiguration: {
    authorizationEndpoint: 'https://keycloak.your-domain.com/realms/project-mirror/protocol/openid-connect/auth',
    tokenEndpoint: 'https://keycloak.your-domain.com/realms/project-mirror/protocol/openid-connect/token',
    revocationEndpoint: 'https://keycloak.your-domain.com/realms/project-mirror/protocol/openid-connect/logout',
  }
};

// ===================================================================
// 【将来実装】ログイン関数
// ===================================================================
export async function loginWithKeycloak() {
  try {
    const authState = await authorize(keycloakConfig);
    
    // トークンを安全に保存
    await SecureStore.setItemAsync('accessToken', authState.accessToken);
    await SecureStore.setItemAsync('refreshToken', authState.refreshToken || '');
    await SecureStore.setItemAsync('idToken', authState.idToken || '');
    
    // ユーザー情報をデコード（JWT Claims）
    const userInfo = decodeJWT(authState.idToken || '');
    
    return {
      userId: userInfo.sub,  // Keycloak User UUID
      email: userInfo.email,
      username: userInfo.preferred_username,
      accessToken: authState.accessToken,
    };
  } catch (error) {
    console.error('Keycloak login failed:', error);
    throw error;
  }
}

// ===================================================================
// 【将来実装】トークン更新
// ===================================================================
export async function refreshAccessToken() {
  try {
    const refreshToken = await SecureStore.getItemAsync('refreshToken');
    if (!refreshToken) {
      throw new Error('No refresh token available');
    }

    const newAuthState = await refresh(keycloakConfig, {
      refreshToken,
    });

    await SecureStore.setItemAsync('accessToken', newAuthState.accessToken);
    
    return newAuthState.accessToken;
  } catch (error) {
    console.error('Token refresh failed:', error);
    // リフレッシュ失敗 → 再ログイン必要
    await logout();
    throw error;
  }
}

// ===================================================================
// 【将来実装】ログアウト
// ===================================================================
export async function logout() {
  try {
    const accessToken = await SecureStore.getItemAsync('accessToken');
    const refreshToken = await SecureStore.getItemAsync('refreshToken');

    if (accessToken && refreshToken) {
      await revoke(keycloakConfig, {
        tokenToRevoke: refreshToken,
        sendClientId: true,
      });
    }

    // ローカルストレージクリア
    await SecureStore.deleteItemAsync('accessToken');
    await SecureStore.deleteItemAsync('refreshToken');
    await SecureStore.deleteItemAsync('idToken');
  } catch (error) {
    console.error('Logout failed:', error);
  }
}

// ===================================================================
// 【将来実装】認証済みAPI呼び出し
// ===================================================================
export async function authenticatedFetch(url: string, options: RequestInit = {}) {
  let accessToken = await SecureStore.getItemAsync('accessToken');
  
  // アクセストークン期限切れチェック（簡易版）
  if (isTokenExpired(accessToken)) {
    accessToken = await refreshAccessToken();
  }

  return fetch(url, {
    ...options,
    headers: {
      ...options.headers,
      'Authorization': `Bearer ${accessToken}`,
    },
  });
}

// ===================================================================
// ヘルパー関数
// ===================================================================

function decodeJWT(token: string): any {
  try {
    const base64Url = token.split('.')[1];
    const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
    const jsonPayload = decodeURIComponent(
      atob(base64)
        .split('')
        .map(c => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2))
        .join('')
    );
    return JSON.parse(jsonPayload);
  } catch (error) {
    console.error('JWT decode error:', error);
    return {};
  }
}

function isTokenExpired(token: string | null): boolean {
  if (!token) return true;
  
  try {
    const decoded = decodeJWT(token);
    const currentTime = Math.floor(Date.now() / 1000);
    return decoded.exp < currentTime;
  } catch {
    return true;
  }
}

// ===================================================================
// 【将来実装予定】統合手順
// ===================================================================
//
// 1. 依存関係インストール
//    ```bash
//    cd frontend
//    npm install react-native-app-auth expo-secure-store
//    ```
//
// 2. LoginScreen.tsx作成
//    ```tsx
//    import { loginWithKeycloak } from '../auth/keycloak';
//    
//    export function LoginScreen() {
//      const handleLogin = async () => {
//        const user = await loginWithKeycloak();
//        // ナビゲーション: ログイン後 → HomeScreen
//      };
//      
//      return (
//        <Button onPress={handleLogin}>
//          Keycloakでログイン
//        </Button>
//      );
//    }
//    ```
//
// 3. client.ts更新（APIクライアント）
//    ```typescript
//    import { authenticatedFetch } from '../auth/keycloak';
//    
//    export async function sendMessage(text: string) {
//      return authenticatedFetch(`${API_BASE_URL}/chat/message`, {
//        method: 'POST',
//        body: JSON.stringify({ text }),
//      });
//    }
//    ```
//
// 4. app.jsonにカスタムスキーム追加
//    ```json
//    "scheme": "com.projectmirror.app"
//    ```
//
// 5. ナビゲーションにLoginScreen追加
//    - 未認証時 → LoginScreen表示
//    - 認証済み → HomeScreen表示
//
// ===================================================================
