import os
import sys
import time
import uuid
import requests
import argparse
from pathlib import Path

# API基础URL
BASE_URL = "http://localhost:17777"


def test_system_status():
    """测试系统状态端点"""
    print("\n===== 测试系统状态 =====")
    url = f"{BASE_URL}/system/status"
    try:
        response = requests.get(url)
        if response.status_code == 200:
            status = response.json()
            print(f"系统状态: {status['status']}")
            print(f"活跃任务数: {status['active_tasks']}")
            print(f"队列长度: {status['queue_length']}")
            print(f"最大并发任务数: {status['max_concurrent_tasks']}")
            print("磁盘信息:")
            print(f"  总空间: {status['disk']['total_gb']}GB")
            print(f"  已用空间: {status['disk']['used_gb']}GB")
            print(f"  可用空间: {status['disk']['free_gb']}GB")
            print(f"  使用率: {status['disk']['usage_percent']}%")
            print(f"  磁盘空间是否充足: {'是' if status['disk']['disk_enough'] else '否'}")
            return True
        else:
            print(f"获取系统状态失败，状态码: {response.status_code}")
            return False
    except Exception as e:
        print(f"获取系统状态异常: {str(e)}")
        return False


def upload_file(file_path, task_id=None, label=None):
    """上传文件并返回任务ID"""
    print("\n===== 上传文件 =====")
    url = f"{BASE_URL}/preprocess_pe"
    try:
        with open(file_path, 'rb') as f:
            files = {'file': (os.path.basename(file_path), f)}
            data = {}
            if task_id:
                data['task_id'] = task_id
            if label is not None:
                data['label'] = label
            else:
                data['label'] = 1
            response = requests.post(url, files=files, data=data)
            if response.status_code == 200:
                result = response.json()
                print(f"上传成功，任务ID: {result['task_id']}")
                print(f"消息: {result['message']}")
                return result['task_id']
            else:
                print(f"上传失败，状态码: {response.status_code}")
                print(f"错误信息: {response.text}")
                return None
    except Exception as e:
        print(f"上传文件异常: {str(e)}")
        return None


def check_task_status(task_id):
    """检查任务状态"""
    url = f"{BASE_URL}/task/{task_id}"
    try:
        response = requests.get(url)
        if response.status_code == 200:
            status = response.json()
            print(f"任务状态: {status['status']}")
            print(f"消息: {status['message']}")
            return status
        else:
            print(f"获取任务状态失败，状态码: {response.status_code}")
            return None
    except Exception as e:
        print(f"获取任务状态异常: {str(e)}")
        return None


def get_result(task_id):
    """获取处理结果"""
    print("\n===== 获取处理结果 =====")
    url = f"{BASE_URL}/result/{task_id}"
    try:
        response = requests.get(url)
        if response.status_code == 200:
            result = response.json()
            print(f"处理结果: {'成功' if result['success'] else '失败'}")
            print(f"消息: {result['message']}")
            if result['success'] and result['result_files']:
                print("生成的结果文件:")
                for key, filename in result['result_files'].items():
                    print(f"  {key}: {filename}")
            return result
        else:
            print(f"获取结果失败，状态码: {response.status_code}")
            return None
    except Exception as e:
        print(f"获取结果异常: {str(e)}")
        return None


def download_file(task_id, filename, output_dir):
    """下载结果文件"""
    print(f"\n===== 下载文件: {filename} =====")
    url = f"{BASE_URL}/download/{task_id}/{filename}"
    output_path = os.path.join(output_dir, filename)
    try:
        response = requests.get(url, stream=True)
        if response.status_code == 200:
            with open(output_path, 'wb') as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)
            print(f"文件下载成功: {output_path}")
            return output_path
        else:
            print(f"下载文件失败，状态码: {response.status_code}")
            return None
    except Exception as e:
        print(f"下载文件异常: {str(e)}")
        return None


def run_full_test(file_path, task_id=None, wait_interval=5, max_wait_minutes=30, label=None):
    """运行完整测试流程"""
    # 1. 测试系统状态
    if not test_system_status():
        print("系统状态异常，测试终止")
        return False

    # 2. 上传文件
    task_id = upload_file(file_path, task_id, label)
    if not task_id:
        print("文件上传失败，测试终止")
        return False

    # 3. 等待任务完成
    print("\n===== 等待任务完成 =====")
    max_wait_seconds = max_wait_minutes * 60
    start_time = time.time()
    completed = False

    while time.time() - start_time < max_wait_seconds:
        status = check_task_status(task_id)
        if status and status['status'] in ['completed', 'failed']:
            completed = status['status'] == 'completed'
            break
        print(f"任务仍在处理中，{wait_interval}秒后再次检查...")
        time.sleep(wait_interval)

    if not completed:
        print("任务处理超时或失败，测试终止")
        return False

    # 4. 获取处理结果
    result = get_result(task_id)
    if not result or not result['success']:
        print("获取结果失败，测试终止")
        return False

    # 5. 下载结果文件
    output_dir = os.path.join("test_results", task_id)
    os.makedirs(output_dir, exist_ok=True)

    downloaded_files = []
    for filename in result['result_files'].values():
        file_path = download_file(task_id, filename, output_dir)
        if file_path:
            downloaded_files.append(file_path)

    if len(downloaded_files) != len(result['result_files']):
        print("部分文件下载失败")
        return False

    print("\n===== 测试总结 =====")
    print(f"任务ID: {task_id}")
    print(f"结果文件保存路径: {output_dir}")
    print("测试成功完成！")
    return True


def main():
    parser = argparse.ArgumentParser(description="API测试程序")
    parser.add_argument("file_path", help="要上传的PE文件路径")
    parser.add_argument("--task_id", help="可选的任务ID")
    parser.add_argument("--wait_interval", type=int, default=1, help="检查任务状态的时间间隔(秒)")
    parser.add_argument("--max_wait", type=int, default=30, help="最大等待时间(分钟)")
    parser.add_argument("--label", type=int, required=True, help="标签值")
    args = parser.parse_args()

    # 检查文件是否存在
    if not os.path.exists(args.file_path):
        print(f"错误: 文件 '{args.file_path}' 不存在")
        sys.exit(1)

    # 运行测试
    success = run_full_test(
        args.file_path,
        args.task_id,
        args.wait_interval,
        args.max_wait,
        args.label
    )

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()