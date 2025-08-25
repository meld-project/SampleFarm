# The following code is from the palmtree project https://github.com/palmtreemodel/PalmTree. 


from torch.autograd import Variable
import torch
import re
import numpy

from torch import nn
import torch.nn.functional as F

from config import *
import vocab

from torch.nn import DataParallel

# 检查 CUDA 是否可用
USE_CUDA = torch.cuda.is_available()
# 选择 GPU 设备（默认 0）
CUDA_DEVICE = 0  # 如果有多个 GPU，可以调整


def parse_instruction(ins, symbol_map, string_map):
    ins = re.sub('\s+', ', ', ins, 1)
    parts = ins.split(', ')
    operand = []
    if len(parts) > 1:
        operand = parts[1:]
    for i in range(len(operand)):
        symbols = re.split('([0-9A-Za-z]+)', operand[i])
        for j in range(len(symbols)):
            if symbols[j][:2] == '0x' and len(symbols[j]) >= 6:
                if int(symbols[j], 16) in symbol_map:
                    symbols[j] = "symbol"
                elif int(symbols[j], 16) in string_map:
                    symbols[j] = "string"
                else:
                    symbols[j] = "address"
        operand[i] = ' '.join(symbols)
    opcode = parts[0]
    return ' '.join([opcode]+operand)



class UsableTransformer:
    def __init__(self, model_path, vocab_path):
        print("Loading Vocab", vocab_path)
        self.vocab = vocab.WordVocab.load_vocab(vocab_path)
        print("Vocab Size: ", len(self.vocab))
        self.model = torch.load(model_path, weights_only=False)
        self.model.eval()
        if USE_CUDA:
            self.model.cuda(CUDA_DEVICE)


    def encode(self, text, output_option='lst'):

        segment_label = []
        sequence = []
        for t in text:
            l = (len(t.split(' '))+2) * [1]
            s = self.vocab.to_seq(t)
            # print(t, s)
            s = [3] + s + [2]
            if len(l) > 20:
                segment_label.append(l[:20])
            else:
                segment_label.append(l + [0]*(20-len(l)))
            if len(s) > 20:
                 sequence.append(s[:20])
            else:
                sequence.append(s + [0]*(20-len(s)))
         
        segment_label = torch.LongTensor(segment_label)
        sequence = torch.LongTensor(sequence)

        if USE_CUDA:
            sequence = sequence.cuda(CUDA_DEVICE)
            segment_label = segment_label.cuda(CUDA_DEVICE)

        encoded = self.model.forward(sequence, segment_label)
        result = torch.mean(encoded.detach(), dim=1)

        del encoded
        if USE_CUDA:
            if numpy:
                return result.data.cpu().numpy()
            else:
                return result.to('cpu')
        else:
            if numpy:
                return result.data.numpy()
            else:
                return result
            

# class UsableTransformer:
#     def __init__(self, model_path, vocab_path):
#         print("Loading Vocab", vocab_path)
#         self.vocab = vocab.WordVocab.load_vocab(vocab_path)
#         print("Vocab Size: ", len(self.vocab))
#         self.model = torch.load(model_path, weights_only=False)
#         self.model.eval()
#         if torch.cuda.is_available():
#             self.model = self.model.cuda()
#             if torch.cuda.device_count() > 1:
#                 self.model = DataParallel(self.model)


#     def encode(self, text, output_option='lst'):

#         segment_label = []
#         sequence = []
#         for t in text:
#             l = (len(t.split(' '))+2) * [1]
#             s = self.vocab.to_seq(t)
#             # print(t, s)
#             s = [3] + s + [2]
#             if len(l) > 20:
#                 segment_label.append(l[:20])
#             else:
#                 segment_label.append(l + [0]*(20-len(l)))
#             if len(s) > 20:
#                  sequence.append(s[:20])
#             else:
#                 sequence.append(s + [0]*(20-len(s)))
         
#         segment_label = torch.LongTensor(segment_label)
#         sequence = torch.LongTensor(sequence)

#         encoded = self.model.forward(sequence, segment_label)
#         result = torch.mean(encoded.detach(), dim=1).cpu()

#         # 根据输出选项返回结果
#         if output_option == 'numpy':
#             return result.numpy()
#         return result
