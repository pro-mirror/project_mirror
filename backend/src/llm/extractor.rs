use anyhow::Result;
use async_openai::{
    Client,
    types::{
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
};
use serde::{Deserialize, Serialize};
use crate::models::ExtractedMemory;

const EXTRACTION_PROMPT: &str = r#"以下の会話から、記憶として保存すべき情報を抽出してください。

抽出する情報：
- person_name: 言及された人物の名前（いない場合はnull）
- emotion_type: 感情のタイプ（"positive", "negative", "neutral"）
- intensity: 感情の強さ（0.0〜1.0）
- reason: その感情の理由や背景
- concepts: 関連する概念やキーワード（配列）

JSON形式で返してください。"#;

#[derive(Debug, Serialize, Deserialize)]
struct ExtractionResponse {
    person_name: Option<String>,
    emotion_type: String,
    intensity: f32,
    reason: String,
    concepts: Vec<String>,
}

pub async fn extract_memory(
    client: &Client<async_openai::config::OpenAIConfig>,
    user_text: &str,
) -> Result<ExtractedMemory> {
    let messages = vec![
        ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(EXTRACTION_PROMPT)
                .build()?
        ),
        ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_text)
                .build()?
        ),
    ];

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages(messages)
        .temperature(0.3)
        .build()?;

    let response = client.chat().create(request).await?;
    
    let content = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?;

    // JSONをパース
    let extracted: ExtractionResponse = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {} - Content: {}", e, content))?;

    Ok(ExtractedMemory {
        person_name: extracted.person_name,
        emotion_type: extracted.emotion_type,
        intensity: extracted.intensity,
        reason: extracted.reason,
        concepts: extracted.concepts,
    })
}
