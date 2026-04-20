#!/usr/bin/env python3
"""
英语词汇筛选工具

根据考试类型和词汇数量，从词汇库中筛选单词

使用示例:
    python vocabulary_filter.py --exam cet4 --count 20 --output words.json
    python vocabulary_filter.py --exam ielts --count 30
"""

import json
import random
import argparse
import sys
from pathlib import Path


def load_vocabulary(exam_type: str) -> list:
    """
    从references目录加载词汇数据
    
    Args:
        exam_type: 考试类型 (cet4, cet6, ielts, toefl)
    
    Returns:
        词汇列表
    """
    # 映射考试类型到文件名
    file_map = {
        'cet4': 'cet4_vocabulary.md',
        'cet6': 'cet6_vocabulary.md',
        'ielts': 'ielts_vocabulary.md',
        'toefl': 'toefl_vocabulary.md'
    }
    
    if exam_type not in file_map:
        raise ValueError(f"不支持的考试类型: {exam_type}，支持: {list(file_map.keys())}")
    
    # 词汇文件路径
    vocab_file = Path(__file__).parent.parent / 'references' / file_map[exam_type]
    
    if not vocab_file.exists():
        raise FileNotFoundError(f"词汇文件不存在: {vocab_file}")
    
    # 读取并解析词汇数据
    vocabulary = []
    with open(vocab_file, 'r', encoding='utf-8') as f:
        content = f.read()
        
    # 解析Markdown格式的词汇列表
    lines = content.split('\n')
    current_entry = {}
    
    for line in lines:
        line = line.strip()
        
        # 跳过空行和标题
        if not line or line.startswith('#') or line.startswith('---'):
            continue
        
        # 识别单词行 (格式: 单词 [词性])
        if line and not line.startswith('-') and not line.startswith('*'):
            if current_entry:  # 保存上一个词条
                vocabulary.append(current_entry)
            
            # 解析单词和词性
            parts = line.split()
            word = parts[0]
            pos = parts[1].strip('[]') if len(parts) > 1 else 'n.'
            current_entry = {'word': word, 'pos': pos}
        
        # 识别释义行 (格式: - 释义)
        elif line.startswith('-') or line.startswith('*'):
            if current_entry:
                definition = line.lstrip('-*').strip()
                if not current_entry.get('definition'):
                    current_entry['definition'] = definition
                elif not current_entry.get('example'):
                    # 假设第二行是例句
                    if 'example' not in definition.lower():
                        current_entry['example'] = definition
    
    # 添加最后一个词条
    if current_entry:
        vocabulary.append(current_entry)
    
    if not vocabulary:
        raise ValueError(f"未能从文件中解析出词汇数据: {vocab_file}")
    
    return vocabulary


def filter_words(vocabulary: list, count: int) -> list:
    """
    从词汇列表中随机筛选指定数量的单词
    
    Args:
        vocabulary: 词汇列表
        count: 需要筛选的数量
    
    Returns:
        筛选后的词汇列表
    """
    if count <= 0:
        raise ValueError("词汇数量必须大于0")
    
    if count > len(vocabulary):
        print(f"警告: 请求的词汇数量({count})超过库中总量({len(vocabulary)})，将返回全部词汇")
        count = len(vocabulary)
    
    # 随机筛选
    selected = random.sample(vocabulary, count)
    return selected


def main():
    parser = argparse.ArgumentParser(description='英语词汇筛选工具')
    parser.add_argument('--exam', required=True, 
                       choices=['cet4', 'cet6', 'ielts', 'toefl'],
                       help='考试类型')
    parser.add_argument('--count', type=int, default=20,
                       help='筛选的词汇数量 (默认: 20)')
    parser.add_argument('--output', default='words.json',
                       help='输出文件路径 (默认: words.json)')
    
    args = parser.parse_args()
    
    try:
        # 加载词汇库
        print(f"正在加载{args.exam.upper()}词汇库...")
        vocabulary = load_vocabulary(args.exam)
        print(f"词汇库包含 {len(vocabulary)} 个单词")
        
        # 筛选词汇
        print(f"正在筛选 {args.count} 个单词...")
        selected = filter_words(vocabulary, args.count)
        
        # 添加ID（用于进度追踪）
        for i, word in enumerate(selected, 1):
            word['id'] = i
        
        # 输出到文件
        output_path = Path(args.output)
        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(selected, f, ensure_ascii=False, indent=2)
        
        print(f"✓ 成功筛选 {len(selected)} 个单词")
        print(f"✓ 已保存到: {output_path.absolute()}")
        
        # 打印前5个单词作为预览
        print("\n预览:")
        for word in selected[:5]:
            print(f"  {word['word']} [{word['pos']}] - {word.get('definition', 'N/A')}")
        
        return 0
        
    except Exception as e:
        print(f"错误: {e}", file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
