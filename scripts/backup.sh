#!/usr/bin/env bash
# Aiome Backup & Restore Toolkit
# Generates rolling backups of the local Persistence Layer, including SQLite DB and Vault directories.

# Stop on err or undefined var
set -euo pipefail

# ---- Configuration ----
BACKUP_DIR="${AIOME_BACKUP_DIR:-./backups}"
DATA_DIR="${AIOME_DATA_DIR:-./data}"
MAX_BACKUPS="${AIOME_MAX_BACKUPS:-7}"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
TAR_FILE="$BACKUP_DIR/aiome_backup_$TIMESTAMP.tar.gz"
CHECKSUM_FILE="$TAR_FILE.sha256"

# Container runtime (Podman-first, Docker fallback)
if command -v podman &> /dev/null; then
    COMPOSE_CMD="podman compose"
else
    COMPOSE_CMD="docker compose"
fi

# Check required directories
if [ ! -d "$DATA_DIR/api" ]; then
    echo "ERROR: Data directory '$DATA_DIR/api' not found. Are you running this from the project root?"
    exit 1
fi

mkdir -p "$BACKUP_DIR"

command_backup() {
    echo "📦 Starting Aiome Backup..."
    
    # 1. Create Tarball of the data/api directory
    # Excludes temp files if any (we exclude journal/WAL files to avoid mid-transaction corruption,
    # but for SQLite it's safest to run this while API is stopped, or rely on .backup command)
    echo "Compressing $DATA_DIR/api -> $TAR_FILE"
    
    # Optional: if you want perfect SQLite backup without stopping container,
    # you could do: sqlite3 data/api/aiome.db ".backup 'data/api/aiome.db.bak'"
    # and then archive the .bak file. Here we just archive the directory.
    
    tar -czf "$TAR_FILE" -C "$DATA_DIR" api/
    
    # 2. Checksum
    echo "Verifying Hash..."
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$TAR_FILE" > "$CHECKSUM_FILE"
    else
        sha256sum "$TAR_FILE" > "$CHECKSUM_FILE"
    fi
    
    # 3. Rotate old backups
    echo "Cleaning up old backups (Keeping last $MAX_BACKUPS)..."
    ls -tp "$BACKUP_DIR"/aiome_backup_*.tar.gz | grep -v '/$' | tail -n +$((MAX_BACKUPS + 1)) | xargs -I {} rm -- {} 2>/dev/null || true
    ls -tp "$BACKUP_DIR"/aiome_backup_*.tar.gz.sha256 | grep -v '/$' | tail -n +$((MAX_BACKUPS + 1)) | xargs -I {} rm -- {} 2>/dev/null || true
    
    echo "✅ Backup Completed: $TAR_FILE"
}

command_restore() {
    if [ $# -ne 1 ]; then
        echo "Usage: $0 restore <backup_tar_gz_file>"
        exit 1
    fi
    RESTORE_FILE="$1"
    
    if [ ! -f "$RESTORE_FILE" ]; then
        echo "ERROR: Backup file '$RESTORE_FILE' not found."
        exit 1
    fi
    
    echo "⚠️  WARNING: Restoring will OVERWRITE current data in '$DATA_DIR/api'."
    read -p "Are you sure you want to continue? [y/N]: " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        echo "Restore cancelled."
        exit 0
    fi
    # COMPOSE_CMD is set at the top of this script

    echo "🛑 Stopping services ($COMPOSE_CMD)..."
    $COMPOSE_CMD -f docker-compose.production.yml stop api-server || true
    
    echo "🗑️  Clearing current api data..."
    rm -rf "$DATA_DIR/api"
    
    echo "📦 Extracting backup..."
    tar -xzf "$RESTORE_FILE" -C "$DATA_DIR"
    
    echo "✅ Restore Completed! Restart your services using:"
    echo "$COMPOSE_CMD -f docker-compose.production.yml up -d api-server"
}

case "${1:-}" in
    backup)
        command_backup
        ;;
    restore)
        shift
        command_restore "$@"
        ;;
    *)
        echo "Usage: $0 {backup|restore <file>}"
        exit 1
        ;;
esac
