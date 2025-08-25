import os
import json
import csv
import re
import sys
import argparse
import logging
import numpy as np
import scipy.sparse as sp
import pandas as pd 
from torch.utils.data import DataLoader
import networkx as nx
from scipy.sparse import csr_matrix
from bert_data import REGS, TYPES
from constant import *

logger = logging.getLogger('generate_embeddings')

sys.path.append('Palmtree/pre-trained_model')
import eval_utils as utils


class CFG_Normalized(object):
    def __init__(self, encoder=None):
        palmtree = utils.UsableTransformer(model_path="Palmtree/pre-trained_model/palmtree/transformer.ep19", vocab_path="Palmtree/pre-trained_model/palmtree/vocab")
        self.encoder = palmtree
        
    
    def parse_instruction(self, ins):
        ''' every token is seperated by space '''
        ins = re.sub('\s+', ',', ins, 1)
        parts = ins.split(',')
        operand = []
        if len(parts) > 1:
            operand = parts[1:]
        for i in range(len(operand)):
            symbols = re.split('([0-9A-Za-z]+)', operand[i])  
            symbols = ' '.join(symbols).split()
            operand[i] = " ".join(symbols)
        opcode = parts[0]
        
        return " ".join([opcode]+operand)
    
    def normalize(self, opcode_operands):
        ''' original normalization function, seperate tokens by _ '''
        if opcode_operands[0] == 'call':
            return 'call'

        ret_ins_str = opcode_operands[0]

        for operand in opcode_operands[1:]:

            if operand in REGS:
                ret_ins_str += '_{}'.format(operand)
            elif operand.startswith('[') and operand.endswith(']'):
                ret_ins_str += '_{}'.format(self._handle_ptr(operand))
            elif operand.startswith('ds:') or '_' in operand:
                ret_ins_str += '_MEM'
            elif operand.isnumeric() or operand.endswith('h'):
                ret_ins_str += '_IMM'
            elif operand in TYPES:
                continue
            else:
                ret_ins_str += '_{}'.format(operand)
        
        return ret_ins_str

    def normalize_comma(self, opcode_operands):
        ''' instruction normalization required by Palm Tree '''
        
        if opcode_operands[0] == 'call':
            return 'call'

        ret_ins_str = opcode_operands[0]

        for operand in opcode_operands[1:]:

            if operand in REGS:
                ret_ins_str += ',{}'.format(operand)
            elif operand.startswith('[') and operand.endswith(']'):
                
                ret_ins_str += ',{}'.format(self._handle_ptr(operand))
            elif operand.isnumeric() or operand.endswith('h'):
                ret_ins_str += ',addr'
            elif operand in TYPES:
                continue
            else:
                ret_ins_str += ',string'
        
        return ret_ins_str

    def _handle_ptr(self, ptr):
        ''' 
        [ebp-1Ch] [ebp+8+4] [esp+40h+-18h] [ebp+esp*4] [ebp+8]
        '''

        def _judge_num(string):
            try:
                if string.endswith('h'):
                    tmp = int(string[:-1], 16)
                    return True
                else:
                    return False
            except:
                return False

        ptr = ptr.replace('+-', '-')

        ret_ptr = '['
        item = ''
        count = 0
        operator = ''

        for char in ptr[1:]:
            if char in ['+', '-', ']']:
                if not item.isnumeric() and not _judge_num(item):
                    ret_ptr += operator + item
                else:
                    if item.isnumeric():
                        value = int(item)
                    else:
                        value = int('0x'+item[:-1], 16)

                    if operator == '+':
                        count += value
                    elif operator == '-':
                        count -= value
                operator = char if char != ']' else ''
                item = ''
            else:
                item += char
        
        if count <= -10:
            ret_ptr += '-' + (hex(count)[3:]).upper() + 'h]'
        elif -10 < count < 0:
            ret_ptr += '-' + (hex(count)[3:]).upper() + ']'
        elif count == 0:
            ret_ptr += ']'
        elif 0 < count < 10:
            ret_ptr += '+' + (hex(count)[2:]).upper() + ']'
        elif count >= 10:
            ret_ptr += '+' + (hex(count)[2:]).upper() + 'h]'
        
        return ret_ptr

    def process_file(self, json_file_path, output_dir, label_val):
        ''' 处理单个JSON CFG文件并保存为NPZ格式 '''
        # 确保label_val在[0, 1]范围内
        label_val = int(label_val)
        if label_val not in [0, 1]:
            raise ValueError("label_val必须为0或1")

        # 确保输出目录存在
        os.makedirs(output_dir, exist_ok=True)

        # 文件名处理
        filename = os.path.basename(json_file_path)
        file_id = filename.split('.')[0]

        # 检查输出文件是否已存在
        save_real_path = os.path.join(output_dir, 'graph_{}.npz'.format(file_id))
        save_pat_sparse_matrix_real_path = os.path.join(output_dir, 'graph_{}_sparse_matrix.npz'.format(file_id))
        if os.path.exists(save_real_path) and os.path.exists(save_pat_sparse_matrix_real_path):
            logger.info("文件 {} 已处理，跳过".format(filename))
            return True

        logger.info("处理文件 {}...".format(filename))

        # 加载JSON文件
        try:
            with open(json_file_path, 'r') as f:
                cfg = json.load(f)
        except ValueError as e:
            logger.error("无效的JSON文件: {}".format(e))
            return False

        # 仅做黑白二分类
        num_classes = 2

        # 初始化标签为one-hot编码
        y = np.zeros((num_classes,))
        y[label_val] = 1

        # 获取节点属性矩阵
        addr_to_id = dict()  # {str: int}
        current_node_id = -1
        x = list()  # 节点属性

        for addr, block in cfg.items():
            current_node_id += 1
            addr_to_id[addr] = current_node_id

            # 获取标记化的操作码序列作为节点属性
            tokenized_seq = []
            embeddings = []
            for insn in block['insn_list']:
                opcode = insn['opcode']
                operands = insn['operands']

                opcode_operands = [opcode] + operands
                # 归一化指令并使用逗号分隔操作码和操作数
                normalized = self.normalize_comma(opcode_operands)
                # 解析指令以用作PalmTree模型的输入
                tokenized = self.parse_instruction(normalized)
                
                tokenized_seq.append(tokenized)

            # 为基本块中的每个指令序列生成嵌入
            sequence_loader = DataLoader(tokenized_seq, batch_size=1000, shuffle=False)
            
            for i, batch in enumerate(sequence_loader):
                batch_embeddings = self.encoder.encode(batch)
                if i < 1:
                    embeddings.append(batch_embeddings)
                    embeddings = np.array(embeddings)
                    embeddings = np.reshape(embeddings, (embeddings.shape[1], embeddings.shape[2]))
                else:
                    embeddings = np.append(embeddings, batch_embeddings, axis=0)
            
            block_embeddings = np.mean(embeddings, 0, keepdims=True)
            del embeddings

            x.append(block_embeddings)
            del block_embeddings

        x_np = np.array(x)
        del x
        x_np = np.reshape(x_np, (x_np.shape[0], x_np.shape[2]))

        # 获取稀疏邻接矩阵
        edge_list = list()
        for addr, block in cfg.items():
            start_nid = addr_to_id[addr]
            for out_edge in block['out_edge_list']:
                end_nid = addr_to_id[str(out_edge)]
                edge_list.append([start_nid, end_nid])

        logger.info("已处理文件 {}".format(filename))
        # logger.info("节点属性矩阵形状: {}".format(x_np.shape))

        node_list = [i for i in range(len(cfg.items()))]

        # 从边列表构建有向图
        G = nx.from_edgelist(edge_list, create_using=nx.DiGraph())

        # 构建稀疏邻接数组
        a = nx.to_scipy_sparse_array(G, nodelist=node_list)

        # 转换为压缩稀疏行矩阵
        a = csr_matrix(a)

        # 验证NPZ文件是否合格
        # 1. 检查稀疏矩阵大小
        if a.shape[0] > 46000:
            logger.warning(f"文件 {filename}: 稀疏矩阵过大 ({a.shape[0]}x{a.shape[1]}), 处理失败")
            return False

        # 2. 检查基本块数量
        if x_np.shape[0] < 10:
            logger.warning(f"文件 {filename}: 基本块数量不足 ({x_np.shape[0]}), 处理失败")
            return False

        # 3. 检查上三角矩阵中非自环边的数量
        # 移除对角元素
        adj = a - sp.dia_matrix((a.diagonal()[np.newaxis, :], [0]), shape=a.shape)
        adj.eliminate_zeros()
        adj_triu = sp.triu(adj)
        edges = np.array(adj_triu.nonzero()).T
        if edges.shape[0] < 3:
            logger.warning(f"文件 {filename}: 边数量不足 ({edges.shape[0]}), 处理失败")
            return False

        # 保存为NPZ文件
        np.savez(save_real_path, x=x_np, y=y)
        sp.save_npz(save_pat_sparse_matrix_real_path, a)

        del x_np, a, y, G, edge_list, node_list, adj, adj_triu, edges
        return True


def generate_embeddings(input_file, output_dir, label_val):
    """
    从JSON格式的CFG文件生成嵌入并保存为NPZ格式
    
    参数:
    input_file: JSON文件路径
    output_dir: 输出目录
    label_val: 标签值（0为白1为黑）
    
    返回:
    bool: 处理是否成功
    """
    cfg_normalizer = CFG_Normalized()
    return cfg_normalizer.process_file(input_file, output_dir, label_val)


def main():
    # 命令行参数解析
    parser = argparse.ArgumentParser(description='将JSON格式CFG转换为NPZ嵌入')
    parser.add_argument('--input_file', type=str, required=True, help='JSON文件路径')
    parser.add_argument('--output_dir', type=str, required=True, help='输出目录')
    parser.add_argument('--label_path', type=str, required=True, help='标签文件路径')

    args = parser.parse_args()

    # 创建CFG处理器
    cfg_processor = CFG_Normalized()

    # 处理文件
    # 确保提供了label_path参数
    if not args.label_path:
        logger.error("必须提供标签文件路径")
        sys.exit(1)
    
    success = cfg_processor.process_file(args.input_file, args.output_dir, args.label_path)

    if success:
        logger.info("处理完成")
        sys.exit(0)
    else:
        logger.error("处理失败")
        sys.exit(1)


if __name__ == '__main__':
    main()

