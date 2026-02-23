//! Backup and Restore System for Azera's datastore
//! 
//! Provides incremental/snapshot backups with compression for:
//! - CockroachDB, Qdrant, Meilisearch, DragonflyDB, Jenkins: compressed snapshots
//! - Ollama: model ledger (stores list of models, pulls on restore)

use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
use std::collections::HashMap;

/// Backup configuration
#[derive(Clone)]
pub struct BackupConfig {
    pub datastore_path: PathBuf,
    pub backup_path: PathBuf,
    pub interval_mins: u64,
    pub ollama_host: String,
}

impl BackupConfig {
    pub fn from_env() -> Self {
        let interval_mins: u64 = std::env::var("BACKUP_INTERVAL_MINS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        Self {
            datastore_path: PathBuf::from("../datastore"),
            backup_path: PathBuf::from("../datastore/backup"),
            interval_mins,
            ollama_host: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
        }
    }
}

/// Service types for backup
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    Cockroach,
    Qdrant,
    Meilisearch,
    Dragonfly,
    Jenkins,
    Ollama,
}

impl ServiceType {
    pub fn folder_name(&self) -> &'static str {
        match self {
            Self::Cockroach => "cockroach",
            Self::Qdrant => "qdrant",
            Self::Meilisearch => "meilisearch",
            Self::Dragonfly => "dragonfly",
            Self::Jenkins => "jenkins",
            Self::Ollama => "ollama",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Cockroach,
            Self::Qdrant,
            Self::Meilisearch,
            Self::Dragonfly,
            Self::Jenkins,
            Self::Ollama,
        ]
    }
}

/// Ollama model ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaLedger {
    pub models: Vec<String>,
    pub last_updated: DateTime<Utc>,
}

/// Backup manifest tracking what's been backed up
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub last_backup: DateTime<Utc>,
    pub services: HashMap<String, ServiceBackupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBackupInfo {
    pub last_backup: DateTime<Utc>,
    pub backup_file: String,
    pub size_bytes: u64,
    pub checksum: String,
}

impl BackupManifest {
    pub fn new() -> Self {
        Self {
            version: 1,
            created_at: Utc::now(),
            last_backup: Utc::now(),
            services: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read manifest")?;
        serde_json::from_str(&content)
            .context("Failed to parse manifest")
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Main backup service
pub struct BackupService {
    config: BackupConfig,
}

impl BackupService {
    pub fn new(config: BackupConfig) -> Self {
        Self { config }
    }

    /// Run the backup loop in the background
    pub async fn run_backup_loop(self) {
        tracing::info!("📦 Backup service starting (interval: {} mins)...", self.config.interval_mins);
        
        // Ensure backup directory exists
        if let Err(e) = tokio::fs::create_dir_all(&self.config.backup_path).await {
            tracing::error!("Failed to create backup directory: {}", e);
            return;
        }

        loop {
            if let Err(e) = self.run_backup_cycle().await {
                tracing::error!("Backup cycle failed: {}", e);
            }
            
            sleep(Duration::from_secs(self.config.interval_mins * 60)).await;
        }
    }

    /// Run a single backup cycle for all services
    async fn run_backup_cycle(&self) -> Result<()> {
        tracing::debug!("📦 Running backup cycle...");
        
        let manifest_path = self.config.backup_path.join("manifest.json");
        let mut manifest = if manifest_path.exists() {
            BackupManifest::load(&manifest_path).unwrap_or_else(|_| BackupManifest::new())
        } else {
            BackupManifest::new()
        };

        for service in ServiceType::all() {
            match service {
                ServiceType::Ollama => {
                    if let Err(e) = self.backup_ollama_ledger().await {
                        tracing::warn!("Ollama ledger backup failed: {}", e);
                    }
                }
                _ => {
                    if let Err(e) = self.backup_service(&service, &mut manifest).await {
                        tracing::warn!("{:?} backup failed: {}", service, e);
                    }
                }
            }
        }

        manifest.last_backup = Utc::now();
        manifest.save(&manifest_path)?;
        
        tracing::debug!("📦 Backup cycle complete");
        Ok(())
    }

    /// Backup a service's data directory as compressed tarball
    async fn backup_service(&self, service: &ServiceType, manifest: &mut BackupManifest) -> Result<()> {
        let source_dir = self.config.datastore_path.join(service.folder_name());
        
        if !source_dir.exists() {
            tracing::debug!("{:?} directory doesn't exist, skipping", service);
            return Ok(());
        }

        // Check if directory has content
        let dir_size = dir_size(&source_dir)?;
        if dir_size == 0 {
            tracing::debug!("{:?} directory is empty, skipping", service);
            return Ok(());
        }

        let backup_file = format!("{}.tar.zst", service.folder_name());
        let backup_path = self.config.backup_path.join(&backup_file);
        
        // Check if we need to backup (compare checksums)
        // For databases with locked files, use a simpler check
        let current_checksum = match service {
            ServiceType::Cockroach => {
                // CockroachDB has locked files, use a timestamp-based approach
                // Only backup if no recent backup exists (within last hour)
                if let Some(info) = manifest.services.get(service.folder_name()) {
                    let hours_since = Utc::now()
                        .signed_duration_since(info.last_backup)
                        .num_hours();
                    if hours_since < 1 {
                        tracing::debug!("{:?} backed up recently, skipping", service);
                        return Ok(());
                    }
                }
                format!("time_{}", Utc::now().timestamp())
            }
            _ => calculate_dir_checksum(&source_dir)?,
        };
        
        if let Some(info) = manifest.services.get(service.folder_name()) {
            if info.checksum == current_checksum && !matches!(service, ServiceType::Cockroach) {
                tracing::debug!("{:?} unchanged, skipping backup", service);
                return Ok(());
            }
        }

        tracing::info!("📦 Backing up {:?}...", service);
        
        // Create compressed tarball using tar and zstd
        // For CockroachDB, we try but may fail due to locks - that's ok
        match compress_directory(&source_dir, &backup_path).await {
            Ok(()) => {
                let backup_size = std::fs::metadata(&backup_path)
                    .map(|m| m.len())
                    .unwrap_or(0);

                manifest.services.insert(
                    service.folder_name().to_string(),
                    ServiceBackupInfo {
                        last_backup: Utc::now(),
                        backup_file,
                        size_bytes: backup_size,
                        checksum: current_checksum,
                    },
                );

                tracing::info!("📦 {:?} backed up ({} bytes compressed)", service, backup_size);
            }
            Err(e) => {
                // For Cockroach, file lock errors are expected when DB is running
                // The database will be recoverable from Docker volume or from when it's not running
                if matches!(service, ServiceType::Cockroach) {
                    tracing::debug!("{:?} backup skipped (database in use): {}", service, e);
                } else {
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }

    /// Backup Ollama models as a ledger (list of model names)
    async fn backup_ollama_ledger(&self) -> Result<()> {
        let ledger_path = self.config.backup_path.join("ollama_ledger.json");
        
        // Query Ollama for installed models
        let mut models = self.get_ollama_models().await?;
        
        if models.is_empty() {
            tracing::debug!("No Ollama models found");
            return Ok(());
        }

        // Sort for consistent comparison
        models.sort();

        // Check if models have changed from existing ledger
        if ledger_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&ledger_path).await {
                if let Ok(existing) = serde_json::from_str::<OllamaLedger>(&content) {
                    let mut existing_models = existing.models.clone();
                    existing_models.sort();
                    if existing_models == models {
                        tracing::debug!("Ollama models unchanged, skipping ledger update");
                        return Ok(());
                    }
                }
            }
        }

        let ledger = OllamaLedger {
            models: models.clone(),
            last_updated: Utc::now(),
        };

        let content = serde_json::to_string_pretty(&ledger)?;
        tokio::fs::write(&ledger_path, content).await?;
        
        tracing::debug!("📦 Ollama ledger updated: {} models", ledger.models.len());
        Ok(())
    }

    /// Get list of installed Ollama models
    async fn get_ollama_models(&self) -> Result<Vec<String>> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/tags", self.config.ollama_host);
        
        let response = client.get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        #[derive(Deserialize)]
        struct OllamaModels {
            models: Option<Vec<OllamaModel>>,
        }
        
        #[derive(Deserialize)]
        struct OllamaModel {
            name: String,
        }

        let data: OllamaModels = response.json().await?;
        Ok(data.models
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.name)
            .collect())
    }

    /// Check if datastore volumes exist and have data
    pub async fn check_volumes(&self) -> HashMap<ServiceType, bool> {
        let mut status = HashMap::new();
        
        for service in ServiceType::all() {
            let path = self.config.datastore_path.join(service.folder_name());
            let has_data = path.exists() && dir_size(&path).unwrap_or(0) > 0;
            status.insert(service, has_data);
        }
        
        status
    }

    /// Restore all services from backups
    pub async fn restore_all(&self) -> Result<()> {
        tracing::info!("🔄 Starting restore from backups...");
        
        let manifest_path = self.config.backup_path.join("manifest.json");
        
        if !manifest_path.exists() {
            tracing::info!("No backup manifest found, starting fresh");
            return Ok(());
        }

        let manifest = BackupManifest::load(&manifest_path)?;

        for service in ServiceType::all() {
            match service {
                ServiceType::Ollama => {
                    if let Err(e) = self.restore_ollama_models().await {
                        tracing::warn!("Ollama restore failed: {}", e);
                    }
                }
                _ => {
                    if let Err(e) = self.restore_service(&service, &manifest).await {
                        tracing::warn!("{:?} restore failed: {}", service, e);
                    }
                }
            }
        }

        tracing::info!("🔄 Restore complete");
        Ok(())
    }

    /// Restore a service from its backup
    async fn restore_service(&self, service: &ServiceType, manifest: &BackupManifest) -> Result<()> {
        let info = match manifest.services.get(service.folder_name()) {
            Some(info) => info,
            None => {
                tracing::debug!("No backup found for {:?}", service);
                return Ok(());
            }
        };

        let backup_path = self.config.backup_path.join(&info.backup_file);
        let target_dir = self.config.datastore_path.join(service.folder_name());

        if !backup_path.exists() {
            tracing::warn!("Backup file missing for {:?}: {}", service, info.backup_file);
            return Ok(());
        }

        tracing::info!("🔄 Restoring {:?}...", service);
        
        // Create target directory
        tokio::fs::create_dir_all(&target_dir).await?;
        
        // Extract compressed tarball
        decompress_to_directory(&backup_path, &target_dir).await?;
        
        tracing::info!("🔄 {:?} restored", service);
        Ok(())
    }

    /// Restore Ollama models from ledger
    async fn restore_ollama_models(&self) -> Result<()> {
        let ledger_path = self.config.backup_path.join("ollama_ledger.json");
        
        if !ledger_path.exists() {
            tracing::debug!("No Ollama ledger found");
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&ledger_path).await?;
        let ledger: OllamaLedger = serde_json::from_str(&content)?;
        
        tracing::info!("🔄 Restoring {} Ollama models...", ledger.models.len());
        
        for model in &ledger.models {
            tracing::info!("🔄 Pulling Ollama model: {}", model);
            if let Err(e) = self.pull_ollama_model(model).await {
                tracing::warn!("Failed to pull {}: {}", model, e);
            }
        }

        Ok(())
    }

    /// Pull an Ollama model
    async fn pull_ollama_model(&self, model: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/pull", self.config.ollama_host);
        
        let response = client.post(&url)
            .json(&serde_json::json!({ "name": model }))
            .timeout(Duration::from_secs(3600)) // 1 hour timeout for large models
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to pull model: {}", response.status());
        }

        Ok(())
    }
}

/// Startup initialization - check volumes and restore if needed
pub async fn init_datastore(config: &BackupConfig) -> Result<()> {
    tracing::info!("🔍 Checking datastore volumes...");
    
    let backup_service = BackupService::new(config.clone());
    let volume_status = backup_service.check_volumes().await;
    
    // Check if any volumes are empty
    let empty_volumes: Vec<_> = volume_status
        .iter()
        .filter(|(_, has_data)| !**has_data)
        .map(|(service, _)| service)
        .collect();

    if empty_volumes.is_empty() {
        tracing::info!("✅ All datastore volumes have data");
        return Ok(());
    }

    tracing::info!("📂 Empty volumes detected: {:?}", empty_volumes);
    
    // Check if we have backups
    let manifest_path = config.backup_path.join("manifest.json");
    if manifest_path.exists() {
        tracing::info!("🔄 Found backups, restoring...");
        backup_service.restore_all().await?;
    } else {
        tracing::info!("📂 No backups found, services will initialize fresh");
        
        // Ensure directories exist
        for service in ServiceType::all() {
            let path = config.datastore_path.join(service.folder_name());
            tokio::fs::create_dir_all(&path).await?;
        }
    }

    Ok(())
}

/// Calculate directory size
fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0u64;
    
    if path.is_file() {
        return Ok(std::fs::metadata(path)?.len());
    }

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            size += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    
    Ok(size)
}

/// Calculate a simple checksum of directory contents (file count + total size + mod times)
fn calculate_dir_checksum(path: &Path) -> Result<String> {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    let mut entries: Vec<_> = walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    
    // Sort for consistent ordering
    entries.sort_by_key(|e| e.path().to_path_buf());
    
    for entry in entries {
        if let Ok(meta) = entry.metadata() {
            hasher.update(entry.path().to_string_lossy().as_bytes());
            hasher.update(meta.len().to_le_bytes());
            if let Ok(modified) = meta.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    hasher.update(duration.as_secs().to_le_bytes());
                }
            }
        }
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

/// Compress a directory to a .tar.zst file
async fn compress_directory(source: &Path, dest: &Path) -> Result<()> {
    use std::fs::File;
    
    let dest_path = dest.to_path_buf();
    let source_path = source.to_path_buf();
    
    // Run in blocking task since tar operations are CPU-bound
    tokio::task::spawn_blocking(move || {
        let file = File::create(&dest_path)?;
        
        // Use zstd compression (level 3 for good balance of speed/ratio)
        let encoder = zstd::stream::Encoder::new(file, 3)?;
        let mut tar = tar::Builder::new(encoder.auto_finish());
        
        // Add all files from the source directory
        tar.append_dir_all(".", &source_path)?;
        tar.finish()?;
        
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    
    Ok(())
}

/// Decompress a .tar.zst file to a directory
async fn decompress_to_directory(source: &Path, dest: &Path) -> Result<()> {
    use std::fs::File;
    
    let source_path = source.to_path_buf();
    let dest_path = dest.to_path_buf();
    
    tokio::task::spawn_blocking(move || {
        let file = File::open(&source_path)?;
        let decoder = zstd::stream::Decoder::new(file)?;
        let mut archive = tar::Archive::new(decoder);
        
        archive.unpack(&dest_path)?;
        
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    
    Ok(())
}
