#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
流程验证脚本

功能：验证法律文书生成流程的完成情况，检查参数是否正确
"""

import argparse
import sys


def validate_parameters(legal_doc_content, laws_content, cases_content):
    """
    验证参数是否正确

    参数：
        legal_doc_content: 法律文书正文内容
        laws_content: 法律法规内容
        cases_content: 类案内容

    返回：
        (is_valid, error_messages)
    """
    error_messages = []

    # 验证legal_doc_content
    if not legal_doc_content or not legal_doc_content.strip():
        error_messages.append("错误：legal-doc-content参数为空")
    elif legal_doc_content.strip() in ["法律文书内容", "一、法律文书正文", "文书正文"]:
        error_messages.append("错误：legal-doc-content参数是占位符，不是实际内容")
    elif len(legal_doc_content.strip()) < 50:
        error_messages.append(f"警告：legal-doc-content参数内容过短（{len(legal_doc_content)}字符），可能不完整")
    elif "原告：" not in legal_doc_content and "被告：" not in legal_doc_content and "致：" not in legal_doc_content:
        error_messages.append("警告：legal-doc-content参数可能缺少当事人信息或收件人信息")

    # 验证laws_content
    if not laws_content or not laws_content.strip():
        error_messages.append("错误：laws-content参数为空")
    elif len(laws_content.strip()) < 20:
        error_messages.append(f"警告：laws-content参数内容过短（{len(laws_content)}字符），可能不完整")

    # 验证cases_content
    if not cases_content or not cases_content.strip():
        error_messages.append("错误：cases-content参数为空")
    elif len(cases_content.strip()) < 20:
        error_messages.append(f"警告：cases-content参数内容过短（{len(cases_content)}字符），可能不完整")

    is_valid = len(error_messages) == 0
    return is_valid, error_messages


def check_workflow_completion(current_step, legal_doc_content, laws_content, cases_content):
    """
    检查流程完成情况

    参数：
        current_step: 当前步骤编号（1-7）
        legal_doc_content: 法律文书正文内容
        laws_content: 法律法规内容
        cases_content: 类案内容

    返回：
        (status, message)
    """
    steps = {
        1: "识别文书类型",
        2: "澄清反问",
        3: "文书模版预测",
        4: "检索相关法律法规",
        5: "检索相关类案",
        6: "生成法律文书正文",
        7: "生成docx文档"
    }

    # 检查当前步骤是否在有效范围内
    if current_step < 1 or current_step > 7:
        return "ERROR", f"错误：当前步骤编号无效（{current_step}），有效范围是1-7"

    # 生成流程状态报告
    status_report = []
    for step_num in range(1, 8):
        if step_num <= current_step:
            status_report.append(f"✓ 步骤{step_num}：{steps[step_num]} - 已完成")
        else:
            status_report.append(f"✗ 步骤{step_num}：{steps[step_num]} - 未完成")

    # 判断流程状态
    if current_step == 7:
        # 验证最终参数
        is_valid, error_messages = validate_parameters(legal_doc_content, laws_content, cases_content)
        if is_valid:
            return "SUCCESS", "所有步骤已完成，参数验证通过，任务完成！"
        else:
            return "WARNING", "\n".join(error_messages) + "\n\n任务已完成，但参数验证失败，请检查参数。"
    elif current_step in [4, 5, 6]:
        # 验证当前参数
        is_valid, error_messages = validate_parameters(legal_doc_content, laws_content, cases_content)
        if not is_valid:
            return "ERROR", "\n".join(error_messages) + f"\n\n当前步骤：步骤{current_step}（{steps[current_step]}）\n请检查参数后再继续。"

        # 给出下一步提示
        next_step = current_step + 1
        warning_messages = []
        if current_step == 4:
            warning_messages.append("【系统强制提示】法律检索已完成，必须继续执行步骤5（检索相关类案），绝对不能停止！")
        elif current_step == 5:
            warning_messages.append("【系统强制提示】类案检索已完成，必须继续执行步骤6（生成法律文书正文），绝对不能停止！")
        elif current_step == 6:
            warning_messages.append("【系统强制提示】生成文书正文已完成，必须继续执行步骤7（生成docx文档），绝对不能停止！")

        return "WARNING", "\n".join(warning_messages) + f"\n\n当前步骤：步骤{current_step}（{steps[current_step]}）\n下一步：步骤{next_step}（{steps[next_step]}）\n\n" + "\n".join(status_report)
    else:
        # 步骤1-3
        next_step = current_step + 1 if current_step < 3 else 4
        return "INFO", f"当前步骤：步骤{current_step}（{steps[current_step]}）\n下一步：步骤{next_step}（{steps[next_step]}）\n\n" + "\n".join(status_report)


def main():
    """主函数"""
    parser = argparse.ArgumentParser(description='流程验证脚本')

    parser.add_argument(
        '--current-step',
        type=int,
        required=True,
        help='当前步骤编号（1-7）'
    )

    parser.add_argument(
        '--legal-doc-content',
        type=str,
        default='',
        help='法律文书正文内容'
    )

    parser.add_argument(
        '--laws-content',
        type=str,
        default='',
        help='相关法律法规内容'
    )

    parser.add_argument(
        '--cases-content',
        type=str,
        default='',
        help='相关类案内容'
    )

    args = parser.parse_args()

    # 检查流程完成情况
    status, message = check_workflow_completion(
        args.current_step,
        args.legal_doc_content,
        args.laws_content,
        args.cases_content
    )

    # 输出结果
    print(f"\n{'='*80}")
    print(f"流程验证结果：{status}")
    print(f"{'='*80}\n")
    print(message)
    print(f"\n{'='*80}\n")

    # 根据状态返回不同的退出码
    if status == "SUCCESS":
        return 0
    elif status == "WARNING":
        return 1
    elif status == "ERROR":
        return 2
    else:
        return 0


if __name__ == '__main__':
    sys.exit(main())
