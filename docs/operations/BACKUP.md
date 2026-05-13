# Aiome Backup Strategy

This document outlines the backup strategies available in Aiome for crash recovery and data protection.

## Overview

Aiome provides a comprehensive, multi-layered backup strategy designed to protect the system's SQLite databases against corruption, accidental deletions, and failed migrations.

The backup strategy consists of two main pillars:
1. **Pre-Migration Guard:** Automatic file-level snapshots taken immediately before running database migrations.
2. **Automated Rolling Backups:** Scheduled backups that use WAL-safe online snapshots to archive data without downtime.

## 1. Pre-Migration Guard

Every time the `api-server` starts, it runs `sqlx` migrations to ensure the database schema is up-to-date. Before applying any migrations, Aiome automatically creates a `.pre_migration.bak` snapshot of the primary SQLite database.

**Location**: Next to your original database (e.g., `aiome.db.pre_migration.bak`).
**Behavior**: 
- It gracefully skips in-memory databases (`:memory:`) and PostgreSQL paths.
- If a migration fails or corrupts the database, you can immediately restore this snapshot to revert to the exact state before the migration attempt.

## 2. Automated Rolling Backups

For production environments, we strongly recommend setting up automated rolling backups using the provided `scripts/backup.sh`.

### WAL-Safe Hot Snapshots
The `backup.sh` script leverages the `sqlite3 .backup` command to safely snapshot SQLite databases even while the application is running (WAL-safe). This ensures complete data consistency without needing to stop the container or API server.

### Setup using Cron (Linux/macOS)

1. Open your `.env` file and configure the backup destination and retention policy:
   ```env
   # Backup Storage Location
   AIOME_BACKUP_DIR=./backups
   # Number of backup archives to keep (Rolling retention)
   AIOME_MAX_BACKUPS=7
   ```

2. Open your crontab:
   ```bash
   crontab -e
   ```

3. Add the following line to run the backup script daily at 3:00 AM:
   ```bash
   0 3 * * * cd /path/to/aiome && ./scripts/backup.sh >> /path/to/aiome/logs/backup.log 2>&1
   ```

### Setup using systemd timers (Linux)

For better logging and control, you can use systemd timers.

1. Create a service file `/etc/systemd/system/aiome-backup.service`:
   ```ini
   [Unit]
   Description=Aiome Automated Backup

   [Service]
   Type=oneshot
   WorkingDirectory=/opt/aiome
   ExecStart=/opt/aiome/scripts/backup.sh
   ```

2. Create a timer file `/etc/systemd/system/aiome-backup.timer`:
   ```ini
   [Unit]
   Description=Run Aiome backup daily at 3 AM

   [Timer]
   OnCalendar=*-*-* 03:00:00
   Persistent=true

   [Install]
   WantedBy=timers.target
   ```

3. Enable and start the timer:
   ```bash
   sudo systemctl enable --now aiome-backup.timer
   ```

## Restoring from a Backup

### Restoring a Pre-Migration Snapshot
If you need to recover from a bad migration:
1. Stop the `api-server` (e.g., `docker compose down`).
2. Move the original corrupted database (don't delete it immediately, just rename it):
   `mv aiome.db aiome.db.corrupted`
3. Restore the pre-migration snapshot:
   `cp aiome.db.pre_migration.bak aiome.db`
4. Start the server again.

### Restoring from an Automated Tar Archive
1. Stop the `api-server`.
2. Extract the backup tarball to a temporary directory:
   `tar -xzf aiome_backup_2026-05-13.tar.gz -C /tmp/restore_aiome`
3. Move the necessary database files from the extracted `api/`, `hub/`, and `nurture/` folders back into your `data/` directory.
4. Restart the server.
