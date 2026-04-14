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
use crate::models::{ExtractedMemory, CoreValueExtraction};

const EXTRACTION_PROMPT: &str = r#"以下のユーザーテキストから、記憶として保存すべき情報を抽出してください。

抽出する情報：
- persons: 言及された人物の配列（名前、関係性、代名詞を含む。いない場合は空配列）
  例: ["太郎さん", "母", "上司", "彼女"]
- keywords: 関連するキーワード、テーマ、トピックの配列
  例: ["仕事", "家族", "趣味", "健康"]
- emotion_type: 感情のタイプ（"positive", "negative", "neutral"）
- intensity: 感情の強さ（0.0〜1.0）
- reason: その感情の理由や背景

JSON形式で返してください。"#;

#[derive(Debug, Serialize, Deserialize)]
struct ExtractionResponse {
    persons: Vec<String>,
    keywords: Vec<String>,
    emotion_type: String,
    intensity: f32,
    reason: String,
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
        persons: extracted.persons,
        keywords: extracted.keywords,
        emotion_type: extracted.emotion_type,
        intensity: extracted.intensity,
        reason: extracted.reason,
    })
}

const CORE_VALUE_EXTRACTION_PROMPT: &str = r#"以下の会話から、ユーザーが大切にしている「コアバリュー（核となる価値観）」を推測して抽出してください。

コアバリューとは：
- 具体的な出来事ではなく、抽象的な価値観や信念
- 例：「家族との絆」「誠実さ」「自己成長」「思いやり」「自由」など
- ユーザーが感情を動かされた理由の根底にあるもの

各コアバリューについて以下を抽出：
- value_name: 抽象的な価値観の名前
- weight: この会話における重要度（0.0〜1.0）
- context: ユーザーがその価値観について感じたこと、なぜその価値観が現れたかの要約
- related_person: その価値観に関連する人物名（いない場合はnull）

JSON形式で配列として返してください。コアバリューが見当たらない場合は空配列を返してください。

例：
[
  {
    "value_name": "家族との絆",
    "weight": 0.9,
    "context": "母親との時間を大切にしたいという強い想い",
    "related_person": "母"
  }
]
"#;

#[derive(Debug, Serialize, Deserialize)]
struct CoreValueResponse {
    values: Vec<CoreValueExtraction>,
}

pub async fn extract_core_values(
    client: &Client<async_openai::config::OpenAIConfig>,
    user_text: &str,
    ai_reply: &str,
) -> Result<Vec<CoreValueExtraction>> {
    let combined_text = format!("ユーザー: {}\nAI: {}", user_text, ai_reply);
    
    let messages = vec![
        ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(CORE_VALUE_EXTRACTION_PROMPT)
                .build()?
        ),
        ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessageArgs::default()
                .content(combined_text)
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

    // Try to parse as array directly
    let values: Vec<CoreValueExtraction> = serde_json::from_str(&content)
        .or_else(|_| {
            // Try to parse as object with "values" field
            let response: CoreValueResponse = serde_json::from_str(&content)?;
            Ok(response.values)
        })
        .map_err(|e: anyhow::Error| anyhow::anyhow!("Failed to parse JSON: {} - Content: {}", e, content))?;

    tracing::info!("Extracted {} core values", values.len());
    
    Ok(values)
}
