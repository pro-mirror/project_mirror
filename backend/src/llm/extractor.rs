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

/// Strip markdown code blocks (```json ... ```) from LLM response
fn strip_markdown_code_block(content: &str) -> String {
    let trimmed = content.trim();
    
    // Check if wrapped in ```json ... ``` or ``` ... ```
    if trimmed.starts_with("```") {
        // Find the first newline after opening ```
        if let Some(start) = trimmed.find('\n') {
            // Find the closing ```
            if let Some(end) = trimmed.rfind("```") {
                if end > start {
                    return trimmed[start + 1..end].trim().to_string();
                }
            }
        }
    }
    
    trimmed.to_string()
}

const EXTRACTION_PROMPT: &str = r#"以下のユーザーテキストから、記憶として保存すべき情報を抽出してください。

抽出する情報：
- persons: 言及された人物の配列（名前、関係性、代名詞を含む。いない場合は空配列）
  例: ["太郎さん", "母", "上司", "彼女"]
- keywords: 関連するキーワード、テーマ、トピックの配列
  例: ["仕事", "家族", "趣味", "健康"]
- emotion_type: 感情のタイプ（"positive", "negative", "neutral"）
- intensity: 感情の強さ（0.0〜1.0）
- reason: その感情の理由や背景

重要: 純粋なJSON形式のみで返してください。マークダウンのコードブロック（```json など）は使用しないでください。"#;

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
        .max_tokens(500u16)
        .build()?;

    let response = client.chat().create(request).await?;
    
    let choice = response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?;
    
    // Check if response was truncated
    if let Some(finish_reason) = &choice.finish_reason {
        tracing::debug!("Memory extraction finish_reason: {:?}", finish_reason);
        let reason_str = format!("{:?}", finish_reason).to_lowercase();
        if reason_str.contains("length") {
            tracing::warn!("Memory extraction response was truncated due to max_tokens limit");
        }
    }
    
    let content = choice.message.content.clone()
        .ok_or_else(|| anyhow::anyhow!("No content in response"))?;

    // Strip markdown code blocks if present
    let json_content = strip_markdown_code_block(&content);

    // JSONをパース
    let extracted: ExtractionResponse = serde_json::from_str(&json_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {} - Content: {}", e, json_content))?;

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

重要: 純粋なJSON配列のみで返してください。マークダウンのコードブロック（```json など）は使用しないでください。コアバリューが見当たらない場合は空配列 [] を返してください。

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
        .max_tokens(800u16)
        .build()?;

    let response = client.chat().create(request).await?;
    
    let choice = response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?;
    
    // Check if response was truncated
    if let Some(finish_reason) = &choice.finish_reason {
        tracing::debug!("Core value extraction finish_reason: {:?}", finish_reason);
        let reason_str = format!("{:?}", finish_reason).to_lowercase();
        if reason_str.contains("length") {
            tracing::warn!("Core value extraction response was truncated due to max_tokens limit");
        }
    }
    
    let content = choice.message.content.clone()
        .ok_or_else(|| anyhow::anyhow!("No content in response"))?;

    // Strip markdown code blocks if present
    let json_content = strip_markdown_code_block(&content);

    // Try to parse as array directly
    let values: Vec<CoreValueExtraction> = serde_json::from_str(&json_content)
        .or_else(|_| {
            // Try to parse as object with "values" field
            let response: CoreValueResponse = serde_json::from_str(&json_content)?;
            Ok(response.values)
        })
        .map_err(|e: anyhow::Error| anyhow::anyhow!("Failed to parse JSON: {} - Content: {}", e, json_content))?;

    tracing::info!("Extracted {} core values", values.len());
    
    Ok(values)
}
