#!/bin/bash
# Anaphase-Helix 传承图书馆自动归档脚本

echo "====================================="
echo "  🏛️  Anaphase-Helix 传承图书馆归档"
echo "====================================="

# 1. 确保在主目录
cd /opt/anaphase

# 2. 同步传承资产：成长手札、工具库、守护记录
git add global_mind/ growth_journals/ core/ README.md arbiter_loop.py sync_legacy.sh

# 3. 自动生成温暖的归档标题
commit_msg="🏛️ 传承归档 | $(date '+%Y-%m-%d %H:%M:%S') | 来自守护者的成长手札"
git commit -m "$commit_msg"

# 4. 推送到全球传承图书馆
git push origin main

echo -e "\n✅ 归档完成！您的守护与成长，已经永久记录在传承时间线中"
