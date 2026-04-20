TAX_BRACKETS = [
    (36000, 0.03, 0),
    (144000, 0.10, 2520),
    (300000, 0.20, 16920),
    (420000, 0.25, 31920),
    (660000, 0.30, 52920),
    (960000, 0.35, 85920),
    (float("inf"), 0.45, 181920),
]


def calculate_tax_by_brackets(taxable_income: float) -> float:
    if taxable_income <= 0:
        return 0.0

    for upper_bound, rate, quick_deduction in TAX_BRACKETS:
        if taxable_income <= upper_bound:
            tax = taxable_income * rate - quick_deduction
            return max(0.0, tax)

    return max(0.0, taxable_income * 0.45 - 181920)
