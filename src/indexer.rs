use anyhow::{anyhow, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::PathBuf;

pub struct Embedder(TextEmbedding);

fn model_cache_dir() -> Result<PathBuf> {
    let base =
        dirs::cache_dir().ok_or_else(|| anyhow!("could not determine user cache directory"))?;
    Ok(base.join("memex").join("models"))
}

impl Embedder {
    pub fn new() -> Result<Self> {
        let cache = model_cache_dir()?;
        std::fs::create_dir_all(&cache)?;
        let opts = InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_cache_dir(cache);
        let model = TextEmbedding::try_new(opts)?;
        Ok(Self(model))
    }

    pub fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut out = self.0.embed(vec![text], None)?;
        Ok(out.remove(0))
    }

    pub fn embed_batch<S: AsRef<str> + Send + Sync>(&self, texts: Vec<S>) -> Result<Vec<Vec<f32>>> {
        let refs: Vec<&str> = texts.iter().map(|s| s.as_ref()).collect();
        Ok(self.0.embed(refs, None)?)
    }
}
