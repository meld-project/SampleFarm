# 从二进制文件的汇编代码中提取原始控制流图(CFG)
# 提取的CFG将以JSON格式存储
# 基于MCBG代码库构建: https://github.com/Bowen-n/MCBG

import os
import argparse
import logging
from asm import *

# 配置日志
logger = logging.getLogger('extract_cfg')


def extract_cfg(input_file, output_dir, file_format='json'):
    """
    从单个ASM文件提取CFG并保存
    
    参数:
    input_file: ASM文件路径
    output_dir: 输出目录
    file_format: 输出格式 (json 或 pickle)
    """
    # 确保输出目录存在
    os.makedirs(output_dir, exist_ok=True)

    # 获取文件名和ID
    filename = os.path.basename(input_file)
    binary_id = filename.split('.')[0]
    directory = os.path.dirname(input_file)

    # 检查空代码ID列表
    empty_code_ids = []
    empty_code_file = 'empty_code.err'
    if os.path.exists(empty_code_file):
        with open(empty_code_file, 'r') as f:
            empty_code_ids = f.read().split('\n')

    # 检查是否为空代码
    if binary_id in empty_code_ids:
        logger.info(f'文件 {filename}: 空代码，跳过')
        return False

    # 检查是否已处理
    store_path = os.path.join(output_dir, f'{binary_id}.{file_format}')
    if os.path.exists(store_path):
        logger.info(f'文件 {filename}: 已处理，跳过')
        return True

    # 解析ASM文件
    logger.info(f'处理文件: {filename}')
    parser = AsmParser(directory=directory, binary_id=binary_id)
    success = parser.parse()

    if success:
        parser.store_blocks(store_path, fformat=file_format)
        
        # 检查文件大小是否超过50MB
        file_size = os.path.getsize(store_path)
        if file_size > 52428800:  # 50MB = 50 * 1024 * 1024 = 52428800字节
            logger.warning(f'文件 {filename}: 生成的CFG文件大小超过50MB ({file_size/1024/1024:.2f}MB)，删除文件')
            os.remove(store_path)
            return False
        
        logger.info(f'文件 {filename}: 处理成功')
        return True
    else:
        logger.warning(f'文件 {filename}: 解析后为空代码或块')
        return False


if __name__ == '__main__':
    # 命令行参数解析
    parser = argparse.ArgumentParser(description='从ASM文件提取控制流图')
    parser.add_argument('--input_file', type=str, required=True, help='ASM文件路径')
    parser.add_argument('--output_dir', type=str, required=True, help='输出目录')
    parser.add_argument('--format', type=str, default='json', choices=['json', 'pickle'], help='输出格式')

    args = parser.parse_args()

    # 调用提取函数
    extract_cfg(args.input_file, args.output_dir, args.format)

