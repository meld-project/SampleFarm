# Malware CFG Embedding API

## API端点

### 1. 上传PE文件并启动预处理任务
- **URL**: `/preprocess_pe`
- **方法**: `POST`
- **参数**: 
  - `file`: 要上传的PE文件 (必需)
  - `task_id`: 任务ID，必须为文件的sha256值 (必需)
  - `label`: 样本标签，0为白样本，1为黑样本 (必需)
- **请求示例**: 
  ```bash
  curl -X POST "http://localhost:17777/preprocess_pe" -F "file=@/path/to/your/pe_file" -F "task_id=your_file_sha256" -F "label=0"
  ```
- **成功响应**: 
  ```json
  {
    "success": true,
    "message": "任务已提交，当前队列位置: 0",
    "task_id": "your_file_sha256"
  }
  ```
- **失败响应**: 
  ```json
  {
    "success": false,
    "message": "不支持的文件格式，需要有效的PE文件"
    "task_id": null
  }
  ```

### 2. 查询任务状态
- **URL**: `/task/{task_id}`
- **方法**: `GET`
- **参数**: 
  - `task_id`: 任务ID (必需)
- **请求示例**: 
  ```bash
  curl "http://localhost:17777/task/abcd1234"
  ```
- **响应示例**: 
  ```json
  {
    "task_id": "abcd1234",
    "status": "processing",
    "message": "正在提取CFG...",
    "result_files": null
  }
  ```
  或
  ```json
  {
    "task_id": "abcd1234",
    "status": "completed",
    "message": "处理完成",
    "result_files": {
      "graph": "graph_xxx.npz",
      "sparse_matrix": "graph_xxx_sparse_matrix.npz"
    }
  }
  ```

### 3. 获取处理结果
- **URL**: `/result/{task_id}`
- **方法**: `GET`
- **参数**: 
  - `task_id`: 任务ID (必需)
- **请求示例**: 
  ```bash
  curl "http://localhost:17777/result/abcd1234"
  ```
- **成功响应**: 
  ```json
  {
    "success": true,
    "message": "处理完成",
    "task_id": "abcd1234",
    "result_files": {
      "graph": "graph_xxx.npz",
      "sparse_matrix": "graph_xxx_sparse_matrix.npz"
    }
  }
  ```
- **失败响应**: 
  ```json
  {
    "success": false,
    "message": "处理失败: 超时",
    "task_id": "abcd1234",
    "result_files": null
  }
  ```

### 4. 下载结果文件
- **URL**: `/download/{task_id}/{filename}`
- **方法**: `GET`
- **参数**: 
  - `task_id`: 任务ID (必需)
  - `filename`: 要下载的文件名 (必需)
- **请求示例**: 
  ```bash
  curl -O "http://localhost:17777/download/your_file_sha256/graph_xxx.npz"
  ```
- **成功响应**: 返回文件内容
- **失败响应**: 
  ```json
  {
    "detail": "文件不存在"
  }
  ```

### 5. 获取系统状态
- **URL**: `/system/status`
- **方法**: `GET`
- **描述**: 获取系统状态，包括活跃任务数、队列长度和磁盘空间信息
- **响应示例**: 
  ```json
  {
    "active_tasks": 1,
    "queue_length": 3,
    "max_concurrent_tasks": 1,
    "status": "normal",
    "disk": {
      "total_gb": 40.0,
      "used_gb": 15.2,
      "free_gb": 24.8,
      "usage_percent": 38.0,
      "min_required_gb": 1,
      "disk_enough": true
    }
  }
- **请求示例**: 
  ```bash
  curl -X GET "http://localhost:17777/system/status"
  ```
- **成功响应**: 
  ```json
  {
    "active_tasks": 1,
    "queue_length": 3,
    "max_concurrent_tasks": 1,
    "status": "normal",
    "disk": {
      "total_gb": 40.0,
      "used_gb": 15.2,
      "free_gb": 24.8,
      "usage_percent": 38.0,
      "min_required_gb": 1,
      "disk_enough": true
    }
  }
  ```
- **失败响应**: 
  ```json
  {
    "detail": "系统状态获取失败"
  }
  ```

## API端点python请求示例
参考`test_api.py`，包含上传PE文件、查询任务状态、获取处理结果、下载结果文件和获取系统状态的示例代码。

## 部署步骤
- 1.构建镜像： docker-compose build
- 2.启动服务： docker-compose up -d
- 3.验证 GPU 可用性： docker exec -it malware-detection-api python3 -c "import torch; print(torch.cuda.is_available())"
- 4.进入docker环境手动执行ida91-linux目录下的idapyswitch选择默认环境
- 5.进入docker环境手动执行ida91-linux目录下的idat，全部点ok激活license

## 注意事项
- 1. 处理超时设置为PE转ASM 10分钟、ASM转CFG 10分钟，超过此时间的任务将被标记为失败。
- 2. 服务器需要至少1GB的空闲磁盘空间才能处理新任务。
- 3. 临时文件存储在`temp/`目录下，定期清理可释放磁盘空间。
- 4. 为确保服务稳定性，建议使用进程管理器如systemd或supervisor管理服务。
