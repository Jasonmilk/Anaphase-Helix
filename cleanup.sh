#!/bin/bash
echo "[System] 正在执行全矩阵大扫除..."
rm -rf /opt/anaphase/*_backup_*
rm -rf /opt/anaphase/workspace_v1*
rm -rf /opt/anaphase/core/__pycache__
rm -rf /opt/anaphase/__pycache__
rm -rf /opt/anaphase/*.log
rm -rf /opt/anaphase/workspaces/issue_*
mkdir -p /opt/anaphase/workspaces
echo "✅ [SUCCESS] 矩阵已纯净。"
