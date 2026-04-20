---
name: webapp-testing
description: 用于与本地 Web 应用程序交互和测试的工具包。支持验证前端功能、调试 UI 行为、捕获浏览器截图和查看浏览器日志。尽可能使用标准工具。
license: 完整条款见 LICENSE.txt
---

# Web 应用程序测试

要测试本地 Web 应用程序，您可以使用 Python Playwright 脚本或系统提供的标准工具。

**可用的辅助脚本**：
- `with_server.py` - 管理服务器生命周期（支持多个服务器）

**请始终先使用 `--help` 运行脚本** 查看用法。在尝试运行脚本并发现确实需要自定义解决方案之前，不要阅读源代码。这些脚本可能非常大，会污染您的上下文窗口。它们的存在是为了作为黑盒脚本直接调用，而不是被摄入到您的上下文窗口中。

## 决策树：选择您的方法

```
用户任务 → 是静态 HTML 吗？
    ├─ 是 → 直接读取 HTML 文件以识别选择器
    │         ├─ 成功 → 使用标准浏览器工具或 Playwright 脚本
    │         └─ 失败/不完整 → 按动态处理（如下）
    │
    └─ 否（动态 Web 应用） → 服务器是否已运行？
        ├─ 否 → 运行：python with_server.py --help
        │        然后使用辅助脚本 + 编写简化的 Playwright 脚本
        │
        └─ 是 → 选择方法：
            1. 标准工具：使用 open_url_in_browser、click、type、take_screenshot
            2. Playwright：使用详细的自动化，包括元素发现和日志记录
```

## 使用标准工具

对于基本的 Web 应用程序测试，您可以使用以下标准工具：

1. **打开 URL**：`open_url_in_browser` - 在浏览器中打开网页
2. **与元素交互**：`click`、`type`、`scroll` - 执行基本的浏览器交互
3. **捕获截图**：`take_screenshot` - 捕获当前浏览器状态
4. **运行命令**：`ShellExec` - 启动服务器或运行其他命令
5. **等待命令**：`ShellWait` - 等待服务器启动

### 示例：使用标准工具进行基本测试

```python
# 1. 启动服务器（如果未运行）
# ShellExec: "npm run dev"

# 2. 等待服务器启动
# ShellWait

# 3. 打开浏览器
# open_url_in_browser: "http://localhost:5173"

# 4. 与页面交互
# click: "button#login"
# type: "input#username", "testuser"
# type: "input#password", "password"
# click: "button#submit"

# 5. 捕获截图
# take_screenshot
```

## 使用 Playwright（高级）

对于更高级的测试场景，使用 Playwright 脚本：

### 示例：使用 with_server.py

要启动服务器，请先运行 `--help`，然后使用辅助脚本：

**单个服务器**：
```bash
python with_server.py --server "npm run dev" --port 5173 -- python your_automation.py
```

**多个服务器（例如后端 + 前端）**：
```bash
python with_server.py \
  --server "cd backend && python server.py" --port 3000 \
  --server "cd frontend && npm run dev" --port 5173 \
  -- python your_automation.py
```

要创建自动化脚本，只需要包含 Playwright 逻辑（服务器会自动管理）：
```python
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True) # 始终以无头模式启动 chromium
    page = browser.new_page()
    page.goto('http://localhost:5173') # 服务器已运行并准备就绪
    page.wait_for_load_state('networkidle') # 关键：等待 JS 执行
    # ... 您的自动化逻辑
    browser.close()
```

## 侦察-然后-行动模式

1. **检查渲染的 DOM**：
   ```python
   page.screenshot(path='/tmp/inspect.png', full_page=True)
   content = page.content()
   page.locator('button').all()
   ```

2. **从检查结果中识别选择器**

3. **使用发现的选择器执行操作**

## 常见陷阱

❌ **不要** 在动态应用上等待 `networkidle` 之前检查 DOM
✅ **要** 在检查之前等待 `page.wait_for_load_state('networkidle')`

## 最佳实践

- **基本任务使用标准工具** - 对于简单的交互，使用标准浏览器工具
- **高级任务使用 Playwright** - 对于复杂的自动化，使用 Playwright 脚本
- **将捆绑脚本用作黑盒** - 要完成任务，请考虑是否有可用的脚本可以帮助。这些脚本可以可靠地处理常见的复杂工作流，而不会使上下文窗口混乱。使用 `--help` 查看用法，然后直接调用。
- 使用 `sync_playwright()` 编写同步脚本
- 完成后始终关闭浏览器
- 使用描述性选择器：`text=`、`role=`、CSS 选择器或 ID
- 添加适当的等待：`page.wait_for_selector()` 或 `page.wait_for_timeout()`

## 参考文件

- 展示常见模式的示例：
  - `element_discovery.py` - 发现页面上的按钮、链接和输入字段
  - `static_html_automation.py` - 使用 file:// URLs 处理本地 HTML
  - `console_logging.py` - 捕获自动化过程中的控制台日志