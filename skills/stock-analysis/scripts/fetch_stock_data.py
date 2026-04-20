#!/usr/bin/env python3
"""
股票数据获取脚本（多数据源支持）
支持新浪财经、东方财富、雪球等多个数据源，自动切换保证稳定性
"""

import requests
import json
import argparse
import sys
from datetime import datetime, timedelta
import pandas as pd
import os
from abc import ABC, abstractmethod


class DataSourceAdapter(ABC):
    """数据源适配器基类"""
    
    def __init__(self):
        self.name = "BaseAdapter"
        self.timeout = 10
    
    @abstractmethod
    def format_stock_code(self, stock_code):
        """格式化股票代码"""
        pass
    
    @abstractmethod
    def fetch_real_time_quote(self, stock_code):
        """获取实时行情"""
        pass
    
    @abstractmethod
    def fetch_historical_data(self, stock_code, days=30):
        """获取历史K线数据"""
        pass


class SinaAdapter(DataSourceAdapter):
    """新浪财经数据源"""
    
    def __init__(self):
        super().__init__()
        self.name = "新浪财经"
    
    def format_stock_code(self, stock_code):
        stock_code = stock_code.strip().upper()
        
        if stock_code.isdigit() and len(stock_code) == 6:
            if stock_code.startswith('6'):
                return f"sh{stock_code}"
            elif stock_code.startswith(('0', '3')):
                return f"sz{stock_code}"
            elif stock_code.startswith(('8', '4')):
                return f"bj{stock_code}"
        
        if stock_code.startswith(('sh', 'sz', 'bj', 'hk')):
            return stock_code
        
        return stock_code
    
    def fetch_real_time_quote(self, stock_code):
        formatted_code = self.format_stock_code(stock_code)
        url = f"http://hq.sinajs.cn/list={formatted_code}"
        
        try:
            headers = {
                'Referer': 'http://finance.sina.com.cn',
                'User-Agent': 'Mozilla/5.0'
            }
            response = requests.get(url, headers=headers, timeout=self.timeout)
            response.encoding = 'gbk'
            
            if response.status_code == 200:
                data = response.text
                if 'var hq_str_' in data:
                    content = data.split('"')[1]
                    values = content.split(',')
                    
                    if len(values) >= 32:
                        return {
                            'source': self.name,
                            'name': values[0],
                            'open': float(values[1]) if values[1] else None,
                            'pre_close': float(values[2]) if values[2] else None,
                            'current': float(values[3]) if values[3] else None,
                            'high': float(values[4]) if values[4] else None,
                            'low': float(values[5]) if values[5] else None,
                            'bid': float(values[6]) if values[6] else None,
                            'ask': float(values[7]) if values[7] else None,
                            'volume': float(values[8]) if values[8] else None,
                            'amount': float(values[9]) if values[9] else None,
                            'date': values[30] if len(values) > 30 else '',
                            'time': values[31] if len(values) > 31 else ''
                        }
            return None
        except Exception as e:
            print(f"{self.name} 获取实时数据失败: {str(e)}")
            return None
    
    def fetch_historical_data(self, stock_code, days=30):
        formatted_code = self.format_stock_code(stock_code)
        
        url = f"http://money.finance.sina.com.cn/quotes_service/api/json_v2.php/CN_MarketData.getKLineData?symbol={formatted_code}&scale=240&ma=no&datalen={days}"
        
        try:
            headers = {
                'Referer': 'http://finance.sina.com.cn',
                'User-Agent': 'Mozilla/5.0'
            }
            response = requests.get(url, headers=headers, timeout=self.timeout)
            response.encoding = 'gbk'
            
            if response.status_code == 200:
                data = response.text
                try:
                    if data.startswith('var'):
                        data = data.split('=', 1)[1].strip()
                    
                    historical_data = json.loads(data)
                    
                    formatted_data = []
                    for item in historical_data:
                        formatted_data.append({
                            'date': item.get('day', ''),
                            'open': float(item.get('open', 0)),
                            'high': float(item.get('high', 0)),
                            'low': float(item.get('low', 0)),
                            'close': float(item.get('close', 0)),
                            'volume': float(item.get('volume', 0))
                        })
                    
                    return formatted_data
                except json.JSONDecodeError:
                    return []
        except Exception as e:
            print(f"{self.name} 获取历史数据失败: {str(e)}")
        
        return []


class EastmoneyAdapter(DataSourceAdapter):
    """东方财富数据源"""
    
    def __init__(self):
        super().__init__()
        self.name = "东方财富"
    
    def format_stock_code(self, stock_code):
        stock_code = stock_code.strip().upper()
        
        if stock_code.isdigit() and len(stock_code) == 6:
            # 沪市: 1.000001, 深市: 0.000001
            if stock_code.startswith('6'):
                return f"1.{stock_code}"
            elif stock_code.startswith(('0', '3')):
                return f"0.{stock_code}"
            elif stock_code.startswith(('8', '4')):
                return f"0.{stock_code}"
        
        # 已格式化的直接返回
        if '.' in stock_code:
            return stock_code
        
        # 默认深市
        return f"0.{stock_code}"
    
    def fetch_real_time_quote(self, stock_code):
        secid = self.format_stock_code(stock_code)
        url = f"http://push2.eastmoney.com/api/qt/stock/get?secid={secid}"
        
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0',
                'Referer': 'http://quote.eastmoney.com/'
            }
            response = requests.get(url, headers=headers, timeout=self.timeout)
            
            if response.status_code == 200:
                data = response.json()
                
                if data.get('rc') == 0 and data.get('data'):
                    info = data['data']
                    return {
                        'source': self.name,
                        'name': info.get('f58', ''),
                        'open': info.get('f43') / 100 if info.get('f43') else None,
                        'pre_close': info.get('f60') / 100 if info.get('f60') else None,
                        'current': info.get('f43') / 100 if info.get('f43') else None,
                        'high': info.get('f44') / 100 if info.get('f44') else None,
                        'low': info.get('f45') / 100 if info.get('f45') else None,
                        'volume': info.get('f47') if info.get('f47') else None,
                        'amount': info.get('f48') / 10000 if info.get('f48') else None,
                        'date': info.get('f5', ''),
                        'time': info.get('f51', '')
                    }
            return None
        except Exception as e:
            print(f"{self.name} 获取实时数据失败: {str(e)}")
            return None
    
    def fetch_historical_data(self, stock_code, days=30):
        secid = self.format_stock_code(stock_code)
        url = f"http://push2.eastmoney.com/api/qt/stock/kline?secid={secid}&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61&klt=101&fqt=1&beg=0&end=20500101&lmt={days}"
        
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0',
                'Referer': 'http://quote.eastmoney.com/'
            }
            response = requests.get(url, headers=headers, timeout=self.timeout)
            
            if response.status_code == 200:
                data = response.json()
                
                if data.get('rc') == 0 and data.get('data'):
                    klines = data['data'].get('klines', [])
                    
                    formatted_data = []
                    for kline in klines:
                        values = kline.split(',')
                        if len(values) >= 6:
                            formatted_data.append({
                                'date': values[0],
                                'open': float(values[1]),
                                'close': float(values[2]),
                                'high': float(values[3]),
                                'low': float(values[4]),
                                'volume': float(values[5])
                            })
                    
                    return formatted_data
        except Exception as e:
            print(f"{self.name} 获取历史数据失败: {str(e)}")
        
        return []


class XueqiuAdapter(DataSourceAdapter):
    """雪球数据源"""
    
    def __init__(self):
        super().__init__()
        self.name = "雪球"
        self.timeout = 15  # 雪球可能需要更长超时
    
    def format_stock_code(self, stock_code):
        stock_code = stock_code.strip().upper()
        
        if stock_code.isdigit() and len(stock_code) == 6:
            if stock_code.startswith('6'):
                return f"SH{stock_code}"
            elif stock_code.startswith(('0', '3', '8', '4')):
                return f"SZ{stock_code}"
        
        if stock_code.startswith(('SH', 'SZ')):
            return stock_code
        
        # 默认深市
        return f"SZ{stock_code}"
    
    def fetch_real_time_quote(self, stock_code):
        symbol = self.format_stock_code(stock_code)
        url = f"https://stock.xueqiu.com/v5/stock/quote.json?symbol={symbol}"
        
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0',
                'Cookie': 'xq_a_token=',
                'Referer': 'https://xueqiu.com/'
            }
            response = requests.get(url, headers=headers, timeout=self.timeout)
            
            if response.status_code == 200:
                data = response.json()
                
                if data.get('error_code') == 0 and data.get('data'):
                    quote = data['data']['quote']
                    return {
                        'source': self.name,
                        'name': quote.get('name', ''),
                        'open': quote.get('open'),
                        'pre_close': quote.get('prev_close'),
                        'current': quote.get('current'),
                        'high': quote.get('high'),
                        'low': quote.get('low'),
                        'volume': quote.get('volume'),
                        'amount': quote.get('amount'),
                        'date': quote.get('time'),
                        'time': quote.get('time')
                    }
            return None
        except Exception as e:
            print(f"{self.name} 获取实时数据失败: {str(e)}")
            return None
    
    def fetch_historical_data(self, stock_code, days=30):
        symbol = self.format_stock_code(stock_code)
        url = f"https://stock.xueqiu.com/v5/stock/chart/kline.json?symbol={symbol}&period=day&type=before&count={days}&indicator=kline,pe,pb,ps,pcf,market_capital"
        
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0',
                'Cookie': 'xq_a_token=',
                'Referer': 'https://xueqiu.com/'
            }
            response = requests.get(url, headers=headers, timeout=self.timeout)
            
            if response.status_code == 200:
                data = response.json()
                
                if data.get('error_code') == 0 and data.get('data'):
                    items = data['data'].get('item', [])
                    
                    formatted_data = []
                    for item in items:
                        if len(item) >= 6:
                            formatted_data.append({
                                'date': datetime.fromtimestamp(item[0] / 1000).strftime('%Y-%m-%d'),
                                'open': item[1],
                                'close': item[2],
                                'high': item[3],
                                'low': item[4],
                                'volume': item[5]
                            })
                    
                    return formatted_data
        except Exception as e:
            print(f"{self.name} 获取历史数据失败: {str(e)}")
        
        return []


class DataSourceManager:
    """数据源管理器，支持自动切换"""
    
    def __init__(self):
        # 数据源优先级列表
        self.adapters = [
            SinaAdapter(),      # 主数据源：新浪财经
            EastmoneyAdapter(), # 备用1：东方财富
            XueqiuAdapter()     # 备用2：雪球
        ]
    
    def fetch_with_fallback(self, stock_code, fetch_method, *args, **kwargs):
        """
        使用数据源获取数据，支持自动切换
        
        Args:
            stock_code: 股票代码
            fetch_method: 获取方法名称 ('fetch_real_time_quote' 或 'fetch_historical_data')
            *args: 方法参数
            **kwargs: 方法参数
        
        Returns:
            dict/list: 获取的数据或None
        """
        for adapter in self.adapters:
            print(f"尝试使用 {adapter.name} 获取数据...")
            
            try:
                method = getattr(adapter, fetch_method)
                result = method(stock_code, *args, **kwargs)
                
                if result:
                    print(f"✓ {adapter.name} 数据获取成功")
                    return result
                else:
                    print(f"✗ {adapter.name} 数据获取失败，尝试下一个数据源...")
            except Exception as e:
                print(f"✗ {adapter.name} 发生异常: {str(e)}，尝试下一个数据源...")
                continue
        
        print("所有数据源均获取失败")
        return None


def calculate_change(real_time_data):
    """计算涨跌幅"""
    if not real_time_data:
        return None
    
    current = real_time_data.get('current')
    pre_close = real_time_data.get('pre_close')
    
    if current and pre_close and pre_close > 0:
        change = current - pre_close
        change_percent = (change / pre_close) * 100
        
        return {
            'current': current,
            'pre_close': pre_close,
            'change': change,
            'change_percent': change_percent
        }
    
    return None


def save_to_file(stock_code, real_time_data, historical_data, output_dir='.'):
    """保存数据到文件"""
    clean_code = stock_code.replace('/', '_').replace('\\', '_')
    filename = f"stock_data_{clean_code}.json"
    filepath = os.path.join(output_dir, filename)
    
    data = {
        'stock_code': stock_code,
        'fetch_time': datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
        'data_source': real_time_data.get('source', 'Unknown') if real_time_data else 'Unknown',
        'real_time': real_time_data,
        'historical': historical_data
    }
    
    with open(filepath, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    
    print(f"数据已保存到: {filepath}")
    return filepath


def main():
    parser = argparse.ArgumentParser(description='获取股票数据（多数据源支持）')
    parser.add_argument('--stock_code', required=True, help='股票代码（如 000001, sh600000, AAPL）')
    parser.add_argument('--days', type=int, default=30, help='获取历史数据天数（默认30天）')
    parser.add_argument('--output', default='.', help='输出目录（默认当前目录）')
    parser.add_argument('--source', help='指定数据源（sina/eastmoney/xueqiu），不指定则自动切换')
    
    args = parser.parse_args()
    
    print(f"正在获取股票 {args.stock_code} 的数据...")
    
    # 创建数据源管理器
    manager = DataSourceManager()
    
    # 如果指定了数据源
    if args.source:
        source_map = {
            'sina': SinaAdapter(),
            'eastmoney': EastmoneyAdapter(),
            'xueqiu': XueqiuAdapter()
        }
        
        if args.source.lower() in source_map:
            adapter = source_map[args.source.lower()]
            print(f"使用指定数据源: {adapter.name}")
            manager.adapters = [adapter]
        else:
            print(f"未知数据源: {args.source}，使用自动切换模式")
    
    # 获取实时行情
    real_time_data = manager.fetch_with_fallback(args.stock_code, 'fetch_real_time_quote')
    
    if real_time_data:
        print(f"\n=== 实时行情（来源: {real_time_data.get('source', 'Unknown')}） ===")
        print(f"股票名称: {real_time_data['name']}")
        print(f"当前价格: {real_time_data['current']:.2f}")
        print(f"开盘价: {real_time_data['open']:.2f}")
        print(f"最高价: {real_time_data['high']:.2f}")
        print(f"最低价: {real_time_data['low']:.2f}")
        print(f"昨收价: {real_time_data['pre_close']:.2f}")
        
        # 计算涨跌幅
        change_info = calculate_change(real_time_data)
        if change_info:
            print(f"涨跌额: {change_info['change']:+.2f}")
            print(f"涨跌幅: {change_info['change_percent']:+.2f}%")
    
    # 获取历史数据
    historical_data = manager.fetch_with_fallback(args.stock_code, 'fetch_historical_data', args.days)
    
    if historical_data:
        print(f"\n=== 历史K线数据（最近{len(historical_data)}个交易日）===")
        print(f"日期范围: {historical_data[-1]['date']} 至 {historical_data[0]['date']}")
    
    # 保存数据
    filepath = save_to_file(args.stock_code, real_time_data, historical_data, args.output)
    
    if not real_time_data and not historical_data:
        print("\n错误：未能获取任何数据，请检查股票代码是否正确或网络连接")
        sys.exit(1)
    
    print(f"\n✓ 数据获取完成，文件路径: {filepath}")


if __name__ == '__main__':
    main()
