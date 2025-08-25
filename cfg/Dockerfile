# 使用支持CUDA的Python基础镜像（兼容PyTorch 2.4.1和Python 3.8）
FROM nvidia/cuda:11.8.0-runtime-ubuntu20.04

# 安装系统依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.8 \
    python3-pip \
    python3.8-venv \
    python3-dev \
    gcc \
    g++ \
    make \
    p7zip-full \
    && rm -rf /var/lib/apt/lists/*

# 设置Python 3.8为默认
RUN update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.8 1

# 设置工作目录
WORKDIR /app

# 复制Python依赖包分卷压缩文件
COPY pypackage/pypackage.7z.* /tmp/

# 解压并安装Python依赖包
RUN mkdir -p /tmp/pypackage && \
    cd /tmp && \
    7z x pypackage.7z.001 -o/tmp/pypackage && \
    pip install --no-index --find-links=/tmp/pypackage /tmp/pypackage/*.whl && \
    rm -rf /tmp/pypackage /tmp/pypackage.7z.*

# 验证依赖是否正确安装及CUDA可用性
RUN python3 -c "import sys; print('Python version:', sys.version); import torch; print('PyTorch version:', torch.__version__); print('CUDA available:', torch.cuda.is_available())"

# 复制项目文件
COPY . .

# 暴露端口
EXPOSE 17777

# 设置环境变量
ENV PYTHONUNBUFFERED=1

# 启动命令
CMD ["python3", "api.py"]