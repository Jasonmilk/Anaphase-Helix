#!/usr/bin/env python3
"""
学习进度管理工具

保存和读取英语词汇学习进度

使用示例:
    # 保存进度
    python progress_manager.py --action save --exam cet4 --words learned_words.json --progress current_story.txt
    
    # 读取进度
    python progress_manager.py --action load --exam cet4
    
    # 重置进度
    python progress_manager.py --action reset --exam cet4
"""

import json
import argparse
import sys
from pathlib import Path
from datetime import datetime


PROGRESS_FILE = './vocab_progress.json'


def load_progress_file() -> dict:
    """
    加载进度文件
    
    Returns:
        进度数据字典
    """
    progress_file = Path(PROGRESS_FILE)
    
    if not progress_file.exists():
        return {}
    
    with open(progress_file, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_progress_file(progress: dict):
    """
    保存进度文件
    
    Args:
        progress: 进度数据字典
    """
    with open(PROGRESS_FILE, 'w', encoding='utf-8') as f:
        json.dump(progress, f, ensure_ascii=False, indent=2)


def save_progress(exam_type: str, words: list, story: str = '', quiz_score: dict = None):
    """
    保存学习进度
    
    Args:
        exam_type: 考试类型
        words: 已学习的词汇列表
        story: 当前故事内容（可选）
        quiz_score: 答题分数记录（可选，可以是dict或list）
    """
    # 加载现有进度
    all_progress = load_progress_file()
    
    # 处理quiz_score，可能是dict或list
    correct_count = 0
    total_count = 0
    
    if quiz_score:
        if isinstance(quiz_score, dict):
            correct_count = sum(1 for s in quiz_score.values() if s)
            total_count = len(quiz_score)
        elif isinstance(quiz_score, list):
            # 如果是list，假设每个元素是一个dict，包含'correct'字段
            total_count = len(quiz_score)
            correct_count = sum(1 for item in quiz_score if item.get('correct', False))
    
    # 准备进度数据
    progress_data = {
        'exam_type': exam_type,
        'last_updated': datetime.now().isoformat(),
        'learned_words': words,
        'total_words': len(words),
        'current_story': story,
        'quiz_score': quiz_score if quiz_score else {},
        'quiz_correct_count': correct_count,
        'quiz_total_count': total_count
    }
    
    # 计算正确率
    if progress_data['quiz_total_count'] > 0:
        progress_data['accuracy'] = round(
            progress_data['quiz_correct_count'] / progress_data['quiz_total_count'] * 100, 2
        )
    else:
        progress_data['accuracy'] = 0
    
    # 保存到总进度
    all_progress[exam_type] = progress_data
    
    # 保存文件
    save_progress_file(all_progress)
    
    print(f"✓ 进度已保存 ({exam_type.upper()})")
    print(f"  - 已学习词汇: {progress_data['total_words']} 个")
    if progress_data['quiz_total_count'] > 0:
        print(f"  - 答题正确率: {progress_data['accuracy']}%")
    print(f"  - 更新时间: {progress_data['last_updated']}")


def load_progress(exam_type: str) -> dict:
    """
    读取学习进度
    
    Args:
        exam_type: 考试类型
    
    Returns:
        进度数据字典
    """
    all_progress = load_progress_file()
    
    if exam_type not in all_progress:
        print(f"未找到 {exam_type.upper()} 的学习进度")
        print(f"提示: 请先开始学习并保存进度")
        return None
    
    progress = all_progress[exam_type]
    
    print(f"✓ 加载进度 ({exam_type.upper()})")
    print(f"  - 已学习词汇: {progress['total_words']} 个")
    if progress['quiz_total_count'] > 0:
        print(f"  - 答题正确率: {progress['accuracy']}%")
    print(f"  - 最后更新: {progress['last_updated']}")
    
    return progress


def list_all_progress():
    """
    列出所有考试类型的学习进度
    """
    all_progress = load_progress_file()
    
    if not all_progress:
        print("暂无任何学习进度")
        return
    
    print("学习进度概览:")
    print("-" * 60)
    
    for exam_type, progress in all_progress.items():
        print(f"\n📚 {exam_type.upper()}")
        print(f"   已学习: {progress['total_words']} 个词汇")
        if progress['quiz_total_count'] > 0:
            print(f"   正确率: {progress['accuracy']}%")
        print(f"   更新时间: {progress['last_updated']}")


def reset_progress(exam_type: str):
    """
    重置指定考试类型的学习进度
    
    Args:
        exam_type: 考试类型
    """
    all_progress = load_progress_file()
    
    if exam_type not in all_progress:
        print(f"未找到 {exam_type.upper()} 的学习进度，无需重置")
        return
    
    # 删除该考试类型的进度
    del all_progress[exam_type]
    
    # 保存
    save_progress_file(all_progress)
    
    print(f"✓ 已重置 {exam_type.upper()} 的学习进度")


def main():
    parser = argparse.ArgumentParser(description='学习进度管理工具')
    parser.add_argument('--action', required=True,
                       choices=['save', 'load', 'reset', 'list'],
                       help='操作类型')
    parser.add_argument('--exam',
                       choices=['cet4', 'cet6', 'ielts', 'toefl'],
                       help='考试类型（save/load/reset需要）')
    parser.add_argument('--words',
                       help='已学习词汇的JSON文件路径（save需要）')
    parser.add_argument('--story',
                       help='当前故事内容文件路径（save需要，可选）')
    parser.add_argument('--quiz',
                       help='答题记录JSON文件路径（save需要，可选）')
    
    args = parser.parse_args()
    
    try:
        if args.action == 'list':
            list_all_progress()
        
        elif args.action == 'save':
            if not args.exam:
                raise ValueError("--save 操作需要指定 --exam 参数")
            
            # 加载词汇数据
            if args.words and Path(args.words).exists():
                with open(args.words, 'r', encoding='utf-8') as f:
                    words = json.load(f)
            else:
                words = []
            
            # 加载故事
            story = ''
            if args.story and Path(args.story).exists():
                with open(args.story, 'r', encoding='utf-8') as f:
                    story = f.read()
            
            # 加载答题记录
            quiz_score = None
            if args.quiz and Path(args.quiz).exists():
                with open(args.quiz, 'r', encoding='utf-8') as f:
                    quiz_score = json.load(f)
            
            save_progress(args.exam, words, story, quiz_score)
        
        elif args.action == 'load':
            if not args.exam:
                raise ValueError("--load 操作需要指定 --exam 参数")
            
            progress = load_progress(args.exam)
            
            if progress:
                # 输出到文件供其他脚本使用
                output_file = f'progress_{args.exam}.json'
                with open(output_file, 'w', encoding='utf-8') as f:
                    json.dump(progress, f, ensure_ascii=False, indent=2)
                print(f"\n✓ 进度已导出到: {output_file}")
        
        elif args.action == 'reset':
            if not args.exam:
                raise ValueError("--reset 操作需要指定 --exam 参数")
            
            reset_progress(args.exam)
        
        return 0
        
    except Exception as e:
        print(f"错误: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return 1


if __name__ == '__main__':
    sys.exit(main())
