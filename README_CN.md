# SampleFarm - 样本农场

一个专业的恶意样本分析平台，提供安全的样本管理、任务编排以及与多个分析沙箱（包括CAPE和CFG）的集成。

## 🚀 功能特性

### 核心功能
- **🔒 安全样本管理**：上传、存储和管理恶意软件样本，包含完整的元数据信息
- **📋 任务编排**：创建和监控分析任务，实时跟踪进度状态
- **🏗️ 沙箱集成**：通过API动态集成CAPE和CFG沙箱实例
- **📁 文件处理**：高级文件处理功能，包括验证、哈希、去重和ZIP文件解压
- **🌐 RESTful API**：完整的REST API，配备OpenAPI/Swagger文档
- **💾 稳健存储**：MinIO/S3兼容对象存储，配合PostgreSQL数据库
- **🔄 健康监控**：内置所有系统组件的健康检查
- **🚀 启动恢复**：系统重启时自动恢复中断的任务

### 分析能力
- **CAPE沙箱**：高级恶意软件分析，包含行为监控和威胁检测
- **CFG分析**：控制流图提取和恶意软件样本的嵌入生成
- **多实例支持**：跨多个沙箱实例的负载均衡
- **批处理**：高效处理多个样本的并行分析
- **实时状态**：分析进度和结果的实时更新

## 🏗️ 架构设计

SampleFarm遵循现代微服务架构，清晰分离各个模块职责：

```
SampleFarm/
├── backend/           # Rust后端API服务器
├── frontend/          # Next.js Web界面
├── client/            # Python批量上传工具
├── cfg/               # CFG提取和嵌入服务
└── database/          # PostgreSQL数据库脚本
```

### 技术栈

#### 后端 (Rust)
- **框架**：Axum异步Web框架
- **版本**：Rust 2024 Edition（最新版本）
- **数据库**：PostgreSQL with SQLx
- **存储**：MinIO/S3兼容对象存储
- **API**：OpenAPI 3.0 with Swagger UI
- **日志**：基于tracing的结构化日志

#### 前端 (Next.js)
- **框架**：Next.js 15 with App Router（最新版本）
- **语言**：TypeScript保证类型安全
- **UI**：Tailwind CSS + shadcn/ui组件
- **状态管理**：Zustand + TanStack Query
- **实时更新**：实时状态更新功能

#### 客户端工具
- **语言**：Python 3.8+
- **包管理器**：uv用于依赖管理
- **功能**：批量上传及进度跟踪

#### CFG分析服务 (Python)
- **框架**：FastAPI REST API
- **AI模型**：Palmtree预训练Transformer模型
- **反汇编器**：IDA Pro Linux（需要外部商业许可）
- **GPU支持**：NVIDIA CUDA加速处理
- **容器**：Docker with NVIDIA runtime
- **功能**：PE到ASM转换、CFG提取、嵌入向量生成

## 📦 快速开始

### 前置条件
- Docker and Docker Compose
- PostgreSQL 14+
- MinIO或S3兼容存储
- **CFG分析功能**（可选）：
  - NVIDIA GPU with CUDA支持
  - IDA Pro Linux许可证（商业软件）
  - 至少4GB GPU内存

### 1. 克隆代码库
```bash
git clone https://github.com/your-org/samplefarm.git
cd samplefarm
```

### 2. 配置环境
```bash
# 复制配置模板
cp backend/config.example.toml backend/config.toml
cp frontend/env.example frontend/.env.local

# 编辑配置文件
```

### 3. 初始化数据库
```bash
# 运行数据库初始化脚本
psql -h localhost -U samplefarm_user -d samplefarm -f database/deploy.sql
```

### 4. 启动服务
```bash
# 使用Docker Compose启动所有服务
docker-compose up -d

# 或者单独启动服务用于开发
cd backend && cargo run
cd frontend && npm run dev
```

### 5. 访问应用
- **Web界面**：http://localhost:3000
- **API文档**：http://localhost:8080/swagger-ui/
- **健康检查**：http://localhost:8080/health
- **CFG服务**（如已部署）：http://localhost:17777

## 🔧 配置说明

### 后端配置 (`backend/config.toml`)
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

### 前端配置 (`frontend/public/config.json`)
```json
{
  "backend": {
    "url": "http://localhost:8080",
    "timeout": 300000,
    "retries": 3
  },
  "app": {
    "title": "SampleFarm - 样本管理系统",
    "description": "专业的恶意样本管理和分析平台",
    "version": "0.1.0"
  },
  "ui": {
    "theme": "light",
    "pageSize": 20,
    "maxFileSize": "1GB"
  }
}
```

## 🧬 CFG分析服务

### 概述

CFG（控制流图）分析服务是一个专门的组件，用于从PE（可移植可执行文件）文件中提取控制流图，并使用先进的机器学习技术生成语义嵌入向量。

### 核心功能

- **PE文件分析**：使用IDA Pro自动反汇编Windows PE文件
- **CFG提取**：从汇编代码中提取高级控制流图
- **嵌入向量生成**：使用Palmtree预训练Transformer模型生成语义嵌入
- **GPU加速**：NVIDIA CUDA支持更快的处理速度
- **REST API**：与SampleFarm主平台集成的完整REST API

### 架构

```
PE文件 → IDA Pro → ASM文件 → CFG提取器 → 图结构 → Palmtree模型 → 嵌入向量
```

1. **PE到ASM**：IDA Pro将PE文件反汇编为汇编语言
2. **ASM到CFG**：自定义解析器从汇编代码提取控制流图
3. **CFG到嵌入**：Palmtree模型生成语义嵌入向量
4. **存储**：结果存储为压缩NumPy数组（.npz文件）

### 前置要求

#### 必需软件
- **IDA Pro Linux**：必须安装在`cfg/ida91-linux/`目录下
  - 从Hex-Rays下载IDA Pro Linux版本
  - 解压到`cfg/ida91-linux/`保持目录结构
  - 确保`idat`可执行文件位于`cfg/ida91-linux/idat`
  - 通过手动运行`idapyswitch`和`idat`完成许可证激活

#### 硬件要求
- **NVIDIA GPU**：Palmtree模型推理必需
- **内存**：推荐至少4GB RAM
- **存储**：临时文件至少需要1GB空余空间

### 安装和部署

#### 1. IDA Pro安装
```bash
# 进入CFG目录
cd cfg/

# 创建ida91-linux目录
mkdir -p ida91-linux

# 将你的IDA Pro Linux安装解压到此目录
# 确保以下目录结构：
# ida91-linux/
# ├── idat           # IDA命令行可执行文件
# ├── idapyswitch    # Python环境切换器
# └── [其他IDA文件...]
```

#### 2. Docker部署
```bash
# 进入CFG目录
cd cfg/

# 构建容器
docker-compose build

# 启动服务
docker-compose up -d

# 验证GPU可用性
docker exec -it malware-detection-api python3 -c "import torch; print(torch.cuda.is_available())"
```

#### 3. IDA Pro许可证激活
```bash
# 进入容器
docker exec -it malware-detection-api bash

# 设置IDA环境
cd ida91-linux
./idapyswitch  # 选择默认Python环境
./idat         # 激活许可证（按提示操作）
```

### 配置

CFG服务运行在17777端口，包含以下关键设置：

- **处理超时**：PE→ASM 10分钟，ASM→CFG 10分钟
- **并发任务数**：1（可配置）
- **最小磁盘空间**：处理需要1GB
- **临时存储**：`cfg/temp/`目录

### API集成

CFG服务提供完整的REST API，与SampleFarm的任务编排系统无缝集成。主要端点包括：

- `POST /preprocess_pe` - 上传PE文件并开始分析
- `GET /task/{task_id}` - 检查处理状态
- `GET /result/{task_id}` - 获取分析结果
- `GET /download/{task_id}/{filename}` - 下载结果文件
- `GET /system/status` - 检查系统健康状况和容量

详细的API文档请参见`cfg/README.md`。

### 输出文件

CFG分析生成两个主要输出文件：

1. **图文件**（`graph_xxx.npz`）：包含提取的控制流图结构
2. **稀疏矩阵**（`graph_xxx_sparse_matrix.npz`）：用于高效处理的压缩表示

这些文件可用于进一步的恶意软件分析、分类或研究用途。

## 🛠️ 开发指南

### 后端开发
```bash
cd backend

# 安装Rust依赖
cargo build

# 运行测试
cargo test

# 启动开发服务器
cargo run

# 格式化代码
cargo fmt

# 运行linter
cargo clippy
```

### 前端开发
```bash
cd frontend

# 安装依赖
npm install

# 启动开发服务器
npm run dev

# 构建生产版本
npm run build

# 运行linter
npm run lint
```

### 客户端开发
```bash
cd client

# 使用uv安装依赖
uv sync

# 运行批量上传工具
uv run python batch_upload.py --help
```

### CFG服务开发
```bash
cd cfg

# 安装Python依赖
pip install -r requirements.txt

# 在ida91-linux/目录设置IDA Pro（参见CFG分析服务章节）

# 本地运行API服务器
python api.py

# 运行测试
python test_api.py

# 手动处理样本
python process.py
```

## 🔌 API集成

### 样本上传
```bash
curl -X POST http://localhost:8080/api/samples/upload \
  -F "file=@sample.exe" \
  -F "label=malware" \
  -F "description=可疑的可执行文件"
```

### 创建分析任务
```bash
curl -X POST http://localhost:8080/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "分析任务",
    "sample_ids": ["uuid1", "uuid2"],
    "analyzer_type": "CAPE",
    "cape_instance_ids": ["instance1"]
  }'
```

### 检查任务状态
```bash
curl http://localhost:8080/api/tasks/{task_id}
```

## 🐳 部署

### Docker Compose (推荐)
```bash
# 生产环境部署（核心服务）
docker-compose -f docker-compose.yml up -d

# 包含数据库服务
docker-compose -f docker-compose.db.yml -f docker-compose.yml up -d

# 包含CFG分析服务（需要NVIDIA Docker运行时）
cd cfg && docker-compose up -d
```

### 手动部署
1. **数据库设置**：使用提供的脚本初始化PostgreSQL
2. **存储设置**：配置MinIO或S3兼容存储
3. **后端部署**：构建和部署Rust后端服务
4. **前端部署**：构建和部署Next.js应用
5. **CFG服务部署**（可选）：使用IDA Pro设置CFG分析服务
6. **反向代理**：配置nginx或类似工具用于生产环境

## 🔒 安全考虑

- **文件验证**：所有上传文件都经过验证和沙箱处理
- **访问控制**：基于角色的权限管理
- **安全存储**：敏感恶意软件样本的加密存储
- **API安全**：API端点的限流和身份验证
- **网络隔离**：沙箱实例运行在隔离环境中

## 🤝 贡献指南

1. Fork该代码库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m '添加某个功能'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启Pull Request

### 开发准则
- 遵循现有的代码风格和模式
- 为新功能编写完整的测试
- 更新API变更的相关文档
- 使用有意义的提交消息
- 确保所有测试通过后再提交

## 📄 许可证

本项目基于 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/deed.zh-hans) （知识共享 署名-非商业性使用-相同方式共享 4.0 国际）许可证发布。

### 许可证摘要

您可以自由地：
- **共享** — 在任何媒介以任何形式复制、发行本作品
- **演绎** — 修改、转换或以本作品为基础进行创作

惟须遵守下列条件：
- **署名** — 您必须给出适当的署名，提供指向本许可协议的链接，同时标明是否（对原始作品）作了修改
- **非商业性使用** — 您不得将本作品用于商业目的
- **相同方式共享** — 如果您再混合、转换或者基于本作品进行创作，您必须基于与原先许可协议相同的许可协议分发您贡献的作品

查看 [LICENSE](LICENSE) 文件了解完整许可证文本。

## 🆘 支持

- **问题反馈**：通过GitHub Issues报告问题和功能请求
- **API参考**：运行后端时访问 `/swagger-ui/` 查看API文档

## 🙏 致谢

- [CAPE Sandbox](https://capesandbox.com/) 提供恶意软件分析能力
- [MCBG项目](https://github.com/Bowen-n/MCBG) 提供CFG提取算法
- [PalmTree模型](https://github.com/palmtreemodel/PalmTree) 提供汇编代码嵌入技术
- [Rust社区](https://www.rust-lang.org/) 提供优秀的生态系统
- [Next.js团队](https://nextjs.org/) 提供现代Web框架
- 所有帮助改进SampleFarm的贡献者们

## 📊 系统要求

### 核心系统（后端+前端）

#### 最低要求
- **CPU**: 2核心
- **内存**: 4GB RAM
- **存储**: 20GB可用空间
- **网络**: 稳定的互联网连接

#### 推荐配置
- **CPU**: 4核心或更多
- **内存**: 8GB RAM或更多
- **存储**: 100GB SSD
- **网络**: 高带宽连接（用于大文件上传）

### CFG分析服务（可选）

#### 额外要求
- **GPU**: NVIDIA GPU with CUDA支持
- **GPU内存**: 最低4GB VRAM
- **CPU**: 额外2核心用于IDA Pro处理
- **内存**: 额外4GB RAM
- **存储**: 额外50GB用于IDA Pro、模型和临时文件
- **软件**: IDA Pro Linux许可证（商业软件）

## 🔧 故障排除

### 常见问题

1. **端口冲突**
   - 确保端口3000（前端）和8080（后端）未被占用
   - 修改配置文件中的端口设置

2. **数据库连接问题**
   - 检查PostgreSQL服务状态
   - 验证数据库配置和凭据

3. **存储访问问题**
   - 确认MinIO服务正在运行
   - 检查访问密钥和密钥配置

4. **内存不足**
   - 增加系统内存或调整配置中的并发设置
   - 监控系统资源使用情况

如需更多帮助，请在GitHub Issues中提交详细的错误信息和系统配置。
