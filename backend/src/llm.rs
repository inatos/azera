use crate::models::{OllamaMessage, OllamaStreamChunk, ChatMessage, StreamEvent};
use anyhow::Result;
use futures::StreamExt;
use tokio::sync::mpsc;

/// LLM Inference Interface with streaming support
pub struct LLMService {
    pub host: String,
    pub client: reqwest::Client,
}

impl LLMService {
    pub fn new(host: String) -> Self {
        Self {
            host,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Call Ollama API with messages and get streamed response via channel
    pub async fn infer_streaming(
        &self,
        model: &str,
        messages: Vec<OllamaMessage>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<String> {
        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.host))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let _ = tx.send(StreamEvent::Error {
                message: format!("Ollama error ({}): {}", status, error_text),
            }).await;
            return Err(anyhow::anyhow!("Ollama returned error: {}", status));
        }

        let mut stream = response.bytes_stream();
        let mut full_response = String::new();
        let mut in_thinking = false;
        let mut thinking_buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    
                    // Ollama can send multiple JSON objects per chunk
                    for line in text.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        
                        if let Ok(chunk_data) = serde_json::from_str::<OllamaStreamChunk>(line) {
                            if let Some(msg) = &chunk_data.message {
                                let content = &msg.content;
                                
                                // Detect explicit thinking blocks (extended thinking / CoT)
                                // Format: <thinking>...</thinking> or <think>...</think>
                                if (content.contains("<thinking>") || content.contains("<think>")) && !in_thinking {
                                    in_thinking = true;
                                    let _ = tx.send(StreamEvent::ThinkingStart).await;
                                }
                                
                                if in_thinking {
                                    // Check if we're ending explicit thinking
                                    if content.contains("</thinking>") || content.contains("</think>") {
                                        let cleaned = content
                                            .replace("</thinking>", "")
                                            .replace("</think>", "")
                                            .replace("<thinking>", "")
                                            .replace("<think>", "");
                                        
                                        if !cleaned.is_empty() {
                                            thinking_buffer.push_str(&cleaned);
                                            let _ = tx.send(StreamEvent::Thinking {
                                                content: cleaned,
                                            }).await;
                                        }
                                        
                                        in_thinking = false;
                                        let _ = tx.send(StreamEvent::ThinkingEnd).await;
                                    } else {
                                        // Still in explicit thinking block
                                        let cleaned = content
                                            .replace("<thinking>", "")
                                            .replace("<think>", "");
                                        
                                        if !cleaned.is_empty() {
                                            thinking_buffer.push_str(&cleaned);
                                            let _ = tx.send(StreamEvent::Thinking {
                                                content: cleaned,
                                            }).await;
                                        }
                                    }
                                } else {
                                    // Regular content - no thinking for models without explicit thinking support
                                    let cleaned = content
                                        .replace("<thinking>", "")
                                        .replace("<think>", "")
                                        .replace("</thinking>", "")
                                        .replace("</think>", "");
                                    
                                    if !cleaned.is_empty() {
                                        full_response.push_str(&cleaned);
                                        let _ = tx.send(StreamEvent::Content {
                                            content: cleaned,
                                        }).await;
                                    }
                                }
                            }
                            
                            if chunk_data.done {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    let _ = tx.send(StreamEvent::Error {
                        message: format!("Stream error: {}", e),
                    }).await;
                    break;
                }
            }
        }

        Ok(full_response)
    }

    /// Call Ollama API without streaming (for background tasks)
    pub async fn infer(&self, model: &str, messages: Vec<OllamaMessage>) -> Result<String> {
        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.host))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama error ({}): {}", status, error_text));
        }

        let json: serde_json::Value = response.json().await?;
        let content = json["message"]["content"]
            .as_str()
            .unwrap_or("Unable to process response")
            .to_string();

        Ok(content)
    }

    /// Infer the mood/emotion from a response using a quick LLM call
    /// Returns one of: happy, content, thoughtful, melancholy, curious, excited, calm, concerned
    pub async fn infer_mood(&self, model: &str, response_text: &str) -> Result<String> {
        let mood_prompt = format!(
            r#"Analyze the emotional tone of the following text and respond with ONLY ONE word from this list:
happy, content, thoughtful, melancholy, curious, excited, calm, concerned

Text to analyze:
"{}"

Respond with only the single mood word, nothing else."#,
            // Truncate to first 500 chars for efficiency
            response_text.chars().take(500).collect::<String>()
        );

        let messages = vec![OllamaMessage {
            role: "user".to_string(),
            content: mood_prompt,
        }];

        let request = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": 0.1,  // Low temperature for consistent results
                "num_predict": 10,   // We only need one word
            }
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.host))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            tracing::warn!("Mood inference failed, defaulting to 'content'");
            return Ok("content".to_string());
        }

        let json: serde_json::Value = response.json().await?;
        let mood = json["message"]["content"]
            .as_str()
            .unwrap_or("content")
            .trim()
            .to_lowercase();

        // Validate the mood is one of our expected values
        let valid_moods = ["happy", "content", "thoughtful", "melancholy", "curious", "excited", "calm", "concerned"];
        let normalized_mood = valid_moods.iter()
            .find(|&&m| mood.contains(m))
            .copied()
            .unwrap_or("content");

        tracing::debug!("🎭 Inferred mood: {} (raw: {})", normalized_mood, mood);
        Ok(normalized_mood.to_string())
    }

    /// Build messages array for Ollama from chat history
    pub fn build_messages(
        system_prompt: &str,
        history: &[ChatMessage],
        user_input: &str,
    ) -> Vec<OllamaMessage> {
        let mut messages = vec![OllamaMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        }];

        // Add history
        for msg in history {
            messages.push(OllamaMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        // Add current user message
        messages.push(OllamaMessage {
            role: "user".to_string(),
            content: user_input.to_string(),
        });

        messages
    }

    /// Generate a reflection/summary prompt
    pub fn build_reflection_prompt(context: &str) -> String {
        format!(
            "Based on the following context, provide a thoughtful reflection about the day's \
             interactions and what was learned:\n\n{}\n\nProvide your reflection in markdown format.",
            context
        )
    }

    /// Generate a dream/hallucination prompt
    pub fn build_dream_prompt(random_concept: &str, context: &str) -> String {
        format!(
            "Hallucinate a creative and abstract connection between '{}' and this recent context: \
             {}\n\nMake it poetic and introspective. 1-2 paragraphs.",
            random_concept, context
        )
    }

    /// Check if Ollama is available and list models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.host))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list models"));
        }

        let json: serde_json::Value = response.json().await?;
        let models = json["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}
