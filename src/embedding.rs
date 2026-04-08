use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::config::EmbeddingConfig;

/// Generate embedding vector for a text string.
pub async fn embed(config: &EmbeddingConfig, text: &str) -> Result<Vec<f32>> {
    let client = Client::new();

    let base = if config.base_url.is_empty() {
        match config.provider.as_str() {
            "openai" => "https://api.openai.com",
            _ => "http://127.0.0.1:11434",
        }
    } else {
        config.base_url.trim_end_matches('/')
    };

    let url = format!("{base}/v1/embeddings");

    let body = serde_json::json!({
        "model": config.model,
        "input": text,
    });

    let mut req = client
        .post(&url)
        .header("content-type", "application/json");

    if !config.api_key.is_empty() {
        req = req.header("authorization", format!("Bearer {}", config.api_key));
    }

    let resp = req
        .json(&body)
        .send()
        .await
        .context("Embedding request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("Embedding API error {status}: {text}");
    }

    let data: serde_json::Value = resp.json().await.context("Failed to parse embedding response")?;

    let embedding = data
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.get("embedding"))
        .and_then(|e| e.as_array())
        .context("Invalid embedding response format")?
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect::<Vec<f32>>();

    if embedding.is_empty() {
        bail!("Empty embedding returned");
    }

    Ok(embedding)
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_vectors_have_similarity_one() {
        let v = vec![1.0, 2.0, 3.0];
        let score = cosine_similarity(&v, &v);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn orthogonal_vectors_have_similarity_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let score = cosine_similarity(&a, &b);
        assert!(score.abs() < 1e-6);
    }

    #[test]
    fn opposite_vectors_have_negative_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let score = cosine_similarity(&a, &b);
        assert!((score - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn empty_vectors_return_zero() {
        let score = cosine_similarity(&[], &[]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn mismatched_lengths_return_zero() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let score = cosine_similarity(&a, &b);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn zero_vector_returns_zero() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
        assert_eq!(cosine_similarity(&b, &a), 0.0);
    }
}
