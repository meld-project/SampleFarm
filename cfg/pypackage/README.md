# Python离线安装包说明 / Python Offline Packages

## 中文说明

由于Python wheel包文件体积过大（总计约2.76GB），这些文件已从Git仓库中移除。这些包是CFG分析服务在离线环境下运行所需的依赖。

### 获取方式

如果您需要在离线环境下部署CFG服务，请通过以下方式获取这些包：

1. **从发布页面下载**（推荐）
   - 访问项目的GitHub Releases页面
   - 下载 `pypackage.tar.gz` 压缩包
   - 解压到 `cfg/pypackage/` 目录

2. **手动下载**
   - 根据 `cfg/requirements.txt` 中的依赖列表
   - 使用pip download命令下载所需包：
   ```bash
   cd cfg
   pip download -r requirements.txt -d pypackage/ --platform manylinux2014_x86_64 --python-version 38 --only-binary :all:
   ```

### 包含的主要依赖

- PyTorch 2.4.1 with CUDA 11.8 support
- NVIDIA CUDA runtime libraries
- Pandas, NumPy, SciPy等科学计算库
- FastAPI及相关Web框架
- Palmtree模型所需的BERT相关包

---

## English Instructions

Due to the large size of Python wheel packages (approximately 2.76GB in total), these files have been removed from the Git repository. These packages are required dependencies for running the CFG analysis service in offline environments.

### How to Obtain

If you need to deploy the CFG service in an offline environment, please obtain these packages through:

1. **Download from Release Page** (Recommended)
   - Visit the project's GitHub Releases page
   - Download the `pypackage.tar.gz` archive
   - Extract to the `cfg/pypackage/` directory

2. **Manual Download**
   - Based on the dependency list in `cfg/requirements.txt`
   - Use pip download command to get required packages:
   ```bash
   cd cfg
   pip download -r requirements.txt -d pypackage/ --platform manylinux2014_x86_64 --python-version 38 --only-binary :all:
   ```

### Main Dependencies Included

- PyTorch 2.4.1 with CUDA 11.8 support
- NVIDIA CUDA runtime libraries
- Scientific computing libraries like Pandas, NumPy, SciPy
- FastAPI and related web frameworks
- BERT-related packages required for Palmtree model
