#!/bin/bash
echo "[System] 正在同步进化资产至 GitHub..."
cd /opt/anaphase
git add core/ global_mind/ README.md evolution_briefing.md *.py *.sh .gitignore
git commit -m "EVO: Pulse - System hardened and modularized"
git push origin main
echo "✅ [SUCCESS] 观赏台已更新。"
