import idautils, idc, idaapi
import os
import sys

def save_asm_with_address(output_dir=None):
    # Get the input file path from IDA
    input_path = idaapi.get_input_file_path()
    
    # Generate output path by changing directory and extension
    input_filename = os.path.basename(input_path)
    output_filename = f"{os.path.splitext(input_filename)[0]}.asm"
    
    # Use specified output directory or default
    if output_dir is None:
        output_dir = os.path.join(os.path.dirname(os.path.dirname(input_path)), f"temp/processing/{os.path.splitext(input_filename)[0]}/asm")
    
    # Create output directory if it doesn't exist
    os.makedirs(output_dir, exist_ok=True)
    
    output_path = os.path.join(output_dir, output_filename)
    
    with open(output_path, 'w') as f:
        for seg in idautils.Segments():  # 遍历所有段
            seg_name = idc.get_segm_name(seg)
            seg_start = idc.get_segm_start(seg)
            seg_end = idc.get_segm_end(seg)
            
            if idc.get_segm_attr(seg, SEGATTR_TYPE) == idc.SEG_CODE:  # 仅代码段
                ea = seg_start
                while ea < seg_end:
                    disasm = idc.generate_disasm_line(ea, 0)  # 获取反汇编行
                    if disasm:
                        line = f"{seg_name}:{ea:08X} {idc.get_bytes(ea, idc.get_item_size(ea)).hex(' ')} {disasm}\n"
                        f.write(line)
                    ea = idc.next_head(ea)
    
    print(f"Disassembly saved to: {output_path}")

# 使用示例
idaapi.auto_wait()

# 从命令行参数获取输出目录
output_dir = None
if len(sys.argv) > 1:
    output_dir = sys.argv[1]

save_asm_with_address(output_dir)
idaapi.qexit(0)