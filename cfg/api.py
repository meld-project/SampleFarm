import os
import sys
import time
import shutil
import logging
import threading
import concurrent.futures
from queue import Queue
from pathlib import Path
from typing import Optional, Dict, Any, List

# 导入自定义模块
from extract_cfg import extract_cfg
from generate_embeddings import generate_embeddings

from fastapi import FastAPI, UploadFile, File, Form, HTTPException, BackgroundTasks
from fastapi.responses import FileResponse
from pydantic import BaseModel

# 导入mkasm模块
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from process import process_single_file

# MKASM配置
MKASM_DIR = Path(__file__).parent
PE_SAMPLES_DIR = MKASM_DIR / "samples"

# 确保mkasm目录存在
PE_SAMPLES_DIR.mkdir(parents=True, exist_ok=True)

# 配置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'api.log')),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger('api')

# 初始化FastAPI应用
app = FastAPI(title="Malware CFG Embedding API", description="API for converting ASM files to NPZ embeddings")

# 配置路径和常量
PROCESS_DIR = Path("./temp/processing")
RESULT_DIR = Path("./temp/results")
LABELS_FILE = Path("./trainLabels.csv")

# 处理超时设置 (单位: 秒)
PROCESS_TIMEOUT = 600  # 10分钟

# 确保目录存在
for dir_path in [PROCESS_DIR, RESULT_DIR]:
    dir_path.mkdir(parents=True, exist_ok=True)

# 任务状态存储
TASKS: Dict[str, Dict[str, Any]] = {}

# 显存控制配置
MAX_CONCURRENT_TASKS = 1  # 最大并发任务数，根据显存大小调整
TASK_QUEUE = Queue()  # 任务队列 (元素格式: (task_id, file_path, file_type, label))
PROCESSING_LOCK = threading.Lock()  # 处理锁
ACTIVE_TASKS: List[str] = []  # 活跃任务列表

# 启动工作线程
def worker():
    while True:
        task_id, file_path, file_type, label = TASK_QUEUE.get()
        with PROCESSING_LOCK:
            ACTIVE_TASKS.append(task_id)
        try:
            if file_type == 'pe':
                process_pe_file(task_id, file_path, label)
            else:
                process_asm_file(task_id, file_path, label)
        finally:
            with PROCESSING_LOCK:
                if task_id in ACTIVE_TASKS:
                    ACTIVE_TASKS.remove(task_id)
            TASK_QUEUE.task_done()

# 启动工作线程
for _ in range(MAX_CONCURRENT_TASKS):
    threading.Thread(target=worker, daemon=True).start()

# 定义任务状态模型
class TaskStatus(BaseModel):
    task_id: str
    status: str  # pending, processing, completed, failed
    message: Optional[str] = None
    result_files: Optional[Dict[str, str]] = None

# 定义处理结果模型
class ProcessResult(BaseModel):
    success: bool
    message: str
    task_id: str
    result_files: Optional[Dict[str, str]] = None

# 处理超时设置 (10分钟)
PROCESS_TIMEOUT = 600

# 磁盘空间配置
MIN_DISK_SPACE_GB = 1  # 最小所需磁盘空间(GB)


def check_disk_space() -> None:
    """
    检查磁盘空间是否充足，若小于MIN_DISK_SPACE_GB则抛出异常
    """
    # 获取当前磁盘的使用情况
    disk = shutil.disk_usage('/')
    # 计算可用空间(GB)
    free_space_gb = disk.free / (1024 ** 3)
    
    if free_space_gb < MIN_DISK_SPACE_GB:
        logger.error(f"磁盘空间不足: 可用 {free_space_gb:.2f}GB, 所需 {MIN_DISK_SPACE_GB}GB")
        raise HTTPException(
            status_code=507,
            detail=f"服务器磁盘空间不足: 可用 {free_space_gb:.2f}GB, 所需 {MIN_DISK_SPACE_GB}GB"
        )


def process_pe_file(task_id: str, pe_file_path: str, label: int) -> None:
    """处理PE文件，转换为NPZ文件"""
    try:
        # 更新任务状态为处理中
        TASKS[task_id].update({
            "status": "processing",
            "message": "开始处理PE文件..."
        })

        # 1. PE转ASM
        TASKS[task_id]["message"] = "正在将PE文件转换为ASM..."
        asm_output_dir = PROCESS_DIR / task_id / "asm"
        asm_output_dir.mkdir(parents=True, exist_ok=True)

        # 调用mkasm的process_single_file处理PE文件，设置超时
        logger.info(f"任务 {task_id}: 开始调用process_single_file处理PE文件")
        with concurrent.futures.ThreadPoolExecutor() as executor:
            future = executor.submit(
                process_single_file,
                input_path=pe_file_path,
                output_dir=str(asm_output_dir)
            )
            try:
                # 设置10分钟超时
                future.result(timeout=PROCESS_TIMEOUT)
            except concurrent.futures.TimeoutError:
                logger.warning(f"任务 {task_id}: PE转ASM处理超时 (超过10分钟)")
                raise Exception("PE转ASM处理超时 (超过10分钟)")

        # 查找生成的ASM文件
        asm_file_path = asm_output_dir / (os.path.basename(pe_file_path) + '.asm')
        if not asm_file_path.exists():
            raise Exception("PE转ASM失败，未找到生成的ASM文件")
        logger.info(f"任务 {task_id}: PE转ASM成功，生成文件: {asm_file_path}")

        # 2. 处理ASM文件
        process_asm_file(task_id, asm_file_path, label)

    except Exception as e:
        logger.error(f"任务 {task_id}: PE文件处理失败: {str(e)}")
        error_msg = str(e)
        if "超时" in error_msg:
            TASKS[task_id].update({
                "status": "failed",
                "message": "处理超时 (超过10分钟)"
            })
        else:
            TASKS[task_id].update({
                "status": "failed",
                "message": f"PE文件处理失败: {str(e)}"
            })
        filename = os.path.splitext(pe_file_path)[0]
        for ext in ['.i64', '.idb', '.id0', '.id1', '.id2', '.nam', '.til']:
            file_path = filename + ext
            print(file_path)
            if os.path.exists(file_path):
                os.remove(file_path)
                logger.info(f"Deleted {file_path}")
    finally:
        # 清理临时PE文件
        if os.path.exists(pe_file_path):
            os.remove(pe_file_path)

def process_asm_file(task_id: str, asm_file_path: str, label: int) -> None:
    """处理ASM文件，转换为NPZ文件"""
    try:
        # 更新任务状态为处理中
        TASKS[task_id].update({
            "status": "processing",
            "message": "开始处理ASM文件..."
        })

        # 1. 从ASM文件提取CFG (直接调用函数)
        cfg_output_dir = PROCESS_DIR / task_id / "raw_cfg"
        cfg_output_dir.mkdir(parents=True, exist_ok=True)

        TASKS[task_id]["message"] = "正在提取CFG..."
        logger.info(f"任务 {task_id}: 开始提取CFG")
        with concurrent.futures.ThreadPoolExecutor() as executor:
            future = executor.submit(
                extract_cfg,
                input_file=str(asm_file_path),
                output_dir=str(cfg_output_dir)
            )
            try:
                extract_success = future.result(timeout=PROCESS_TIMEOUT)
            except concurrent.futures.TimeoutError:
                logger.warning(f"任务 {task_id}: 提取CFG超时 (超过{PROCESS_TIMEOUT/60}分钟)")
                raise Exception(f"提取CFG超时 (超过{PROCESS_TIMEOUT/60}分钟)")

        if not extract_success:
            logger.error(f"任务 {task_id}: 提取CFG失败")
            raise Exception("提取CFG失败")
        logger.info(f"任务 {task_id}: 提取CFG成功")

        # 2. 生成NPZ文件
        TASKS[task_id]["message"] = "正在生成NPZ文件..."
        npz_output_dir = RESULT_DIR / task_id
        npz_output_dir.mkdir(parents=True, exist_ok=True)

        # 查找生成的JSON文件
        json_files = list(cfg_output_dir.glob("*.json"))
        if not json_files:
            raise Exception("未找到生成的CFG JSON文件")

        # 为简化，我们假设只有一个JSON文件
        json_file_path = json_files[0]

        # 现在生成NPZ文件
        logger.info(f"任务 {task_id}: 开始生成NPZ文件")
        with concurrent.futures.ThreadPoolExecutor() as executor:
            future = executor.submit(
                generate_embeddings,
                input_file=str(json_file_path),
                output_dir=str(npz_output_dir),
                label_val=label
            )
            try:
                generate_success = future.result(timeout=PROCESS_TIMEOUT)
            except concurrent.futures.TimeoutError:
                logger.warning(f"任务 {task_id}: 生成NPZ文件超时 (超过{PROCESS_TIMEOUT/60}分钟)")
                raise Exception(f"生成NPZ文件超时 (超过{PROCESS_TIMEOUT/60}分钟)")

        if not generate_success:
            logger.error(f"任务 {task_id}: 生成NPZ文件失败")
            raise Exception("生成NPZ文件失败")
        logger.info(f"任务 {task_id}: 生成NPZ文件成功")

        # 3. 查找生成的NPZ文件
        npz_files = list(npz_output_dir.glob("*.npz"))
        if len(npz_files) < 2:
            raise Exception("未找到足够的NPZ文件")


        # 构建结果文件映射
        result_files = {
            "graph": str(npz_files[0].name),
            "sparse_matrix": str(npz_files[1].name)
        }

        # 更新任务状态为完成
        TASKS[task_id].update({
            "status": "completed",
            "message": "处理完成",
            "result_files": result_files
        })

    except concurrent.futures.TimeoutError:
        logger.warning(f"任务 {task_id}: 处理超时 (超过{PROCESS_TIMEOUT/60}分钟)")
        TASKS[task_id].update({
            "status": "failed",
            "message": f"处理超时 (超过{PROCESS_TIMEOUT/60}分钟)"
        })
    except Exception as e:
        logger.error(f"任务 {task_id}: 处理失败: {str(e)}")
        TASKS[task_id].update({
            "status": "failed",
            "message": f"处理失败: {str(e)}"
        })
    finally:
        # 清理临时文件
        if os.path.exists(PROCESS_DIR / task_id):
            shutil.rmtree(PROCESS_DIR / task_id)


@app.post("/preprocess_pe", response_model=ProcessResult)
async def preprocess_pe(
    background_tasks: BackgroundTasks,
    file: UploadFile = File(...),
    task_id: str = Form(...),  # 改为必填参数
    label: int = Form(..., ge=0)
):
    """
    上传PE文件并启动预处理任务

    参数:
    - background_tasks: FastAPI后台任务对象
    - file: 上传的PE文件 (.exe, .dll, .sys)
    - task_id: 可选的任务ID，若未提供则自动生成。若提供则必须唯一
    """
    # 检查磁盘空间
    check_disk_space()

    # 检查任务ID是否已存在
    if task_id in TASKS:
        raise HTTPException(status_code=400, detail=f"任务ID '{task_id}' 已存在")

    # 保存上传的文件
    file_path = PE_SAMPLES_DIR / f"{task_id}"
    with open(file_path, "wb") as buffer:
        shutil.copyfileobj(file.file, buffer)
    buffer.close()
    # 确保文件上传成功
    if not os.path.exists(file_path):
        raise HTTPException(status_code=500, detail="文件上传失败")
    # 设置文件权限
    os.chmod(file_path, 0o755)
    # 初始化任务状态
    TASKS[task_id] = {
        "status": "pending",
        "message": "PE文件已上传，等待处理...",
        "created_at": time.time(),
        "label": label
    }

    # 将任务加入队列
    TASK_QUEUE.put((task_id, str(file_path), "pe", label))

    return ProcessResult(
        success=True,
        message=f"任务已提交，当前队列位置: {TASK_QUEUE.qsize()}",
        task_id=task_id
    )


@app.get("/task/{task_id}", response_model=TaskStatus)
async def get_task_status(task_id: str):
    """
    查询任务状态
    """
    if task_id not in TASKS:
        raise HTTPException(status_code=404, detail="任务不存在")

    task = TASKS[task_id]
    task_status = TaskStatus(
        task_id=task_id,
        status=task["status"],
        message=task.get("message"),
        result_files=task.get("result_files")
    )
    # 若任务状态为失败，则查询一次后删除任务
    if task_status.status == "failed":
        TASKS.pop(task_id)
    return task_status


@app.get("/result/{task_id}", response_model=ProcessResult)
async def get_result(task_id: str):
    """
    获取处理结果
    """
    if task_id not in TASKS:
        raise HTTPException(status_code=404, detail="任务不存在")

    task = TASKS[task_id]

    if task["status"] == "pending" or task["status"] == "processing":
        return ProcessResult(
            success=False,
            message="任务尚未完成",
            task_id=task_id
        )
    elif task["status"] == "completed":
        return ProcessResult(
            success=True,
            message="处理完成",
            task_id=task_id,
            result_files=task["result_files"]
        )
    else:
        return ProcessResult(
            success=False,
            message=task["message"],
            task_id=task_id
        )


@app.get("/download/{task_id}/{filename}")
async def download_file(task_id: str, filename: str):
    """
    下载结果文件
    """
    if task_id not in TASKS:
        raise HTTPException(status_code=404, detail="任务不存在")

    task = TASKS[task_id]

    if task["status"] != "completed":
        raise HTTPException(status_code=400, detail="任务尚未完成或失败")

    if filename not in [task["result_files"]["graph"], task["result_files"]["sparse_matrix"]]:
        raise HTTPException(status_code=404, detail="文件不存在")

    file_path = RESULT_DIR / task_id / filename
    if not file_path.exists():
        raise HTTPException(status_code=404, detail="文件不存在")

    return FileResponse(
        path=file_path,
        filename=filename,
        media_type="application/octet-stream"
    )


# 系统状态端点
@app.get("/system/status")
async def get_system_status():
    """
    获取系统状态，包括活跃任务数、队列长度和磁盘空间
    """
    with PROCESSING_LOCK:
        active_tasks = len(ACTIVE_TASKS)
        queue_length = TASK_QUEUE.qsize()
        max_concurrent = MAX_CONCURRENT_TASKS

    # 获取磁盘空间信息
    disk = shutil.disk_usage('/')
    total_space_gb = disk.total / (1024 ** 3)
    used_space_gb = disk.used / (1024 ** 3)
    free_space_gb = disk.free / (1024 ** 3)
    disk_usage_percent = (disk.used / disk.total) * 100

    return {
        "active_tasks": active_tasks,
        "queue_length": queue_length,
        "max_concurrent_tasks": max_concurrent,
        "status": "normal" if active_tasks < max_concurrent else "busy",
        "disk": {
            "total_gb": round(total_space_gb, 2),
            "used_gb": round(used_space_gb, 2),
            "free_gb": round(free_space_gb, 2),
            "usage_percent": round(disk_usage_percent, 2),
            "min_required_gb": MIN_DISK_SPACE_GB,
            "disk_enough": free_space_gb >= MIN_DISK_SPACE_GB
        }
    }


# 根路径端点
@app.get("/")
async def root():
    return {
        "api": "Malware CFG Embedding API",
        "version": "1.0",
        "endpoints": [
            {"path": "/preprocess_pe", "method": "POST", "description": "上传PE文件并启动预处理任务，支持pe格式，可通过task_id参数指定外部任务ID"},
            {"path": "/task/{task_id}", "method": "GET", "description": "查询任务状态"},
            {"path": "/result/{task_id}", "method": "GET", "description": "获取处理结果"},
            {"path": "/download/{task_id}/{filename}", "method": "GET", "description": "下载结果文件"},
            {"path": "/system/status", "method": "GET", "description": "获取系统状态，包括活跃任务数和队列长度"}
        ]
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=17777)