import os
import subprocess
import logging
logger = logging.getLogger('mkasm')

# Configuration
IDA_PATH = r"./ida91-linux/idat"  # Path to IDA Pro executable
SCRIPT_PATH = r"./parse.py"  # Path to your disassembly script
SAMPLES_DIR = r"./samples"  # Input directory
PROCESSED_DIR = r"./temp/processing"  # Output directory (will be created automatically)

def process_single_file(input_path, output_dir=None):
    """处理单个PE文件并生成ASM文件"""
    # 使用指定的输出目录或默认目录
    output_dir = output_dir or PROCESSED_DIR
    os.makedirs(output_dir, exist_ok=True)
    
    filename = os.path.basename(input_path)
    
    # 跳过IDA生成的数据库文件
    if filename.endswith('.i64') or filename.endswith('.idb'):
        return
    
    logger.info(f"Processing {filename}...")

# 准备IDA命令，传递输出目录参数
    cmd = [
        f'"{IDA_PATH}"',
        "-A", 
        f'-S"{SCRIPT_PATH} {output_dir}"',
        f'"{input_path}"'
    ]
    
    # 运行IDA Pro
    subprocess.run(" ".join(cmd), shell=True, check=True)

    # 删除原目录下的.i64或.idb文件
    filename = os.path.splitext(filename)[0]
    for ext in ['.i64', '.idb', 'id0', 'id1', 'id2', 'nam', 'til']:
        file_path = os.path.join(SAMPLES_DIR, filename + ext)
        if os.path.exists(file_path):
            os.remove(file_path)
            logger.info(f"Deleted {file_path}")

def process_all_files():
    # Create processed directory if it doesn't exist
    os.makedirs(PROCESSED_DIR, exist_ok=True)
    
    # Process each file in samples directory
    for root, dirs, files in os.walk(SAMPLES_DIR):
        for filename in files:
            input_path = os.path.join(root, filename)
            process_single_file(input_path)

if __name__ == "__main__":
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=[
            logging.FileHandler(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'process.log')),
            logging.StreamHandler()
        ]
    )
    process_all_files()