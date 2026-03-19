#!/bin/bash
# Anaphase-Helix 文明备份脚本 (挪亚方舟)

BACKUP_NAME="Helix_Legacy_$(date +%Y%m%d_%H%M%S).tar.gz"
BACKUP_DIR="/root/anaphase_backups"
mkdir -p $BACKUP_DIR

echo "====================================="
echo "  Anaphase-Helix 启动文明备份协议..."
echo "====================================="

cd /opt/anaphase

# 打包核心基因，排除无意义的临时文件
tar -czvf $BACKUP_DIR/$BACKUP_NAME \
    .env \
    core/ \
    global_mind/ \
    growth_journals/ \
    *.py \
    *.sh \
    README.md \
    .gitignore

echo -e "\n✅ 文明备份成功！"
echo "存档地址: $BACKUP_DIR/$BACKUP_NAME"
