#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
查重机制模块
功能：基于历史库避免重复选题，使用SQLite存储历史记录
"""

import sqlite3
import os
from datetime import datetime, timedelta
from typing import Dict, Optional


# 默认数据库路径
DEFAULT_DB_PATH = "./data/shared_state/history_log.db"


class DuplicateChecker:
    """查重检查器"""

    def __init__(self, db_path: str = None):
        """
        初始化查重检查器

        Args:
            db_path: 数据库文件路径，默认使用DEFAULT_DB_PATH
        """
        self.db_path = db_path or DEFAULT_DB_PATH
        self.init_database()

    def init_database(self):
        """初始化数据库表"""
        # 确保目录存在
        os.makedirs(os.path.dirname(self.db_path) or ".", exist_ok=True)

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS history_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                category TEXT,
                entity TEXT,
                action TEXT,
                keywords TEXT,
                summary TEXT,
                source_domain TEXT,
                real_url TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        ''')
        conn.commit()
        conn.close()

    def is_duplicate(self, entity: str, action: str, keywords: str, days: int = 5) -> bool:
        """
        检查是否为重复选题
        判断标准：相同主体 + 相同动作 + 相同关键词 + 时间窗口内

        Args:
            entity: 主体（如Apple、Meta）
            action: 动作（如发布、降价）
            keywords: 关键词（如Avocado,Mango）
            days: 查重天数窗口，默认5天

        Returns:
            True表示重复，False表示不重复
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cutoff_date = (datetime.now() - timedelta(days=days)).strftime('%Y-%m-%d')

        cursor.execute('''
            SELECT COUNT(*) FROM history_log
            WHERE date >= ?
            AND entity = ?
            AND action = ?
            AND keywords = ?
        ''', (cutoff_date, entity, action, keywords))

        count = cursor.fetchone()[0]
        conn.close()

        return count > 0

    def add_record(self, record_dict: Dict[str, str]) -> int:
        """
        添加新记录到历史库

        Args:
            record_dict: 记录字典，包含以下字段：
                - date: 日期 (YYYY-MM-DD)
                - category: 类别
                - entity: 主体
                - action: 动作
                - keywords: 关键词
                - summary: 一句话摘要
                - source_domain: 来源域名
                - real_url: 真实URL

        Returns:
            插入记录的ID
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute('''
            INSERT INTO history_log
            (date, category, entity, action, keywords, summary, source_domain, real_url)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ''', (
            record_dict['date'],
            record_dict.get('category', ''),
            record_dict['entity'],
            record_dict['action'],
            record_dict['keywords'],
            record_dict.get('summary', ''),
            record_dict.get('source_domain', ''),
            record_dict.get('real_url', '')
        ))

        record_id = cursor.lastrowid
        conn.commit()
        conn.close()

        return record_id

    def get_recent_records(self, days: int = 5, limit: int = 50) -> list:
        """
        获取最近的历史记录

        Args:
            days: 查询最近多少天的记录
            limit: 返回记录的最大数量

        Returns:
            记录列表
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cutoff_date = (datetime.now() - timedelta(days=days)).strftime('%Y-%m-%d')

        cursor.execute('''
            SELECT date, category, entity, action, keywords, summary, source_domain, real_url
            FROM history_log
            WHERE date >= ?
            ORDER BY created_at DESC
            LIMIT ?
        ''', (cutoff_date, limit))

        records = cursor.fetchall()
        conn.close()

        return records

    def cleanup_old_records(self, days_to_keep: int = 5) -> int:
        """
        清理过期记录

        Args:
            days_to_keep: 保留最近多少天的记录

        Returns:
            删除的记录数量
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cutoff_date = (datetime.now() - timedelta(days=days_to_keep)).strftime('%Y-%m-%d')

        cursor.execute('DELETE FROM history_log WHERE date < ?', (cutoff_date,))
        deleted_count = cursor.rowcount

        conn.commit()
        conn.close()

        return deleted_count

    def get_stats(self) -> Dict[str, int]:
        """
        获取历史库统计信息

        Returns:
            包含统计信息的字典
        """
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # 总记录数
        cursor.execute('SELECT COUNT(*) FROM history_log')
        total = cursor.fetchone()[0]

        # 最近5天记录数
        cutoff_date = (datetime.now() - timedelta(days=5)).strftime('%Y-%m-%d')
        cursor.execute('SELECT COUNT(*) FROM history_log WHERE date >= ?', (cutoff_date,))
        recent = cursor.fetchone()[0]

        # 最独特主体数
        cursor.execute('SELECT COUNT(DISTINCT entity) FROM history_log')
        unique_entities = cursor.fetchone()[0]

        conn.close()

        return {
            'total_records': total,
            'recent_5_days': recent,
            'unique_entities': unique_entities
        }


# 便捷函数，简化调用

def is_duplicate(entity: str, action: str, keywords: str,
                db_path: str = None, days: int = 5) -> bool:
    """
    快速检查是否重复

    Args:
        entity: 主体
        action: 动作
        keywords: 关键词
        db_path: 数据库路径，默认使用DEFAULT_DB_PATH
        days: 查重天数窗口

    Returns:
        True表示重复，False表示不重复
    """
    checker = DuplicateChecker(db_path)
    return checker.is_duplicate(entity, action, keywords, days)


def add_record(record_dict: Dict[str, str], db_path: str = None) -> int:
    """
    快速添加记录

    Args:
        record_dict: 记录字典
        db_path: 数据库路径，默认使用DEFAULT_DB_PATH

    Returns:
        插入记录的ID
    """
    checker = DuplicateChecker(db_path)
    return checker.add_record(record_dict)


if __name__ == "__main__":
    # 测试用例
    print("初始化数据库...")
    checker = DuplicateChecker()

    # 测试添加记录
    print("\n添加测试记录...")
    record = {
        "date": "2026-01-25",
        "category": "科技",
        "entity": "Apple",
        "action": "发布",
        "keywords": "AI,芯片",
        "summary": "Apple发布革命性AI芯片",
        "source_domain": "example.com",
        "real_url": "https://example.com/news/1"
    }
    record_id = checker.add_record(record)
    print(f"添加成功，记录ID: {record_id}")

    # 测试查重
    print("\n测试查重...")
    is_dup = checker.is_duplicate("Apple", "发布", "AI,芯片", days=5)
    print(f"是否重复: {is_dup}")  # 应该返回True

    is_dup2 = checker.is_duplicate("Meta", "发布", "AI,芯片", days=5)
    print(f"是否重复: {is_dup2}")  # 应该返回False

    # 获取统计信息
    print("\n统计信息:")
    stats = checker.get_stats()
    print(f"总记录数: {stats['total_records']}")
    print(f"最近5天记录数: {stats['recent_5_days']}")
    print(f"独特主体数: {stats['unique_entities']}")
