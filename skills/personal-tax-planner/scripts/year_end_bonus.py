#!/usr/bin/env python3
"""
年终奖计税方案对比工具
对比年终奖单独计税与并入综合所得两种方案的税负差异
"""
import argparse
import json
from typing import Dict

from common_tax import calculate_tax_by_brackets

# 按月换算后的综合所得税率表（用于年终奖单独计税）
# 依据：财税〔2018〕164号，速算扣除数为月度值（综合旧版本正确配置）
BONUS_MONTHLY_BRACKETS = [
    (3000, 0.03, 0),
    (12000, 0.10, 210),
    (25000, 0.20, 1410),
    (35000, 0.25, 2660),
    (55000, 0.30, 4410),
    (80000, 0.35, 7160),
    (float('inf'), 0.45, 15160),
]

def calculate_year_end_bonus_separate(bonus: float) -> float:
    """
    年终奖单独计税
    
    计算方法（依据财税〔2018〕164号）：
    1. 将年终奖除以12，得到月均值
    2. 用月均值查【按月换算后的综合所得税率表】确定税率和速算扣除数
    3. 应纳税额 = 年终奖 × 税率 - 速算扣除数（月度速算扣除数）
    
    Args:
        bonus: 年终奖金额
        
    Returns:
        应纳税额
    """
    if bonus <= 0:
        return 0.0
    
    # 年终奖除以12，确定适用税率和速算扣除数
    monthly_equivalent = bonus / 12
    
    for upper_bound, rate, quick_deduction in BONUS_MONTHLY_BRACKETS:
        if monthly_equivalent <= upper_bound:
            # 直接用全年一次性奖金乘以税率，减去速算扣除数
            tax = bonus * rate - quick_deduction
            return max(0.0, tax)
    
    return max(0.0, bonus * 0.45 - 15160)

def calculate_bonus_into_comprehensive(
    comprehensive_taxable_income: float,
    bonus: float
) -> float:
    """
    年终奖并入综合所得计税
    
    Args:
        comprehensive_taxable_income: 原综合所得应纳税所得额（不含年终奖）
        bonus: 年终奖金额
        
    Returns:
        应纳税额
    """
    total_taxable = comprehensive_taxable_income + bonus
    return calculate_tax_by_brackets(max(0.0, total_taxable))

def compare_bonus_schemes(
    comprehensive_taxable_income: float,
    bonus: float
) -> Dict:
    """
    对比两种计税方案
    
    Args:
        comprehensive_taxable_income: 综合所得应纳税所得额（不含年终奖）
        bonus: 年终奖金额
        
    Returns:
        包含两种方案税负和对比结果的字典
    """
    # 方案一：年终奖单独计税
    # 综合所得税额
    comprehensive_tax = calculate_tax_by_brackets(max(0.0, comprehensive_taxable_income))
    # 年终奖单独计税额
    bonus_tax_separate = calculate_year_end_bonus_separate(bonus)
    # 方案一总税额
    total_tax_separate = comprehensive_tax + bonus_tax_separate
    
    # 方案二：年终奖并入综合所得
    total_tax_together = calculate_bonus_into_comprehensive(
        comprehensive_taxable_income,
        bonus
    )
    
    # 计算税负差异
    tax_difference = total_tax_separate - total_tax_together
    
    # 判断最优方案
    if tax_difference > 0:
        # 单独计税税负更高，并入更优
        optimal_scheme = "并入综合所得"
        savings = abs(tax_difference)
        reason = "年终奖并入综合所得后，由于累进税率效应，整体税负更低"
    elif tax_difference < 0:
        # 单独计税税负更低，单独计税更优
        optimal_scheme = "单独计税"
        savings = abs(tax_difference)
        reason = "年终奖单独计税可以避免与综合所得叠加进入更高税率档"
    else:
        optimal_scheme = "两者相同"
        savings = 0.0
        reason = "两种方案税负相同"
    
    # 税负率对比
    total_income = comprehensive_taxable_income + bonus
    rate_separate = total_tax_separate / total_income if total_income > 0 else 0.0
    rate_together = total_tax_together / total_income if total_income > 0 else 0.0
    
    return {
        'separate_scheme': {
            'comprehensive_tax': comprehensive_tax,
            'bonus_tax': bonus_tax_separate,
            'total_tax': total_tax_separate,
            'effective_rate': rate_separate,
            'effective_rate_basis': 'taxable_income',
        },
        'together_scheme': {
            'total_tax': total_tax_together,
            'effective_rate': rate_together,
            'effective_rate_basis': 'taxable_income',
        },
        'comparison': {
            'tax_difference': tax_difference,
            'optimal_scheme': optimal_scheme,
            'savings': savings,
            'reason': reason
        },
        'input_data': {
            'comprehensive_taxable_income': comprehensive_taxable_income,
            'bonus': bonus,
            'total_income': total_income
        }
    }

def identify_bonus_tax_trap(bonus: float) -> Dict:
    """
    识别年终奖"无效区间"（税负陷阱）
    """
    if bonus <= 0:
        return {'has_trap': False}
    
    traps = []
    thresholds = [int(upper_bound * 12) for upper_bound, _, _ in BONUS_MONTHLY_BRACKETS[:-1]]
    for i, boundary in enumerate(thresholds):
        if i >= len(BONUS_MONTHLY_BRACKETS) - 1:
            break
        next_rate = BONUS_MONTHLY_BRACKETS[i + 1][1]
        next_quick = BONUS_MONTHLY_BRACKETS[i + 1][2]
        
        tax_at_boundary = calculate_year_end_bonus_separate(boundary)
        after_tax_at_boundary = boundary - tax_at_boundary
        
        tax_at_boundary_plus_1 = calculate_year_end_bonus_separate(boundary + 1)
        after_tax_at_boundary_plus_1 = (boundary + 1) - tax_at_boundary_plus_1
        
        if after_tax_at_boundary_plus_1 < after_tax_at_boundary:
            end_point = (after_tax_at_boundary - next_quick) / (1 - next_rate)
            next_threshold = thresholds[i + 1] if i + 1 < len(thresholds) else None
            range_end = min(end_point, next_threshold) if next_threshold is not None else end_point
            if range_end > boundary:
                traps.append({
                    'range_start': boundary,
                    'range_end': range_end,
                    'tax_loss_at_boundary_plus_1': after_tax_at_boundary - after_tax_at_boundary_plus_1
                })
    
    # 检查当前年终奖是否在陷阱区间
    in_trap = False
    trap_info = None
    for trap in traps:
        if trap['range_start'] < bonus < trap['range_end']:
            in_trap = True
            trap_info = trap
            break
    
    return {
        'has_trap': in_trap,
        'trap_info': trap_info,
        'all_traps': traps
    }

def main():
    parser = argparse.ArgumentParser(description='年终奖计税方案对比工具')
    parser.add_argument('--comprehensive-income', type=float, required=True,
                       help='综合所得应纳税所得额（不含年终奖，单位：元）')
    parser.add_argument('--bonus', type=float, required=True,
                       help='年终奖金额（单位：元）')
    parser.add_argument('--check-trap', action='store_true',
                       help='检查年终奖税负陷阱')
    parser.add_argument('--output-format', type=str, default='json',
                       choices=['json', 'text'],
                       help='输出格式')
    
    args = parser.parse_args()
    
    # 验证输入
    if args.comprehensive_income < 0:
        print("错误：综合所得应纳税所得额不能为负数")
        return
    
    if args.bonus < 0:
        print("错误：年终奖金额不能为负数")
        return
    
    # 方案对比
    result = compare_bonus_schemes(args.comprehensive_income, args.bonus)
    
    # 检查税负陷阱（可选）
    if args.check_trap:
        trap_result = identify_bonus_tax_trap(args.bonus)
        result['tax_trap_analysis'] = trap_result
    
    # 输出结果
    if args.output_format == 'json':
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print("=" * 60)
        print("年终奖计税方案对比")
        print("=" * 60)
        print(f"\n输入信息：")
        print(f"  综合所得应纳税所得额：¥{args.comprehensive_income:,.2f}")
        print(f"  年终奖金额：¥{args.bonus:,.2f}")
        print(f"  合计应纳税所得额：¥{args.comprehensive_income + args.bonus:,.2f}")
        
        print(f"\n{'方案一：年终奖单独计税'}")
        print("-" * 60)
        print(f"  综合所得税额：¥{result['separate_scheme']['comprehensive_tax']:,.2f}")
        print(f"  年终奖税额：¥{result['separate_scheme']['bonus_tax']:,.2f}")
        print(f"  总税额：¥{result['separate_scheme']['total_tax']:,.2f}")
        print(f"  实际税负率：{result['separate_scheme']['effective_rate']:.2%}")
        
        print(f"\n{'方案二：年终奖并入综合所得'}")
        print("-" * 60)
        print(f"  总税额：¥{result['together_scheme']['total_tax']:,.2f}")
        print(f"  实际税负率：{result['together_scheme']['effective_rate']:.2%}")
        
        print(f"\n{'对比分析'}")
        print("=" * 60)
        comparison = result['comparison']
        print(f"  税负差异：¥{comparison['tax_difference']:,.2f}")
        print(f"  最优方案：{comparison['optimal_scheme']}")
        if comparison['savings'] > 0:
            print(f"  可节税：¥{comparison['savings']:,.2f}")
        print(f"  原因：{comparison['reason']}")
        
        # 税负陷阱分析（如果启用）
        if 'tax_trap_analysis' in result:
            print(f"\n{'税负陷阱分析'}")
            print("=" * 60)
            trap_result = result['tax_trap_analysis']
            if trap_result['has_trap']:
                trap = trap_result['trap_info']
                print(f"  ⚠️  警告：年终奖处于税负陷阱区间！")
                print(f"  陷阱区间：¥{trap['range_start']:,.2f} ~ ¥{trap['range_end']:,.2f}")
                print(f"  临界点+1元税负损失：¥{trap['tax_loss_at_boundary_plus_1']:,.2f}")
                print(f"  建议：调整年终奖金额至区间外")
            else:
                print(f"  ✓ 当前年终奖金额未落入税负陷阱区间")

if __name__ == '__main__':
    main()
