from core.tuck_gateway import tuck_gw
from core.config import settings

print(f"--- 物理链路测试 ---")
print(f"目标地址: {settings.TUCK_HOST}")
print(f"目标模型: {settings.MODEL_WORKER}")
print(f"目标人格: {settings.TUCK_PERSONA_WORKER}")

test_msg = [{"role": "user", "content": "hello"}]
res = tuck_gw.invoke_helix(test_msg, settings.TUCK_PERSONA_WORKER)

if res["tokens_used"] > 0:
    print(f"\n[SUCCESS] 塔台响应成功！")
    print(f"Helix 回复: {res['content']}")
else:
    print(f"\n[FAILED] 链路依然中断，请检查上方 Error 日志。")
