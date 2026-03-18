#!/bin/bash
# Anaphase-Helix 智慧资产同步脚本

echo "====================================="
echo "  Helix 进化基因同步中..."
echo "====================================="

# 1. 确保在主目录
cd /opt/anaphase

# 2. 检查是否有新变进
# 我们只添加：
# - global_mind/ (武器库与指标)
# - core/ (系统逻辑)
# - README.md (宣言)
# - 以及脚本本身
git add global_mind/ core/ README.md arbiter_loop.py run_evolution.py sync_helix.sh

# 3. 提交
commit_msg="Evolution Pulse: $(date '+%Y-%m-%d %H:%M:%S')"
if git commit -m "$commit_msg"; then
    # 4. 推送
    if git push origin main; then
        echo -e "\n✅ [成功] 进化基因已同步至 GitHub 观赏台！"
    else
        echo -e "\n❌ [失败] 推送异常，请检查网络或 SSH Key。"
    fi
else
    echo -e "\nℹ️ [跳过] 本轮无基因变动，无需推送。"
fi
