# Python离线安装包说明 / Python Offline Packages

## 中文说明

由于Python依赖包体积较大，这些文件已打包为分卷压缩包（约2.8GB，包含47个依赖包）。

### 获取方式

1. **解压本地分卷压缩包**
   ```bash
   cd cfg/pypackage
   7z x pypackage.7z.001
   ```

2. **重新下载**（如果需要）
   ```bash
   cd cfg
   pip download -r requirements.txt -d pypackage/ --platform manylinux2014_x86_64 --python-version 38 --only-binary :all:
   ```

### 主要依赖

- PyTorch 2.4.1 + CUDA 11.8
- NVIDIA CUDA运行库
- 科学计算库（NumPy, Pandas, SciPy等）
- Web框架（FastAPI, Uvicorn等）
- BERT模型支持

---

## English Instructions

Python dependencies are packaged as split archives (approximately 2.8GB, containing 47 packages).

### How to Use

1. **Extract local split archive**
   ```bash
   cd cfg/pypackage
   7z x pypackage.7z.001
   ```

2. **Re-download** (if needed)
   ```bash
   cd cfg
   pip download -r requirements.txt -d pypackage/ --platform manylinux2014_x86_64 --python-version 38 --only-binary :all:
   ```

### Main Dependencies

- PyTorch 2.4.1 + CUDA 11.8
- NVIDIA CUDA runtime libraries
- Scientific computing (NumPy, Pandas, SciPy, etc.)
- Web framework (FastAPI, Uvicorn, etc.)
- BERT model support