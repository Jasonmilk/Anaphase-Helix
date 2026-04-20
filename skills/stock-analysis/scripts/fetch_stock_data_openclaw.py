#!/usr/bin/env python3
"""
基于 openclaw-china-market-gateway 的股票数据获取脚本
集成 openclaw 的多数据源能力，提高数据稳定性
"""

import sys
import json
import argparse
from datetime import datetime
import os

# 尝试导入 openclaw
try:
    from openclaw import ChinaMarketGateway
    OPENCLAW_AVAILABLE = True
except ImportError:
    OPENCLAW_AVAILABLE = False
    print("警告：openclaw 未安装，使用备用数据源")


def fetch_with_openclaw(stock_code, days=30):
    """
    使用 openclaw 获取股票数据
    
    Args:
        stock_code: 股票代码
        days: 获取天数
    
    Returns:
        dict: 包含实时数据和历史数据的字典
    """
    if not OPENCLAW_AVAILABLE:
        return None
    
    try:
        # 创建网关实例
        gateway = ChinaMarketGateway()
        
        # 格式化股票代码
        # openclaw 期望的格式：000001.SZ (深市) 或 600000.SH (沪市)
        formatted_code = format_code_for_openclaw(stock_code)
        
        print(f"使用 openclaw 获取 {formatted_code} 的数据...")
        
        # 获取实时行情
        real_time_data = gateway.get_quote(formatted_code)
        
        # 获取历史K线
        historical_data = gateway.get_kline(formatted_code, period='1d', count=days)
        
        # 统一数据格式
        result = {
            'real_time': normalize_realtime_data(real_time_data),
            'historical': normalize_historical_data(historical_data),
            'source': 'openclaw',
            'data_sources': gateway.get_available_sources()
        }
        
        print(f"✓ openclaw 数据获取成功（使用数据源: {result.get('source', 'unknown')}）")
        return result
        
    except Exception as e:
        print(f"✗ openclaw 获取数据失败: {str(e)}")
        return None


def format_code_for_openclaw(stock_code):
    """
    将股票代码格式化为 openclaw 期望的格式
    
    Args:
        stock_code: 原始股票代码
    
    Returns:
        str: 格式化后的代码
    """
    code = stock_code.strip().upper()
    
    # 如果已经是 .SZ 或 .SH 格式
    if '.' in code:
        return code
    
    # 6位数字，需要添加交易所后缀
    if code.isdigit() and len(code) == 6:
        if code.startswith('6'):
            return f"{code}.SH"  # 沪市
        elif code.startswith(('0', '3', '8', '4')):
            return f"{code}.SZ"  # 深市
    
    # 港股
    if len(code) <= 5 and code.isdigit():
        return f"{code}.HK"
    
    # 美股或其他
    return code


def normalize_realtime_data(raw_data):
    """
    标准化实时数据格式
    
    Args:
        raw_data: openclaw 返回的原始数据
    
    Returns:
        dict: 标准化后的数据
    """
    if not raw_data:
        return None
    
    # 根据 openclaw 的实际返回格式进行转换
    normalized = {
        'source': 'openclaw',
        'name': raw_data.get('name', ''),
        'current': raw_data.get('current', 0),
        'open': raw_data.get('open', 0),
        'high': raw_data.get('high', 0),
        'low': raw_data.get('low', 0),
        'pre_close': raw_data.get('pre_close', 0),
        'volume': raw_data.get('volume', 0),
        'amount': raw_data.get('amount', 0),
        'date': raw_data.get('date', ''),
        'time': raw_data.get('time', '')
    }
    
    return normalized


def normalize_historical_data(raw_data):
    """
    标准化历史数据格式
    
    Args:
        raw_data: openclaw 返回的原始数据
    
    Returns:
        list: 标准化后的K线数据列表
    """
    if not raw_data:
        return []
    
    normalized = []
    
    for item in raw_data:
        normalized.append({
            'date': item.get('date', ''),
            'open': float(item.get('open', 0)),
            'high': float(item.get('high', 0)),
            'low': float(item.get('low', 0)),
            'close': float(item.get('close', 0)),
            'volume': float(item.get('volume', 0))
        })
    
    return normalized


def save_to_file(stock_code, real_time_data, historical_data, output_dir='.', source='openclaw'):
    """保存数据到文件"""
    clean_code = stock_code.replace('/', '_').replace('\\', '_')
    filename = f"stock_data_{clean_code}.json"
    filepath = os.path.join(output_dir, filename)
    
    data = {
        'stock_code': stock_code,
        'fetch_time': datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
        'data_source': source,
        'real_time': real_time_data,
        'historical': historical_data
    }
    
    with open(filepath, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    
    print(f"数据已保存到: {filepath}")
    return filepath


def print_summary(real_time_data, historical_data):
    """打印数据摘要"""
    if real_time_data:
        print(f"\n=== 实时行情 ===")
        print(f"股票名称: {real_time_data.get('name', '')}")
        print(f"当前价格: {real_time_data.get('current', 0):.2f}")
        print(f"开盘价: {real_time_data.get('open', 0):.2f}")
        print(f"最高价: {real_time_data.get('high', 0):.2f}")
        print(f"最低价: {real_time_data.get('low', 0):.2f}")
        print(f"昨收价: {real_time_data.get('pre_close', 0):.2f}")
        
        # 计算涨跌幅
        current = real_time_data.get('current', 0)
        pre_close = real_time_data.get('pre_close', 0)
        if pre_close > 0:
            change = current - pre_close
            change_pct = (change / pre_close) * 100
            print(f"涨跌额: {change:+.2f}")
            print(f"涨跌幅: {change_pct:+.2f}%")
    
    if historical_data:
        print(f"\n=== 历史K线数据 ===")
        print(f"数据条数: {len(historical_data)}")
        if len(historical_data) > 0:
            print(f"日期范围: {historical_data[-1]['date']} 至 {historical_data[0]['date']}")


def main():
    parser = argparse.ArgumentParser(description='基于 openclaw 获取股票数据')
    parser.add_argument('--stock_code', required=True, help='股票代码（如 000001, 600000）')
    parser.add_argument('--days', type=int, default=30, help='获取历史数据天数（默认30天）')
    parser.add_argument('--output', default='.', help='输出目录（默认当前目录）')
    parser.add_argument('--fallback', action='store_true', help='openclaw 失败时降级到备用数据源')
    
    args = parser.parse_args()
    
    print(f"正在获取股票 {args.stock_code} 的数据...")
    
    # 尝试使用 openclaw
    result = fetch_with_openclaw(args.stock_code, args.days)
    
    if result:
        real_time_data = result.get('real_time')
        historical_data = result.get('historical')
        
        print_summary(real_time_data, historical_data)
        
        # 保存数据
        filepath = save_to_file(
            args.stock_code,
            real_time_data,
            historical_data,
            args.output,
            source='openclaw'
        )
        
        # 显示可用数据源
        if 'data_sources' in result:
            print(f"\n=== 可用数据源 ===")
            for source in result['data_sources']:
                print(f"  - {source}")
        
        print(f"\n✓ 数据获取完成，文件路径: {filepath}")
        
    else:
        print("✗ openclaw 数据获取失败")
        
        if args.fallback:
            print("尝试降级到备用数据源...")
            # 导入原有的 fetch_stock_data
            from fetch_stock_data import main as fallback_main
            sys.argv = ['fetch_stock_data.py', '--stock_code', args.stock_code, '--days', str(args.days), '--output', args.output]
            fallback_main()
        else:
            print("提示：添加 --fallback 参数可在 openclaw 失败时使用备用数据源")
            sys.exit(1)


if __name__ == '__main__':
    main()
