#!/usr/bin/env python3
"""
个人所得税计算器 - 综合所得与分类所得
支持工资薪金、劳务报酬、稿酬、特许权使用费
"""
import argparse
import json
from dataclasses import dataclass, asdict
from typing import Optional, Dict

from common_tax import calculate_tax_by_brackets

@dataclass
class TaxResult:
    """税负计算结果"""
    total_income: float  # 总收入
    taxable_income: float  # 应纳税所得额
    tax_amount: float  # 应纳税额
    effective_rate: float  # 实际税负率
    income_adjustment: Dict[str, float]  # 各项收入调整后明细
    details: Dict  # 计算详情

def adjust_labor_income(amount: float) -> float:
    """
    劳务报酬收入调整：扣除20%费用
    """
    return max(0.0, amount * 0.8)

def adjust_author_income(amount: float) -> float:
    """
    稿酬收入调整：扣除20%费用，再减征30%
    """
    return max(0.0, amount * 0.8 * 0.7)

def adjust_royalty_income(amount: float) -> float:
    """
    特许权使用费收入调整：扣除20%费用
    """
    return max(0.0, amount * 0.8)

def calculate_comprehensive_income(
    salary: float = 0.0,
    labor: float = 0.0,
    author: float = 0.0,
    royalty: float = 0.0,
    social_insurance: float = 0.0,
    special_deductions: float = 0.0,
    other_deductions: float = 0.0,
    standard_deduction: float = 60000.0
) -> TaxResult:
    """
    计算综合所得个人所得税
    """
    # 限制扣除项不能为负
    social_insurance = max(0.0, social_insurance)
    special_deductions = max(0.0, special_deductions)
    other_deductions = max(0.0, other_deductions)
    
    # 计算各项调整后收入
    salary_adjusted = max(0.0, salary)
    labor_adjusted = adjust_labor_income(labor)
    author_adjusted = adjust_author_income(author)
    royalty_adjusted = adjust_royalty_income(royalty)
    
    # 总收入（税前）
    total_income = salary + labor + author + royalty
    
    # 综合所得应纳税所得额
    comprehensive_income = (
        salary_adjusted +
        labor_adjusted +
        author_adjusted +
        royalty_adjusted -
        standard_deduction -
        social_insurance -
        special_deductions -
        other_deductions
    )
    
    # 计算应纳税额
    tax_amount = calculate_tax_by_brackets(max(0.0, comprehensive_income))
    
    # 计算实际税负率
    effective_rate = tax_amount / total_income if total_income > 0 else 0.0
    
    # 各项收入调整后明细
    income_adjustment = {}
    if salary > 0:
        income_adjustment['salary'] = salary_adjusted
    if labor > 0:
        income_adjustment['labor'] = labor_adjusted
    if author > 0:
        income_adjustment['author'] = author_adjusted
    if royalty > 0:
        income_adjustment['royalty'] = royalty_adjusted
    
    # 计算详情
    details = {
        'salary_adjusted': salary_adjusted,
        'labor_adjusted': labor_adjusted,
        'author_adjusted': author_adjusted,
        'royalty_adjusted': royalty_adjusted,
        'social_insurance': social_insurance,
        'special_deductions': special_deductions,
        'other_deductions': other_deductions,
        'standard_deduction': standard_deduction,
        'total_deductions': standard_deduction + social_insurance + special_deductions + other_deductions,
        'taxable_income': max(0.0, comprehensive_income),
        'effective_rate_basis': 'gross_income',
    }
    
    return TaxResult(
        total_income=total_income,
        taxable_income=max(0.0, comprehensive_income),
        tax_amount=tax_amount,
        effective_rate=effective_rate,
        income_adjustment=income_adjustment,
        details=details
    )

def calculate_classified_income(
    income_type: str,
    amount: float,
    rent_per_time_amount: Optional[float] = None,
    rent_times: Optional[int] = None,
    original_value: Optional[float] = None,
    reasonable_fees: Optional[float] = None,
) -> TaxResult:
    """
    计算分类所得个人所得税（单独计税）
    """
    amount = max(0.0, amount)
    total_income = amount
    # 分类所得税率统一为20%
    classified_tax_rate = 0.20
    
    if income_type in ['interest', 'dividend']:
        # 利息、股息、红利所得：直接按20%计税
        tax_amount = amount * classified_tax_rate
        taxable_income = amount
    elif income_type == 'rent':
        if rent_per_time_amount is not None and rent_times is not None:
            rent_per_time_amount = max(0.0, rent_per_time_amount)
            rent_times = max(0, int(rent_times))
            total_income = rent_per_time_amount * rent_times
            if rent_per_time_amount <= 4000:
                taxable_per_time = max(0.0, rent_per_time_amount - 800)
            else:
                taxable_per_time = rent_per_time_amount * 0.8
            taxable_income = taxable_per_time * rent_times
        else:
            taxable_income = amount * 0.8
        tax_amount = taxable_income * classified_tax_rate
    elif income_type == 'property_transfer':
        # 财产转让所得：扣除原值和合理费用后按20%计税
        original_value = max(0.0, original_value or 0.0)
        reasonable_fees = max(0.0, reasonable_fees or 0.0)
        taxable_income = max(0.0, amount - original_value - reasonable_fees)
        tax_amount = taxable_income * classified_tax_rate
    else:
        raise ValueError(f"不支持的分类所得类型: {income_type}")
    
    effective_rate = tax_amount / total_income if total_income > 0 else 0.0
    
    return TaxResult(
        total_income=total_income,
        taxable_income=taxable_income,
        tax_amount=tax_amount,
        effective_rate=effective_rate,
        income_adjustment={income_type: taxable_income},
        details={
            'tax_rate': classified_tax_rate,
            'rent_per_time_amount': rent_per_time_amount,
            'rent_times': rent_times,
            'original_value': original_value,
            'reasonable_fees': reasonable_fees,
            'effective_rate_basis': 'gross_income',
        }
    )

def main():
    parser = argparse.ArgumentParser(description='个人所得税计算器')
    parser.add_argument('--income-type', type=str, required=True,
                       choices=['综合所得', '分类所得'],
                       help='收入类型')
    
    # 综合所得参数
    parser.add_argument('--salary', type=float, default=0.0,
                       help='工资薪金收入（元）')
    parser.add_argument('--labor', type=float, default=0.0,
                       help='劳务报酬收入（元）')
    parser.add_argument('--author', type=float, default=0.0,
                       help='稿酬收入（元）')
    parser.add_argument('--royalty', type=float, default=0.0,
                       help='特许权使用费收入（元）')
    parser.add_argument('--social-insurance', type=float, default=0.0,
                       help='专项扣除-社保公积金个人缴费年合计（元）')
    parser.add_argument('--special-deductions', type=float, default=0.0,
                       help='专项附加扣除总额（元），含子女教育、赡养老人等六项')
    parser.add_argument('--other-deductions', type=float, default=0.0,
                       help='其他扣除（元），含企业年金、个人养老金等')
    parser.add_argument('--standard-deduction', type=float, default=60000.0,
                       help='基本减除费用（元，默认60000）')
    
    # 分类所得参数
    parser.add_argument('--classified-type', type=str,
                       choices=['interest', 'dividend', 'rent', 'property_transfer'],
                       help='分类所得具体类型')
    parser.add_argument('--amount', type=float, default=0.0,
                       help='分类所得金额（元）')
    parser.add_argument('--rent-per-time-amount', type=float,
                       help='财产租赁按次收入金额（元）；与 --rent-times 同时提供时按“≤4000减800，否则减20%费用”计算')
    parser.add_argument('--rent-times', type=int,
                       help='财产租赁按次次数（次）；与 --rent-per-time-amount 同时提供时生效')
    parser.add_argument('--original-value', type=float,
                       help='财产转让原值（元，仅 property_transfer 使用）')
    parser.add_argument('--reasonable-fees', type=float,
                       help='财产转让合理费用（元，仅 property_transfer 使用）')
    
    parser.add_argument('--output-format', type=str, default='json',
                       choices=['json', 'text'],
                       help='输出格式')
    
    args = parser.parse_args()
    
    if args.income_type == '综合所得':
        # 验证至少有一项收入
        total = args.salary + args.labor + args.author + args.royalty
        if total <= 0:
            print("错误：综合所得至少需要一项收入来源")
            return
        
        result = calculate_comprehensive_income(
            salary=args.salary,
            labor=args.labor,
            author=args.author,
            royalty=args.royalty,
            social_insurance=args.social_insurance,
            special_deductions=args.special_deductions,
            other_deductions=args.other_deductions,
            standard_deduction=args.standard_deduction
        )
        
    else:  # 分类所得
        if not args.classified_type:
            print("错误：分类所得需要指定类型")
            return
        if args.classified_type == 'rent':
            if (args.rent_per_time_amount is None) ^ (args.rent_times is None):
                print("错误：使用按次口径时需同时提供 --rent-per-time-amount 和 --rent-times")
                return
            if args.rent_per_time_amount is None and args.amount <= 0:
                print("错误：财产租赁需提供 --amount（汇总口径）或按次口径参数")
                return
            if args.rent_per_time_amount is not None and (args.rent_per_time_amount <= 0 or args.rent_times <= 0):
                print("错误：财产租赁按次口径参数必须为正数")
                return
        else:
            if args.amount <= 0:
                print("错误：分类所得需要提供 --amount")
                return
        
        result = calculate_classified_income(
            income_type=args.classified_type,
            amount=args.amount,
            rent_per_time_amount=args.rent_per_time_amount,
            rent_times=args.rent_times,
            original_value=args.original_value,
            reasonable_fees=args.reasonable_fees,
        )
    
    # 输出结果
    if args.output_format == 'json':
        output = asdict(result)
        print(json.dumps(output, indent=2, ensure_ascii=False))
    else:
        print("=" * 50)
        print("个人所得税计算结果")
        print("=" * 50)
        print(f"总收入：¥{result.total_income:,.2f}")
        print(f"应纳税所得额：¥{result.taxable_income:,.2f}")
        print(f"应纳税额：¥{result.tax_amount:,.2f}")
        print(f"实际税负率：{result.effective_rate:.2%}")
        print("-" * 50)
        print("各项收入调整后明细：")
        for income_type, adjusted in result.income_adjustment.items():
            if adjusted > 0:
                print(f"  {income_type}: ¥{adjusted:,.2f}")
        print("-" * 50)
        print("计算详情：")
        for key, value in result.details.items():
            print(f"  {key}: {value}")

if __name__ == '__main__':
    main()
