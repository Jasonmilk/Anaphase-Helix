---
name: authoritative-database-query-skill
description: 面向深入、复杂、权威数据的检索与分析能力；教会模型基于权威数据库网站进行精准查询，并优先使用浏览器GUI类工具完成交互与提取
version: 1.1.0
---

# 权威数据库网站深度检索与分析

本 Skill 明确定位为：在需要进行深入、复杂的权威数据搜索与分析时，指导模型通过各权威数据库“官方网站”完成定位、筛选、提取与交叉验证；工具使用上优先通过API访问数据库，其次，可以“采用浏览器的 GUI 类工具”进行页面级交互，必要时再回退到搜索定位。

## 1. 适用范围与触发场景

当用户问题涉及以下任何领域，且需要依赖可信的、公开的数据源进行回答时，应优先触发本 Skill。

- **事实与数据核查**：验证具体的数字、日期、统计数据或事实性陈述。
- **学术与科研查询**：查找论文、期刊、研究项目、引文数据等。
- **宏观经济与社会统计**：获取国家、地区或全球层面的经济指标、人口数据、发展指数等。
- **法律与法规检索**：查询特定国家或地区的法律条文、判例、上市公司官方披露（如 10-K/10-Q）等。
- **专利与知识产权**：搜索已注册或正在申请的专利信息。
- **地理与空间信息**：获取地理坐标、地图数据、行政区划、自然地貌等信息。
- **科学与生物医学数据**：查询化学物质、蛋白质序列、基因组数据、临床试验信息等。
- **企业与组织信息**：查找公司的注册信息、组织架构等。
- **常识与知识图谱查询**：需要结构化、可验证的通用知识时。

## 2. GUI 工具优先与操作原则

### 浏览器工具组（GUI）
- 可用动作：click, left_double, right_single, drag, scroll, move_to, mouse_down, mouse_up, type, hotkey, press, release, wait, take_screenshot, open_url_in_browser, AskHumanToControlBrowser

### 核心原则
- 官方网站优先：直接在权威数据源的“官网”页面完成检索与筛选。
- GUI 交互优先：用站内搜索框、筛选器、下载按钮等完成操作，减少对第三方摘要或非官方页面的依赖。
- 标识符与溯源：在结果中保留 DOI、PMID、FRED Series ID、OSM ID 等权威标识。
- 交叉验证：对关键结论用备选权威源进行复核；协作性内容需二次验证。
- 接口优先：若存在官方 API/开发接口（REST、SPARQL、SDMX 等），优先通过代码查询；GUI 仅作为备选。
 - 保留标准：在“4. 数据库分类与官方网站导航”中，仅保留“无法通过官方接口检索”的站点；具备公开接口的站点统一在“4.10 API 优先与代码接入”列出。

## 3. 总体工作流（面向网站 GUI）

遵循以下步骤，确保在“网站 GUI”环境下获得可靠结果：

1. 理解问题：明确实体、指标、维度与时间范围。
2. 识别领域：确定对应的网站类别与首选权威网站。
3. 打开官网：使用 open_url_in_browser 直达官网主页或数据入口。
4. 站内检索与筛选：使用 type 输入关键词；通过 click/scroll 操作筛选条件、下载或展开详情。
5. 提取与记录：在详情页使用 take_screenshot 佐证页面；记录权威标识符与直接链接。
6. 交叉验证：在备选权威网站重复查询或对比。
7. 输出规范：用中文总结并附上来源与标识符。

## 4. 数据库类型目录与选用原则

### 类型目录
- 学术与文献：详见 [academic-literature.md](./assets/academic-literature.md)
- 统计与宏观经济：详见 [macroeconomics.md](./assets/macroeconomics.md)
- 法规与披露：详见 [regulations-disclosures.md](./assets/regulations-disclosures.md)
- 专利：详见 [patents.md](./assets/patents.md)
- 公司注册信息：详见 [company-registries.md](./assets/company-registries.md)
- 地理与空间：详见 [geospatial.md](./assets/geospatial.md)
- 科学与公共数据：详见 [science-public-data.md](./assets/science-public-data.md)
- 知识图谱与通识：详见 [knowledge-graph.md](./assets/knowledge-graph.md)
- 科技咨询与行业报告：详见 [tech-consulting.md](./assets/tech-consulting.md)

### 选用原则
- 依据问题类型选择对应 assets 文件
- 若存在官方 API，优先程序化查询；GUI 用于无法接口检索或页面细节
- 输出保留权威标识符与来源链接，并按需做交叉验证
### 4.1 学术与文献
仅保留中文与无公开接口的数据源；具备 API 的学术站点请参见 4.10

详见 [academic-literature.md](./assets/academic-literature.md)

### 4.4 专利
详见 [patents.md](./assets/patents.md)

### 4.6 地理与空间
详见 [geospatial.md](./assets/geospatial.md)

### 4.9 科技咨询与行业报告
详见 [tech-consulting.md](./assets/tech-consulting.md)

### 4.10 使用方法与脚本调用
- 选择类型后，打开 assets 中对应文件查看 GUI 操作建议与 API 接入方法。
- 若可程序化查询，优先调用 scripts 中的函数获取标准化数据，减少页面交互耗时。
- 函数入口与示例见 [data_api.py](./scripts/data_api.py)

## 5. 查询参数与权威标识符

为了提高查询的精确度，应尽可能使用标准化的权威标识符。

- **学术文献**:
    - **DOI (Digital Object Identifier)**: 如 `10.1038/nature12345`。首选标识符。
    - **PMID (PubMed ID)**: 如 `25355208`。用于生物医学文献。
    - **arXiv ID**: 如 `1706.03762`。用于预印本论文。
- **统计数据**:
    - **FRED Series ID**: 如 `GDPCA` (真实 GDP)。用于 FRED 数据。
    - **World Bank Indicator Code**: 如 `NY.GDP.MKTP.CD` (市场价现价美元 GDP)。
- **科学数据**:
    - **CAS Registry Number**: 如 `50-78-2` (阿司匹林)。用于化学物质。
    - **UniProt ID / Accession**: 如 `P04637` (人类 P53 蛋白)。
- **地理空间**:
    - **OpenStreetMap ID**: 如 `node/25494147`, `way/4322989`, `relation/118784`。用于 OSM 地图要素。

在输出结果时，应明确包含这些标识符，以便追溯和验证。

## 6. 并行与回退策略（面向 GUI）

- 并行查询：为不同子问题在浏览器中开设独立标签页并分别执行 GUI 操作，最多并行 3 条线索。
  - 示例拆分：Apple 最新 10-K（EDGAR 标签页）/ Tim Cook 的 Wikidata ID（Wikidata 标签页）/ Apple 相关专利（Lens 标签页）。
- 回退策略：当官网检索不畅或入口难以定位时，才使用站外搜索定位“官网页面”，建议使用限定语：
  - `site:sec.gov Apple 10-K`
  - `site:wikidata.org Tim Cook`
  - `site:lens.org assignee:"Apple Inc."`

## 7. 质量控制与注意事项

- **权威性优先**: 始终优先选择政府、国际组织、顶尖学术机构或知名行业开放平台发布的数据。对个人博客、论坛、或未经审核的协作性内容保持警惕。
- **时效性**: 注意数据和文献的发布日期。对于统计数据，优先使用最新版本；对于法规和专利，注意其当前状态（是否生效、过期或被取代）。
- **工具边界**:
  - 不使用图像识别类工具解析抽象表格或法规文本。
  - 涉及账户登录与验证时，通过 AskHumanToControlBrowser 请求用户协助。

## 8. 输出规范

所有查询的最终输出都应遵循以下规范：

1.  **使用清晰的中文进行总结**：直接回答用户的问题，将关键信息提炼出来。
2.  **提供可核验的来源**：在结论的末尾，附上一个“来源”或“参考资料”部分，列出所有信息来源的直接链接。
3.  **包含权威标识符**：在适当的情况下，附上相关的权威 ID，以增强答案的专业性和可追溯性。

- 文献类输出示例：
  > 根据 OpenAlex 与 Crossref 条目，论文 X 的 DOI 为 10.1234/abcdef，主要贡献为……
  > - 来源: 文章条目页
  > - DOI: `10.1234/abcdef`

- 统计类输出示例：
  > 根据世界银行数据门户，德国近十年失业率如下（单位：%），最近年份为……
  > - 来源: World Bank 指标页面
  > - 指标代码: `SL.UEM.TOTL.ZS`

- 地理类输出示例：
  > 自由女神像坐标经 OSM 验证为……，要素 ID 为……
  > - 来源: OpenStreetMap 要素页
  > - OSM ID: `node/357382898`
