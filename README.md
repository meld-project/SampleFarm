# SampleFarm

A comprehensive malware analysis platform that provides secure sample management, task orchestration, and integration with multiple analysis sandboxes including CAPE and CFG.

## 🚀 Features

### Core Capabilities
- **🔒 Secure Sample Management**: Upload, store, and manage malware samples with comprehensive metadata
- **📋 Task Orchestration**: Create and monitor analysis tasks with real-time progress tracking
- **🏗️ Sandbox Integration**: Dynamic integration with CAPE and CFG sandbox instances via API
- **📁 File Processing**: Advanced file handling with validation, hashing, deduplication, and ZIP extraction
- **🌐 RESTful API**: Comprehensive REST API with OpenAPI/Swagger documentation
- **💾 Robust Storage**: MinIO/S3-compatible object storage with PostgreSQL database
- **🔄 Health Monitoring**: Built-in health checks for all system components
- **🚀 Startup Recovery**: Automatic recovery of interrupted tasks on system restart

### Analysis Capabilities
- **CAPE Sandbox**: Advanced malware analysis with behavior monitoring and threat detection
- **CFG Analysis**: Control Flow Graph extraction and embedding generation for malware samples
- **Multi-Instance Support**: Load balancing across multiple sandbox instances
- **Batch Processing**: Efficient handling of multiple samples with parallel analysis
- **Real-time Status**: Live updates on analysis progress and results

## 🏗️ Architecture

SampleFarm follows a modern microservices architecture with clear separation of concerns:

```
SampleFarm/
├── backend/           # Rust-based API server
├── frontend/          # Next.js web interface  
├── client/            # Python batch upload tool
├── cfg/               # CFG extraction and embedding service
└── database/          # PostgreSQL schema and migrations
```

### Technology Stack

#### Backend (Rust)
- **Framework**: Axum async web framework
- **Edition**: Rust 2024 Edition (latest)
- **Database**: PostgreSQL with SQLx
- **Storage**: MinIO/S3 compatible object storage
- **API**: OpenAPI 3.0 with Swagger UI
- **Logging**: Structured logging with tracing

#### Frontend (Next.js)
- **Framework**: Next.js 15 with App Router (latest)
- **Language**: TypeScript for type safety
- **UI**: Tailwind CSS + shadcn/ui components
- **State**: Zustand + TanStack Query
- **Real-time**: Live status updates

#### Client Tools
- **Language**: Python 3.8+
- **Package Manager**: uv for dependency management
- **Features**: Batch upload with progress tracking

#### CFG Analysis Service (Python)
- **Framework**: FastAPI for REST API
- **AI Model**: Palmtree pre-trained transformer model
- **Disassembler**: IDA Pro Linux (required external dependency)
- **GPU Support**: NVIDIA CUDA for accelerated processing
- **Container**: Docker with NVIDIA runtime
- **Features**: PE to ASM conversion, CFG extraction, embedding generation

## 📦 Quick Start

### Prerequisites
- Docker and Docker Compose
- PostgreSQL 14+
- MinIO or S3-compatible storage
- **For CFG Analysis** (optional):
  - NVIDIA GPU with CUDA support
  - IDA Pro Linux license (commercial software)
  - At least 4GB GPU memory

### 1. Clone Repository
```bash
git clone https://github.com/your-org/samplefarm.git
cd samplefarm
```

### 2. Configure Environment
```bash
# Copy configuration templates
cp backend/config.example.toml backend/config.toml
cp frontend/env.example frontend/.env.local

# Edit configuration files with your settings
```

### 3. Initialize Database
```bash
# Run database initialization scripts
psql -h localhost -U samplefarm_user -d samplefarm -f database/deploy.sql
```

### 4. Start Services
```bash
# Start all services with Docker Compose
docker-compose up -d

# Or start individual services for development
cd backend && cargo run
cd frontend && npm run dev
```

### 5. Access Application
- **Web Interface**: http://localhost:3000
- **API Documentation**: http://localhost:8080/swagger-ui/
- **Health Check**: http://localhost:8080/health
- **CFG Service** (if deployed): http://localhost:17777

## 🔧 Configuration

### Backend Configuration (`backend/config.toml`)
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

### Frontend Configuration (`frontend/public/config.json`)
```json
{
  "backend": {
    "url": "http://localhost:8080",
    "timeout": 300000,
    "retries": 3
  },
  "app": {
    "title": "SampleFarm - Sample Management System",
    "description": "Professional malware sample management and analysis platform",
    "version": "0.1.0"
  },
  "ui": {
    "theme": "light",
    "pageSize": 20,
    "maxFileSize": "1GB"
  }
}
```

## 📚 Documentation

### Component Documentation
- [Backend README](backend/README.md) - Rust backend service
- [Frontend README](frontend/README.md) - Next.js web interface
- [Client README](client/README.md) - Python batch upload tool
- [CFG Service README](cfg/README.md) - Control Flow Graph analysis service
- [Frontend Development Progress](frontend/DEVELOPMENT_PROGRESS.md) - Current development status
- [Frontend Configuration Guide](frontend/CONFIG.md) - Configuration details

## 🧬 CFG Analysis Service

### Overview

The CFG (Control Flow Graph) Analysis Service is a specialized component that extracts control flow graphs from PE (Portable Executable) files and generates semantic embeddings using advanced machine learning techniques.

### Key Features

- **PE File Analysis**: Automatic disassembly of Windows PE files using IDA Pro
- **CFG Extraction**: Advanced control flow graph extraction from assembly code
- **Embedding Generation**: Semantic embeddings using the Palmtree pre-trained transformer model
- **GPU Acceleration**: NVIDIA CUDA support for faster processing
- **REST API**: Complete REST API for integration with the main SampleFarm platform

### Architecture

```
PE File → IDA Pro → ASM File → CFG Extractor → Graph → Palmtree Model → Embeddings
```

1. **PE to ASM**: IDA Pro disassembles PE files into assembly language
2. **ASM to CFG**: Custom parser extracts control flow graphs from assembly
3. **CFG to Embeddings**: Palmtree model generates semantic embeddings
4. **Storage**: Results stored as compressed NumPy arrays (.npz files)

### Prerequisites

#### Required Software
- **IDA Pro Linux**: Must be installed in `cfg/ida91-linux/` directory
  - Download IDA Pro Linux version from Hex-Rays
  - Extract to `cfg/ida91-linux/` maintaining directory structure
  - Ensure `idat` executable is available at `cfg/ida91-linux/idat`
  - Complete license activation by running `idapyswitch` and `idat` manually

#### Hardware Requirements
- **NVIDIA GPU**: Required for Palmtree model inference
- **Memory**: Minimum 4GB RAM recommended
- **Storage**: At least 1GB free space for temporary files

### Setup and Deployment

#### 1. IDA Pro Installation
```bash
# Navigate to CFG directory
cd cfg/

# Create ida91-linux directory
mkdir -p ida91-linux

# Extract your IDA Pro Linux installation to this directory
# Ensure the following structure:
# ida91-linux/
# ├── idat           # IDA command-line executable
# ├── idapyswitch    # Python environment switcher
# └── [other IDA files...]
```

#### 2. Docker Deployment
```bash
# Navigate to CFG directory
cd cfg/

# Build the container
docker-compose build

# Start the service
docker-compose up -d

# Verify GPU availability
docker exec -it malware-detection-api python3 -c "import torch; print(torch.cuda.is_available())"
```

#### 3. IDA Pro License Activation
```bash
# Enter the container
docker exec -it malware-detection-api bash

# Set up IDA environment
cd ida91-linux
./idapyswitch  # Select default Python environment
./idat         # Activate license (follow prompts)
```

### Configuration

The CFG service runs on port 17777 and includes the following key settings:

- **Processing Timeout**: 10 minutes for PE→ASM, 10 minutes for ASM→CFG
- **Concurrent Tasks**: 1 (configurable)
- **Minimum Disk Space**: 1GB required for processing
- **Temporary Storage**: `cfg/temp/` directory

### API Integration

The CFG service provides a complete REST API that integrates seamlessly with SampleFarm's task orchestration system. Key endpoints include:

- `POST /preprocess_pe` - Upload PE file and start analysis
- `GET /task/{task_id}` - Check processing status
- `GET /result/{task_id}` - Retrieve analysis results
- `GET /download/{task_id}/{filename}` - Download result files
- `GET /system/status` - Check system health and capacity

For detailed API documentation, see `cfg/README.md`.

### Output Files

The CFG analysis produces two main output files:

1. **Graph file** (`graph_xxx.npz`): Contains the extracted control flow graph structure
2. **Sparse matrix** (`graph_xxx_sparse_matrix.npz`): Compressed representation for efficient processing

These files can be used for further malware analysis, classification, or research purposes.

## 🛠️ Development

### Backend Development
```bash
cd backend

# Install Rust dependencies
cargo build

# Run tests
cargo test

# Start development server
cargo run

# Format code
cargo fmt

# Run linter
cargo clippy
```

### Frontend Development
```bash
cd frontend

# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Run linter
npm run lint
```

### Client Development
```bash
cd client

# Install dependencies with uv
uv sync

# Run batch upload tool
uv run python batch_upload.py --help
```

### CFG Service Development
```bash
cd cfg

# Install Python dependencies
pip install -r requirements.txt

# Set up IDA Pro in ida91-linux/ directory (see CFG Analysis Service section)

# Run the API server locally
python api.py

# Run tests
python test_api.py

# Process samples manually
python process.py
```

## 🔌 API Integration

### Sample Upload
```bash
curl -X POST http://localhost:8080/api/samples/upload \
  -F "file=@sample.exe" \
  -F "label=malware" \
  -F "description=Suspicious executable"
```

### Create Analysis Task
```bash
curl -X POST http://localhost:8080/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Analysis Task",
    "sample_ids": ["uuid1", "uuid2"],
    "analyzer_type": "CAPE",
    "cape_instance_ids": ["instance1"]
  }'
```

### Check Task Status
```bash
curl http://localhost:8080/api/tasks/{task_id}
```

## 🐳 Deployment

### Docker Compose (Recommended)
```bash
# Production deployment (core services)
docker-compose -f docker-compose.yml up -d

# With database services
docker-compose -f docker-compose.db.yml -f docker-compose.yml up -d

# Including CFG analysis service (requires NVIDIA Docker runtime)
cd cfg && docker-compose up -d
```

### Manual Deployment
1. **Database Setup**: Initialize PostgreSQL with provided schemas
2. **Storage Setup**: Configure MinIO or S3-compatible storage
3. **Backend Deployment**: Build and deploy Rust backend service
4. **Frontend Deployment**: Build and deploy Next.js application
5. **CFG Service Deployment** (optional): Set up CFG analysis service with IDA Pro
6. **Reverse Proxy**: Configure nginx or similar for production

## 🔒 Security Considerations

- **File Validation**: All uploaded files are validated and sandboxed
- **Access Control**: Role-based permissions for different user types
- **Secure Storage**: Encrypted storage for sensitive malware samples
- **API Security**: Rate limiting and authentication for API endpoints
- **Network Isolation**: Sandbox instances run in isolated environments

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines
- Follow existing code style and patterns
- Write comprehensive tests for new features
- Update documentation for API changes
- Use meaningful commit messages
- Ensure all tests pass before submitting

## 📄 License

This project is licensed under the [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) (Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International) license.

### License Summary

You are free to:
- **Share** — copy and redistribute the material in any medium or format
- **Adapt** — remix, transform, and build upon the material

Under the following terms:
- **Attribution** — You must give appropriate credit and indicate if changes were made
- **NonCommercial** — You may not use the material for commercial purposes
- **ShareAlike** — If you remix or build upon the material, you must distribute under the same license

See the [LICENSE](LICENSE) file for full license text.

## 🆘 Support

- **Documentation**: Check the `docs/` directory for detailed guides
- **Issues**: Report bugs and feature requests via GitHub Issues
- **API Reference**: Access Swagger UI at `/swagger-ui/` when running the backend

## 📊 System Requirements

### Core System (Backend + Frontend)

#### Minimum Requirements
- **CPU**: 2 cores
- **Memory**: 4GB RAM
- **Storage**: 20GB available space
- **Network**: Stable internet connection

#### Recommended Configuration
- **CPU**: 4 cores or more
- **Memory**: 8GB RAM or more
- **Storage**: 100GB SSD
- **Network**: High bandwidth connection for large file uploads

### CFG Analysis Service (Optional)

#### Additional Requirements
- **GPU**: NVIDIA GPU with CUDA support
- **GPU Memory**: Minimum 4GB VRAM
- **CPU**: Additional 2 cores for IDA Pro processing
- **Memory**: Additional 4GB RAM
- **Storage**: Additional 50GB for IDA Pro, models, and temporary files
- **Software**: IDA Pro Linux license (commercial)

## 🙏 Acknowledgments

- [CAPE Sandbox](https://capesandbox.com/) for malware analysis capabilities
- [MCBG Project](https://github.com/Bowen-n/MCBG) for CFG extraction algorithms
- [PalmTree Model](https://github.com/palmtreemodel/PalmTree) for assembly code embeddings
- [Rust Community](https://www.rust-lang.org/) for the excellent ecosystem
- [Next.js Team](https://nextjs.org/) for the modern web framework
- All contributors who help improve SampleFarm
