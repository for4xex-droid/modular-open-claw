#!/usr/bin/env bash
# Aiome Backup & Restore Toolkit
# Generates rolling backups of the local Persistence Layer, including SQLite DB and Vault directories.

# Stop on err or undefined var
set -euo pipefail

# ---- Configuration ----
CELL_ID="${CELL_ID:-cell-0}"
# 🛡️ CELL_ID バリデーション (パストラバーサル・シェルインジェクション防止)
if [[ ! "$CELL_ID" =~ ^[a-zA-Z0-9_-]{1,64}$ ]]; then
    echo "ERROR: CELL_ID '$CELL_ID' contains invalid characters. Only [a-zA-Z0-9_-] allowed."
    exit 1
fi
BACKUP_DIR="${AIOME_BACKUP_DIR:-./backups/${CELL_ID}}"
DATA_DIR="${AIOME_DATA_DIR:-./data/${CELL_ID}}"
MAX_BACKUPS="${AIOME_MAX_BACKUPS:-7}"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
TAR_FILE="$BACKUP_DIR/${CELL_ID}_backup_$TIMESTAMP.tar.gz"
CHECKSUM_FILE="$TAR_FILE.sha256"

# Container runtime (Podman-first, Docker fallback)
if command -v podman &> /dev/null; then
    COMPOSE_CMD="podman compose"
else
    COMPOSE_CMD="docker compose"
fi

# Check required directories
if [ ! -d "$DATA_DIR" ]; then
    echo "ERROR: Data directory '$DATA_DIR' not found. Are you running this from the project root?"
    exit 1
fi

mkdir -p "$BACKUP_DIR"

command_backup() {
    echo "📦 Starting Aiome Backup..."
    
    # 1. Create Tarball of the data/api directory
    # Excludes temp files if any (we exclude journal/WAL files to avoid mid-transaction corruption,
    # but for SQLite it's safest to run this while API is stopped, or rely on .backup command)
    echo "Compressing $DATA_DIR -> $TAR_FILE"
    
    # 🛡️ SQLite Online Backup (WAL-safe hot snapshot)
    if command -v sqlite3 >/dev/null 2>&1; then
        for DB_FILE in "$DATA_DIR"/api/*.db "$DATA_DIR"/hub/*.db "$DATA_DIR"/nurture/*.db; do
            if [ -f "$DB_FILE" ]; then
                echo "  📸 Creating WAL-safe snapshot: ${DB_FILE}.bak"
                sqlite3 "$DB_FILE" ".backup '${DB_FILE}.bak'" 2>/dev/null || {
                    echo "  ⚠️  sqlite3 .backup failed for $DB_FILE (continuing with tar fallback)"
                }
            fi
        done
    else
        echo "  ℹ️  sqlite3 not found — using tar-only backup (WAL consistency not guaranteed)"
    fi
    
    # 🛡️ O-1: セル内の全サブディレクトリ (api, hub, nurture) をバックアップ
    tar -czf "$TAR_FILE" -C "$(dirname "$DATA_DIR")" "$(basename "$DATA_DIR")/"
    
    # 🧹 Cleanup temporary .bak snapshots (already archived in tar)
    find "$DATA_DIR" -name "*.db.bak" -type f -delete 2>/dev/null || true
    
    # 1.5 Database Encryption Audit Check (Step 2)
    echo "Verifying Database Encryption Status..."
    if command -v sqlite3 >/dev/null 2>&1; then
        DB_PATH="$DATA_DIR/api/aiome.db"
        if [ -f "$DB_PATH" ]; then
            sqlite3 "$DB_PATH" "SELECT value FROM system_settings WHERE is_secret=1;" | while read -r secret_val; do
                # 暗号化された値はhex(AES-256-GCM [nonce(12B)||ciphertext||tag(16B)])であることを確認
                # 最小 56 hex chars (28 bytes = 12 nonce + 16 tag, 空平文の場合)
                if [[ ! "$secret_val" =~ ^[0-9a-f]{56,}$ ]]; then
                    echo "🚨 CRITICAL WARNING: Found potentially unencrypted secret in backup ($DB_PATH)!"
                    echo "   Key value length: ${#secret_val} chars (expected >= 56 hex chars)"
                fi
            done || true
        fi
    fi
    
    # 2. Checksum
    echo "Verifying Hash..."
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$TAR_FILE" > "$CHECKSUM_FILE"
    else
        sha256sum "$TAR_FILE" > "$CHECKSUM_FILE"
    fi
    
    # 3. Rotate old backups
    echo "Cleaning up old backups (Keeping last $MAX_BACKUPS)..."
    ls -tp "$BACKUP_DIR"/${CELL_ID}_backup_*.tar.gz | grep -v '/$' | tail -n +$((MAX_BACKUPS + 1)) | xargs -I {} rm -- {} 2>/dev/null || true
    ls -tp "$BACKUP_DIR"/${CELL_ID}_backup_*.tar.gz.sha256 | grep -v '/$' | tail -n +$((MAX_BACKUPS + 1)) | xargs -I {} rm -- {} 2>/dev/null || true
    
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
    
    echo "⚠️  WARNING: Restoring will OVERWRITE current data in '$DATA_DIR'."
    read -p "Are you sure you want to continue? [y/N]: " confirm
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        echo "Restore cancelled."
        exit 0
    fi
    # COMPOSE_CMD is set at the top of this script

    echo "🛑 Stopping services ($COMPOSE_CMD)..."
    $COMPOSE_CMD -f docker-compose.cell.yml stop api-server || true
    
    echo "🗑️  Clearing current cell data..."
    rm -rf "$DATA_DIR"
    mkdir -p "$DATA_DIR"
    
    echo "📦 Extracting backup..."
    tar -xzf "$RESTORE_FILE" -C "$(dirname "$DATA_DIR")"
    
    echo "✅ Restore Completed! Restart your services using:"
    echo "$COMPOSE_CMD -f docker-compose.cell.yml up -d api-server"
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
