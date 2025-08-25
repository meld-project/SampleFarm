# SampleFarm Backend

A Rust-based malware analysis platform backend that provides sample management, task orchestration, and integration with CAPE and CFG sandboxes.

## Features

- **Sample Management**: Upload, store, and manage malware samples with metadata
- **Task Orchestration**: Create and manage analysis tasks with multiple analyzer support
- **Sandbox Integration**: Dynamic integration with CAPE and CFG sandbox instances via API
- **File Processing**: Secure file handling with validation, hashing, and extraction
- **RESTful API**: Comprehensive REST API with OpenAPI/Swagger documentation
- **Health Monitoring**: Built-in health checks for all system components
- **Startup Recovery**: Automatic recovery of interrupted tasks on system restart

## Architecture

```
backend/
├── src/
│   ├── config/          # Configuration management
│   ├── database/        # PostgreSQL database layer
│   ├── file_processing/ # File validation, hashing, extraction
│   ├── handlers/        # HTTP request handlers
│   ├── models/          # Data models and schemas
│   ├── repositories/    # Database access layer
│   ├── services/        # Business logic services
│   ├── storage/         # MinIO object storage integration
│   └── main.rs          # Application entry point
├── config.example.toml  # Configuration template
└── Cargo.toml          # Dependencies and metadata
```

## Dependencies

### Core Dependencies
- **axum** - Modern async web framework
- **sqlx** - Async PostgreSQL driver with compile-time checked queries
- **tokio** - Async runtime
- **serde** - Serialization/deserialization
- **tracing** - Structured logging

### Storage & Processing
- **aws-sdk-s3** - MinIO/S3 compatible object storage
- **reqwest** - HTTP client for sandbox communication
- **zip** - ZIP file processing with AES encryption support
- **sha1/sha2/md5** - Cryptographic hashing

### API Documentation
- **utoipa** - OpenAPI 3.0 specification generation
- **utoipa-swagger-ui** - Swagger UI integration

## Configuration

Copy the example configuration:
```bash
cp config.example.toml config.toml
```

Edit `config.toml` with your settings:

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://samplefarm_user:samplefarm_password@localhost/samplefarm"
max_connections = 20

[minio]
endpoint = "http://localhost:9000"
access_key = "minioadmin"
secret_key = "minioadmin"
bucket = "samplefarm"

[file]
max_size = 1073741824  # 1GB
temp_dir = "/tmp/samplefarm"

[startup_recovery]
enabled = true
initial_delay_secs = 10
scan_interval_secs = 300
batch_size = 20
global_concurrency = 8
stuck_submitting_threshold_secs = 300
```

## Development

### Prerequisites
- Rust 1.70+ (2024 edition)
- PostgreSQL 12+
- MinIO or S3-compatible storage

### Setup

#### 1. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 2. Database Setup
Create PostgreSQL database and user:
```sql
-- Connect as postgres superuser
CREATE DATABASE samplefarm;
CREATE USER samplefarm_user WITH PASSWORD 'samplefarm_password';
GRANT ALL PRIVILEGES ON DATABASE samplefarm TO samplefarm_user;
```

Initialize database schema:
```bash
# Option 1: Use the complete deployment script
psql -U samplefarm_user -d samplefarm -f database/deploy.sql

# Option 2: Run scripts individually
psql -U samplefarm_user -d samplefarm -f database/init.sql
psql -U samplefarm_user -d samplefarm -f database/schema.sql
```

#### 3. Build and Run
```bash
# Clone and build
git clone <repository>
cd backend
cargo build

# Run tests
cargo test

# Start development server
cargo run
```

### Code Quality
```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check compilation
cargo check
```

### Database Files

The `database/` directory contains SQL scripts for manual database initialization:

- **`init.sql`** - Creates database types, extensions, and enums
- **`schema.sql`** - Creates all tables, indexes, constraints, and triggers  
- **`deploy.sql`** - Complete deployment script that runs both init.sql and schema.sql

All scripts support idempotent execution (can be run multiple times safely).

## Deployment

### Using Docker Compose
```bash
# From project root
docker-compose up -d
```

### Manual Deployment
1. Build release binary:
   ```bash
   cargo build --release
   ```

2. Copy binary and config:
   ```bash
   cp target/release/samplefarm-backend /opt/samplefarm/
   cp config.toml /opt/samplefarm/
   ```

3. Run with systemd or process manager:
   ```bash
   ./samplefarm-backend
   ```

## API Documentation

### Swagger UI
Visit `http://localhost:8080/swagger-ui` for interactive API documentation.

### OpenAPI Specification
- JSON: `http://localhost:8080/api-docs/openapi.json`
- YAML: Available through Swagger UI export

### Health Endpoints
- **General Health**: `GET /health`
- **Database Health**: `GET /api/health/db`
- **Storage Health**: `GET /api/health/storage`
- **File Processor Health**: `GET /api/health/file-processor`

### Main API Groups
- **Samples**: `/api/samples/*` - Sample upload and management
- **Tasks**: `/api/tasks/*` - Analysis task management
- **CAPE Instances**: `/api/cape-instances/*` - CAPE sandbox management
- **CFG Instances**: `/api/cfg-instances/*` - CFG sandbox management
- **System**: `/api/system/*` - System information and status

## Environment Variables

- `RUST_LOG` - Logging level (default: `info`)
- `CONFIG_FILE` - Configuration file path (default: `config.toml`)

## Logging

The application uses structured logging with tracing. Log levels:
- `error` - Critical errors
- `warn` - Warnings and recoverable errors
- `info` - General information
- `debug` - Detailed debugging information

Example log configuration:
```bash
RUST_LOG=samplefarm_backend=debug,tower_http=info cargo run
```

## License

See the main project LICENSE file for details.
