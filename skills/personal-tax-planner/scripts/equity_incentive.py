#!/usr/bin/env python3
"""
股权激励税务计算工具
支持股票期权、限制性股票、股票增值权等
适配2025年最新政策，支持单独计税与合并计税对比
"""
import argparse
import json
from typing import Dict

from common_tax import TAX_BRACKETS, calculate_tax_by_brackets

def calculate_equity_tax_comparison(
    total_taxable_income: float,
    other_comprehensive_income: float
) -> Dict:
    """
    计算股权激励两种计税方式的对比
    依据：财政部 税务总局公告2023年第2号，股权激励可选择单独计税至2027年底
    """
    # 限制应纳税所得额不能为负
    total_taxable_income = max(0.0, total_taxable_income)
    
    # 方案1：单独计税（政策默认选项）
    equity_tax_separate = calculate_tax_by_brackets(total_taxable_income)
    other_tax_separate = calculate_tax_by_brackets(other_comprehensive_income)
    total_tax_separate = equity_tax_separate + other_tax_separate
    
    # 方案2：合并计税
    combined_taxable = total_taxable_income + other_comprehensive_income
    total_tax_combined = calculate_tax_by_brackets(combined_taxable)
    
    # 计算差异
    tax_diff = total_tax_separate - total_tax_combined
    if tax_diff < 0:
        optimal = "单独计税"
        savings = abs(tax_diff)
        reason = "股权激励单独计税可避免与综合所得叠加进入更高税率档，税负更低"
    elif tax_diff > 0:
        optimal = "并入综合所得"
        savings = abs(tax_diff)
        reason = "并入综合所得后可充分利用低税率档位，整体税负更低"
    else:
        optimal = "两者相同"
        savings = 0.0
        reason = "两种计税方式税负相同"
    
    return {
        'separate_tax': {
            'equity_tax': equity_tax_separate,
            'other_tax': other_tax_separate,
            'total_tax': total_tax_separate,
            'effective_rate': equity_tax_separate / total_taxable_income if total_taxable_income > 0 else 0.0,
            'effective_rate_basis': 'taxable_income',
        },
        'combined_tax': {
            'total_tax': total_tax_combined,
            'effective_rate': total_tax_combined / (total_taxable_income + other_comprehensive_income) if (total_taxable_income + other_comprehensive_income) > 0 else 0.0,
            'effective_rate_basis': 'taxable_income',
        },
        'comparison': {
            'tax_difference': tax_diff,
            'optimal_scheme': optimal,
            'savings': savings,
            'reason': reason
        }
    }

def calculate_stock_option_tax(
    grant_price: float,
    exercise_price: float,
    quantity: int,
    market_price: float,
    other_comprehensive_income: float = 0.0
) -> Dict:
    """
    股票期权行权税务计算
    
    Args:
        grant_price: 授予价格（元/股）
        exercise_price: 行权价格（元/股，通常等于授予价格）
        quantity: 行权数量（股）
        market_price: 行权日市场价格（元/股）
        other_comprehensive_income: 当年其他综合所得应纳税所得额
        
    Returns:
        包含税务计算结果的字典
    """
    # 应纳税所得额 = (行权日市场价 - 行权价) × 行权数量
    taxable_income_per_share = market_price - exercise_price
    total_taxable_income = taxable_income_per_share * quantity
    
    # 计算两种计税方式对比
    tax_compare = calculate_equity_tax_comparison(
        total_taxable_income,
        other_comprehensive_income
    )
    
    # 实际税负率（基于最优方案）
    optimal_equity_tax = tax_compare['separate_tax']['equity_tax'] if tax_compare['comparison']['optimal_scheme'] == "单独计税" else (tax_compare['combined_tax']['total_tax'] - tax_compare['separate_tax']['other_tax'])
    effective_rate = optimal_equity_tax / total_taxable_income if total_taxable_income > 0 else 0.0
    
    # 税后收益
    after_tax_profit = max(0.0, total_taxable_income) - optimal_equity_tax
    
    return {
        'income_type': '股票期权',
        'grant_price': grant_price,
        'exercise_price': exercise_price,
        'quantity': quantity,
        'market_price': market_price,
        'taxable_income_per_share': taxable_income_per_share,
        'total_taxable_income': max(0.0, total_taxable_income),
        'tax_comparison': tax_compare,
        'optimal_tax_amount': optimal_equity_tax,
        'effective_rate': effective_rate,
        'effective_rate_basis': 'taxable_income',
        'after_tax_profit': after_tax_profit,
        'other_comprehensive_income': other_comprehensive_income
    }

def calculate_restricted_stock_tax(
    grant_price: float,
    registration_price: float,
    market_price: float,
    quantity: int,
    paid_total: float = 0.0,
    total_acquired_quantity: int = 0,
    other_comprehensive_income: float = 0.0
) -> Dict:
    """
    限制性股票解禁税务计算
    
    Args:
        grant_price: 每股实际取得成本（元/股，用于简化口径）
        registration_price: 股票登记日市价（元/股，用于完整公式）
        market_price: 本批次解禁日市价（元/股）
        quantity: 本批次解禁数量（股）
        paid_total: 被激励对象实际支付的资金总额（元，用于完整公式）
        total_acquired_quantity: 获取的限制性股票总数（股，用于完整公式分摊）
        other_comprehensive_income: 当年其他综合所得应纳税所得额
        
    Returns:
        包含税务计算结果的字典
    """
    grant_price = max(0.0, grant_price)
    registration_price = max(0.0, registration_price)
    paid_total = max(0.0, paid_total)
    total_acquired_quantity = max(0, int(total_acquired_quantity))

    calculation_method = 'simplified'
    if registration_price > 0 and total_acquired_quantity > 0:
        calculation_method = 'full'
        avg_price = (registration_price + market_price) / 2
        allocated_paid = paid_total * (quantity / total_acquired_quantity)
        total_taxable_income = avg_price * quantity - allocated_paid
        taxable_income_per_share = total_taxable_income / quantity if quantity > 0 else 0.0
    else:
        taxable_income_per_share = market_price - grant_price
        total_taxable_income = taxable_income_per_share * quantity
    
    # 计算两种计税方式对比
    tax_compare = calculate_equity_tax_comparison(
        total_taxable_income,
        other_comprehensive_income
    )
    
    # 实际税负率（基于最优方案）
    optimal_equity_tax = tax_compare['separate_tax']['equity_tax'] if tax_compare['comparison']['optimal_scheme'] == "单独计税" else (tax_compare['combined_tax']['total_tax'] - tax_compare['separate_tax']['other_tax'])
    effective_rate = optimal_equity_tax / total_taxable_income if total_taxable_income > 0 else 0.0
    
    # 税后收益
    after_tax_profit = max(0.0, total_taxable_income) - optimal_equity_tax
    
    return {
        'income_type': '限制性股票',
        'grant_price': grant_price,
        'registration_price': registration_price if calculation_method == 'full' else None,
        'market_price': market_price,
        'quantity': quantity,
        'taxable_income_per_share': taxable_income_per_share,
        'total_taxable_income': max(0.0, total_taxable_income),
        'tax_comparison': tax_compare,
        'optimal_tax_amount': optimal_equity_tax,
        'effective_rate': effective_rate,
        'effective_rate_basis': 'taxable_income',
        'after_tax_profit': after_tax_profit,
        'other_comprehensive_income': other_comprehensive_income,
        'calculation_method': calculation_method,
        'paid_total': paid_total if calculation_method == 'full' else None,
        'total_acquired_quantity': total_acquired_quantity if calculation_method == 'full' else None,
    }

def calculate_stock_appreciation_right_tax(
    base_price: float,
    settlement_price: float,
    quantity: int,
    other_comprehensive_income: float = 0.0
) -> Dict:
    """
    股票增值权结算税务计算
    
    Args:
        base_price: 授予日价格（元/股）
        settlement_price: 结算日价格（元/股）
        quantity: 结算数量（股）
        other_comprehensive_income: 当年其他综合所得应纳税所得额
        
    Returns:
        包含税务计算结果的字典
    """
    # 应纳税所得额 = (结算日价格 - 授予日价格) × 结算数量
    taxable_income_per_share = settlement_price - base_price
    total_taxable_income = taxable_income_per_share * quantity
    
    # 计算两种计税方式对比
    tax_compare = calculate_equity_tax_comparison(
        total_taxable_income,
        other_comprehensive_income
    )
    
    # 实际税负率（基于最优方案）
    optimal_equity_tax = tax_compare['separate_tax']['equity_tax'] if tax_compare['comparison']['optimal_scheme'] == "单独计税" else (tax_compare['combined_tax']['total_tax'] - tax_compare['separate_tax']['other_tax'])
    effective_rate = optimal_equity_tax / total_taxable_income if total_taxable_income > 0 else 0.0
    
    # 税后收益
    after_tax_profit = max(0.0, total_taxable_income) - optimal_equity_tax
    
    return {
        'income_type': '股票增值权',
        'base_price': base_price,
        'settlement_price': settlement_price,
        'quantity': quantity,
        'taxable_income_per_share': taxable_income_per_share,
        'total_taxable_income': max(0.0, total_taxable_income),
        'tax_comparison': tax_compare,
        'optimal_tax_amount': optimal_equity_tax,
        'effective_rate': effective_rate,
        'effective_rate_basis': 'taxable_income',
        'after_tax_profit': after_tax_profit,
        'other_comprehensive_income': other_comprehensive_income
    }

def suggest_optimal_timing(
    current_taxable_income: float,
    equity_income: float,
    is_quarter_end: bool = False
) -> Dict:
    """
    股权激励行权/解禁时点建议
    适配单独计税政策
    """
    equity_income = max(0.0, equity_income)
    # 判断是否建议立即行权（基于单独计税的情况）
    combined_income = current_taxable_income + equity_income
    
    # 计算当前税负率
    current_tax = calculate_tax_by_brackets(current_taxable_income)
    # 单独计税下，两者互不影响
    equity_tax = calculate_tax_by_brackets(equity_income)
    total_tax_separate = current_tax + equity_tax
    
    # 合并计税下的总税负
    total_tax_combined = calculate_tax_by_brackets(combined_income)
    
    # 边际税率
    marginal_tax_rate = equity_tax / equity_income if equity_income > 0 else 0.0
    
    # 判断是否跨税率档
    current_bracket = None
    equity_bracket = None
    
    for i, (upper, rate, _) in enumerate(TAX_BRACKETS):
        if current_taxable_income <= upper:
            current_bracket = i
            break
    
    for i, (upper, rate, _) in enumerate(TAX_BRACKETS):
        if equity_income <= upper:
            equity_bracket = i
            break
    
    # 生成建议
    suggestions = []
    
    if current_bracket != equity_bracket:
        suggestions.append(f"✓ 股权激励单独计税，与其他收入税率档独立，互不影响")
        suggestions.append(f"  其他收入税率档：{TAX_BRACKETS[current_bracket][1]*100:.0f}%")
        suggestions.append(f"  股权激励税率档：{TAX_BRACKETS[equity_bracket][1]*100:.0f}%")
    else:
        suggestions.append("✓ 行权不会跨税率档，税负相对平稳")
        suggestions.append(f"  边际税率：{marginal_tax_rate*100:.2%}")
    
    # 季度末建议
    if is_quarter_end:
        suggestions.append("  当前为季度末，预扣预缴即将进行，建议关注税款支付时间")
    
    # 总体建议
    if equity_income > 300000:  # 超过20%税率
        suggestions.append("  综合考虑：建议咨询专业税务师制定分批行权计划")
    
    return {
        'current_taxable_income': current_taxable_income,
        'equity_income': equity_income,
        'marginal_tax_rate': marginal_tax_rate,
        'suggestions': suggestions
    }

def main():
    parser = argparse.ArgumentParser(description='股权激励税务计算工具（适配2025最新政策）')
    parser.add_argument('--type', type=str, required=True,
                       choices=['期权', '限制性股票', '股票增值权'],
                       help='股权激励类型')
    
    # 股票期权参数
    parser.add_argument('--grant-price', type=float,
                       help='授予价格（元/股）')
    parser.add_argument('--exercise-price', type=float,
                       help='行权价格（元/股）')
    parser.add_argument('--settlement-price', type=float,
                       help='结算价格（元/股，股票增值权专用）')
    parser.add_argument('--market-price', type=float,
                       help='行权/解禁日市场价格（元/股）')
    parser.add_argument('--registration-price', type=float,
                       help='股票登记日市价（元/股，仅限制性股票完整公式使用）')
    parser.add_argument('--paid-total', type=float,
                       help='被激励对象实际支付资金总额（元，仅限制性股票完整公式使用）')
    parser.add_argument('--total-acquired', type=int,
                       help='获取的限制性股票总数（股，仅限制性股票完整公式使用）')
    parser.add_argument('--quantity', type=int, required=True,
                       help='行权/解禁数量（股）')
    parser.add_argument('--other-income', type=float, default=0.0,
                       help='当年其他综合所得应纳税所得额（元）')
    parser.add_argument('--suggest-timing', action='store_true',
                       help='提供行权/解禁时点建议')
    parser.add_argument('--output-format', type=str, default='json',
                       choices=['json', 'text'],
                       help='输出格式')
    
    args = parser.parse_args()
    
    # 验证参数
    if args.quantity <= 0:
        print("错误：数量必须大于0")
        return
    
    if args.market_price is not None and args.market_price <=0:
        print("错误：市场价格必须大于0")
        return
    
    if args.exercise_price is not None and args.exercise_price <=0:
        print("错误：行权价格必须大于0")
        return
    
    if args.registration_price is not None and args.registration_price <= 0:
        print("错误：登记日市价必须大于0")
        return
    
    if args.paid_total is not None and args.paid_total < 0:
        print("错误：实际支付资金总额不能为负数")
        return
    
    if args.total_acquired is not None and args.total_acquired <= 0:
        print("错误：获取的限制性股票总数必须大于0")
        return
    
    # 根据类型调用不同计算函数
    result = None
    
    if args.type == '期权':
        # 避免0值误判
        if args.exercise_price is None or args.market_price is None:
            print("错误：股票期权需要提供行权价格和市场价格")
            return
        
        result = calculate_stock_option_tax(
            grant_price=args.grant_price if args.grant_price is not None else args.exercise_price,
            exercise_price=args.exercise_price,
            quantity=args.quantity,
            market_price=args.market_price,
            other_comprehensive_income=args.other_income
        )
        
    elif args.type == '限制性股票':
        if args.grant_price is None or args.market_price is None:
            print("错误：限制性股票需要提供授予价格和解禁日市场价格")
            return
        
        result = calculate_restricted_stock_tax(
            grant_price=args.grant_price,
            registration_price=args.registration_price if args.registration_price is not None else 0.0,
            market_price=args.market_price,
            quantity=args.quantity,
            paid_total=args.paid_total if args.paid_total is not None else 0.0,
            total_acquired_quantity=args.total_acquired if args.total_acquired is not None else 0,
            other_comprehensive_income=args.other_income
        )
        
    elif args.type == '股票增值权':
        if args.grant_price is None or args.settlement_price is None:
            print("错误：股票增值权需要提供授予价格和结算价格")
            return
        
        result = calculate_stock_appreciation_right_tax(
            base_price=args.grant_price,
            settlement_price=args.settlement_price,
            quantity=args.quantity,
            other_comprehensive_income=args.other_income
        )
    
    # 添加时点建议（如果启用）
    if args.suggest_timing:
        timing_suggestion = suggest_optimal_timing(
            current_taxable_income=args.other_income,
            equity_income=result['total_taxable_income']
        )
        result['timing_suggestion'] = timing_suggestion
    
    # 输出结果
    if args.output_format == 'json':
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print("=" * 60)
        print(f"{result['income_type']}税务计算结果")
        print("=" * 60)
        
        # 基本信息
        print(f"\n{'基本信息'}")
        print("-" * 60)
        print(f"  数量：{result['quantity']:,} 股")
        
        if args.type == '期权':
            print(f"  授予价格：¥{result['grant_price']:.2f}/股")
            print(f"  行权价格：¥{result['exercise_price']:.2f}/股")
            print(f"  行权日市价：¥{result['market_price']:.2f}/股")
        elif args.type == '限制性股票':
            print(f"  每股取得成本：¥{result['grant_price']:.2f}/股")
            if result.get('registration_price') is not None:
                print(f"  登记日市价：¥{result['registration_price']:.2f}/股")
            print(f"  解禁日价格：¥{result['market_price']:.2f}/股")
        elif args.type == '股票增值权':
            print(f"  授予日价格：¥{result['base_price']:.2f}/股")
            print(f"  结算日价格：¥{result['settlement_price']:.2f}/股")
        
        # 税务计算
        print(f"\n{'税务计算'}")
        print("-" * 60)
        print(f"  每股应纳税所得额：¥{result['taxable_income_per_share']:.2f}")
        print(f"  应纳税所得额总额：¥{result['total_taxable_income']:,.2f}")
        print(f"  最优方案应纳税额：¥{result['optimal_tax_amount']:,.2f}")
        print(f"  实际税负率：{result['effective_rate']:.2%}")
        print(f"  税后收益：¥{result['after_tax_profit']:,.2f}")
        
        # 计税方案对比
        print(f"\n{'计税方案对比'}")
        print("-" * 60)
        comp = result['tax_comparison']['comparison']
        sep = result['tax_comparison']['separate_tax']
        comb = result['tax_comparison']['combined_tax']
        print(f"  方案1：单独计税  总税额：¥{sep['total_tax']:,.2f}")
        print(f"  方案2：合并计税  总税额：¥{comb['total_tax']:,.2f}")
        print(f"  最优方案：{comp['optimal_scheme']}，可节税：¥{comp['savings']:,.2f}")
        print(f"  说明：{comp['reason']}")
        
        # 时点建议
        if 'timing_suggestion' in result:
            print(f"\n{'行权/解禁时点建议'}")
            print("=" * 60)
            for suggestion in result['timing_suggestion']['suggestions']:
                print(f"  {suggestion}")

if __name__ == '__main__':
    main()
