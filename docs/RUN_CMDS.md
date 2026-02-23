# Run Commands Reference

## Quick Start

```bash
# Start everything
docker compose up -d

# Check status
docker compose ps

# Stop everything
docker compose down
```

## Local Development

### Start Infrastructure Only
```bash
docker compose up -d cockroach dragonfly qdrant meilisearch ollama
```

### Backend (PowerShell)
```powershell
cd backend
$env:DATABASE_URL="postgres://root@localhost:26257/azera?sslmode=disable"
$env:DRAGONFLY_URL="redis://localhost:6379"
$env:QDRANT_URL="http://localhost:6333"
$env:OLLAMA_HOST="http://localhost:11434"
$env:MEILI_URL="http://localhost:7700"
$env:RUST_LOG="info,azera_core=debug"
cargo run
```

### Frontend
```bash
cd frontend
bun install
bun dev
```

## URLs
- **Web UI**: http://localhost:5173
- **Canvas**: http://localhost:5173/canvas
- **API**: http://localhost:3000
- **API Health**: http://localhost:3000/health
- **CockroachDB Admin**: http://localhost:8080
- **Qdrant Dashboard**: http://localhost:6333/dashboard
- **Meilisearch**: http://localhost:7700
- **ImageGen**: http://localhost:7860
- **Jenkins**: http://localhost:8081 (admin / azera2026)

## Logs
```bash
docker compose logs -f azera-core
docker compose logs -f azera-web
docker compose logs -f ollama
docker compose logs -f imagegen
```

## Database Access
```bash
docker exec -it azera-cockroach cockroach sql --insecure
```

## Rebuild
```bash
docker compose down
docker compose build --progress=plain
docker compose up -d
```

## Clean Start
```bash
docker compose down --volumes --remove-orphans
docker compose up -d
```

## Docker Disk Cleanup

When the WSL2 virtual disk (VHDX) grows too large, run this procedure to reclaim space.

### 1. Stop & Prune
```powershell
# Shut down all containers
docker compose down

# Remove all unused images, volumes, and build cache
docker system prune -a --volumes -f

# Verify Docker is clean
docker system df
```

### 2. Compact the VHDX
```powershell
# Find the VHDX file
$vhdx = Get-ChildItem "$env:LOCALAPPDATA\Docker" -Recurse -Filter "*.vhdx" -ErrorAction SilentlyContinue |
    Where-Object Name -eq "docker_data.vhdx" | Select-Object -First 1
Write-Host "VHDX: $($vhdx.FullName) — $([math]::Round($vhdx.Length/1GB, 2)) GB"

# Kill Docker Desktop and shut down WSL
Get-Process *docker* -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 5
wsl --shutdown
Start-Sleep 3

# Verify WSL is stopped (all distros should show "Stopped")
wsl -l -v

# Compact via diskpart (requires admin — UAC prompt will appear)
@"
select vdisk file="$($vhdx.FullName)"
compact vdisk
exit
"@ | Set-Content "$env:TEMP\compact_vhdx.txt" -Encoding ASCII

Start-Process diskpart -ArgumentList "/s","$env:TEMP\compact_vhdx.txt" -Wait -Verb RunAs -WindowStyle Hidden

# Check result
$after = (Get-Item $vhdx.FullName).Length / 1GB
Write-Host "VHDX after compact: $([math]::Round($after, 2)) GB"
```

### 3. Restart Azera
```powershell
# Start Docker Desktop
Start-Process "$env:ProgramFiles\Docker\Docker\Docker Desktop.exe"

# Wait for daemon
while (-not (docker info 2>$null)) { Start-Sleep 2 }

# Bring everything back up
docker compose up -d --build
docker compose ps
```

> **Tip**: Run this whenever free disk space drops below ~20 GB or after removing large models/images.
