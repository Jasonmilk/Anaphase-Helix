# 黄金市场信息源白名单

## 使用指南

### 工具选择策略

**WebFetch** - 用于官方数据源:

- 美联储官网
- 各国央行官网
- 世界黄金协会
- IMF/世界银行
- 交易所官方数据

**WebSearch** - 用于新闻事件:

- 重大市场事件
- 官员讲话报道
- 宏观数据发布
- 地缘政治事件

### 查询示例

**WebFetch示例**:
```
url: https://www.federalreserve.gov/newsevents.htm
prompt: "Extract latest FOMC meeting decisions and Federal Reserve policy statements from the past 7 days"
```

**WebSearch示例**:
```
query: "Federal Reserve interest rate decision January 2026"
query: "gold ETF holdings change this week"
query: "central bank gold purchases 2026"
```

---

## 因子1: 实际利率信息源

### 官方来源 (WebFetch优先)

**美联储 (Federal Reserve)**

- 官网: https://www.federalreserve.gov/
- FOMC声明: https://www.federalreserve.gov/monetarypolicy/fomccalendars.htm
- 会议纪要: https://www.federalreserve.gov/monetarypolicy/fomcminutes.htm
- 官员讲话: https://www.federalreserve.gov/newsevents/speeches.htm
- 经济预测(点阵图): Summary of Economic Projections
- 重要信息公布日历：https://www.federalreserve.gov/newsevents/calendar.htm

**关键数据**:
- 联邦基金利率目标
- 点阵图(利率预期)
- 通胀预测(PCE)
- 失业率预测
- GDP增长预测
- 联邦公开市场委员会新闻发布会（重要日程提醒）

**圣路易斯联邦储备银行 (FRED)**

- 10年期TIPS收益率: https://fred.stlouisfed.org/series/DFII10
- 10年期国债名义收益率: https://fred.stlouisfed.org/series/DGS10
- 实际利率计算: 名义收益率 - TIPS收益率
- CPI通胀数据: https://fred.stlouisfed.org/series/CPIAUCSL

**美国财政部 (US Treasury)**

- 官网: https://home.treasury.gov/
- 国债收益率: https://home.treasury.gov/resource-center/data-chart-center/interest-rates
- 10年期美债收益率

**美国劳工统计局 (BLS)**

- 官网: https://www.bls.gov/
- CPI数据: https://www.bls.gov/cpi/
- PCE价格指数数据
- 就业数据: https://www.bls.gov/news.release/empsit.toc.htm

### 权威媒体报道 (WebSearch)

**一级来源**:

- Reuters (路透社)
- Bloomberg (彭博)
- Financial Times (金融时报)
- Wall Street Journal (华尔街日报)

**搜索关键词**:
- "Federal Reserve interest rate decision"
- "FOMC meeting statement"
- "Fed Chair Powell speech"
- "US inflation CPI data"
- "Treasury yields real rates"
- "10-year TIPS yield real rates"
- "Real interest rates gold correlation"

---

## 因子2: 央行购金信息源

### 官方来源 (WebFetch优先)

**世界黄金协会 (World Gold Council)**
- 官网: https://www.gold.org/
- 季度需求报告: Gold Demand Trends
- 央行购金数据: Central Bank Gold Reserves
- 黄金需求趋势报告: https://www.gold.org/goldhub/data/gold-demand-trends

**国际货币基金组织 (IMF)**
- 官网: https://www.imf.org/
- 官方黄金储备: International Financial Statistics
- 数据: https://data.imf.org/

**中国人民银行 (PBOC)**
- 官网: http://www.pbc.gov.cn/
- 黄金储备公告(月度)
- 外汇储备数据
- 资产负债表

**俄罗斯央行 (Bank of Russia)**
- 官网: https://www.cbr.ru/eng/
- 黄金储备数据

**其他主要央行**:
- 土耳其央行: https://www.tcmb.gov.tr/wps/wcm/connect/en/tcmb+en
- 印度储备银行: https://www.rbi.org.in/
- 波兰央行: https://www.nbp.pl/homen.aspx
- 哈萨克斯坦央行: https://nationalbank.kz/
- 乌兹别克斯坦央行: https://cbu.uz/

### 权威媒体报道 (WebSearch)

**搜索关键词**:
- "central bank gold purchases 2026"
- "China gold reserves PBOC"
- "World Gold Council demand report"
- "IMF gold reserves data"
- "emerging markets central banks gold"
- "Russia gold reserves accumulation"

**一级来源**:
- Reuters metals markets
- Bloomberg commodities
- Financial Times gold markets

---

## 因子3: 系统性风险信息源

### 官方数据 (WebFetch优先)

**芝加哥期权交易所 (CBOE) - VIX恐慌指数**

- VIX指数: https://www.cboe.com/tradable_products/vix_index/vix-white-paper/
- FRED数据: https://fred.stlouisfed.org/series/VIXCLS
- VIX白皮书: https://www.cboe.com/tradable_products/vix_index/vix-white-paper/

**洲际交易所 (ICE) - 美元指数**

- 美元指数DXY: https://www.theice.com/marketdata/reports/ReportCenter.shtml?ReportId=99
- 实时数据: https://www.theice.com/products/27996643/US-Dollar-Index-Futures

**上海黄金交易所 (SGE)**

- 官网: https://www.sge.com.cn/
- 实时行情: https://www.sge.com.cn/sjzx/ywzx/
- 人民币黄金价格
- 黄金现货合约数据
- 库存数据

**芝加哥商品交易所 (CME)**

- COMEX黄金期货: https://www.cmegroup.com/markets/metals/precious/gold.html
- 黄金持仓数据: https://www.cmegroup.com/markets/metals/precious/gold.html

**美国商品期货交易委员会 (CFTC)**

- 官网: https://www.cftc.gov/
- 持仓报告(COT): Commitments of Traders
- 发布时间: 每周五

### 权威媒体报道 (WebSearch)

**市场情绪指标**:

- **VIX恐慌指数**:
  - 搜索: "VIX index today volatility"
  - 搜索: "market fear index VIX spike"
  - 搜索: "CBOE Volatility Index real-time"

- **美元指数**:
  - 搜索: "US Dollar Index DXY today"
  - 搜索: "Dollar strength currency markets"
  - 搜索: "USD weakness gold support"

- **中国黄金市场**:
  - 搜索: "Shanghai Gold Exchange gold price"
  - 搜索: "SGE gold trading volume"
  - 搜索: "China gold demand retail"

**地缘政治风险**:
- 搜索: "geopolitical risk escalation gold"
- 搜索: "war conflict gold safe haven"
- 搜索: "trade war impact gold"

**金融系统性风险**:
- 搜索: "banking system crisis gold"
- 搜索: "financial stress indicators"
- 搜索: "credit market stress gold"

**权威媒体**:
- Reuters markets
- Bloomberg markets
- Financial Times markets
- Wall Street Journal markets

---

## 因子4: 资金流信息源

### ETF持仓数据 (WebFetch优先)

**SPDR Gold Shares (GLD)**
- 官网: https://www.spdrgoldshares.com/
- 每日持仓量
- 资金流向

**iShares Gold Trust (IAU)**
- 官网: https://www.ishares.com/us/products/239561/ishares-gold-trust-fund
- 持仓数据

**其他黄金ETF**:
- GDX (金矿股ETF)
- GDXJ (初级金矿股ETF)

### 期货持仓 (WebFetch优先)

**美国商品期货交易委员会 (CFTC)**
- 官网: https://www.cftc.gov/
- 持仓报告(COT): Commitments of Traders
- 发布时间: 每周五

**芝加哥商品交易所 (CME)**
- 官网: https://www.cmegroup.com/
- COMEX黄金期货数据
- 持仓量、成交量

**上海期货交易所 (SHFE)**

- 官网: http://www.shfe.com.cn/
- 黄金期货数据
- 持仓量变化

---

## 实时价格数据

### 国际现货黄金

- **伦敦金银市场协会 (LBMA)**: https://www.lbma.org.uk/prices-and-data/precious-metals-prices/gold
- **黄金现货价格 (XAU/USD)**: 实时查询通过WebSearch

### 交易所官方数据

**纽约商品交易所 (COMEX)**
- 官网: https://www.cmegroup.com/markets/metals/precious/gold.html
- 实时期货价格
- 持仓数据

**上海黄金交易所 (SGE)**
- 官网: https://www.sge.com.cn/
- 人民币金价 (AU99.99, AU99.95)
- 实时行情

**东京工业品交易所 (TOCOM)**
- 官网: https://www.tocom.or.jp/
- 黄金期货数据

---

## 宏观经济数据源

### 美国经济数据

**关键指标**:
- GDP增长率 (BEA)
- 失业率 (BLS)
- CPI/PCE通胀 (BLS/BEA)
- 零售销售 (Commerce Department)
- PMI制造业指数 (ISM)
- 房屋数据

**发布日历**:
- 美联储经济日历: https://www.federalreserve.gov/newsevents/calendar.htm
- Trading Economics: https://tradingeconomics.com/united-states/calendar

### 全球经济数据

**国际组织**:
- IMF世界经济展望: https://www.imf.org/en/Publications/WEO
- OECD经济展望: https://www.oecd.org/economic-outlook/
- 世界银行全球经济监测: https://www.worldbank.org/en/research/blogs/all-hands

**中国数据**:
- 国家统计局: http://www.stats.gov.cn/
- 中国人民银行: http://www.pbc.gov.cn/

---

## 地缘政治与事件驱动

### 官方来源

**主要国家政府网站**:
- 白宫: https://www.whitehouse.gov/
- 国务院: https://www.state.gov/
- 欧盟委员会: https://ec.europa.eu/

**国际组织**:
- 联合国: https://www.un.org/
- WTO: https://www.wto.org/

### 权威媒体 (WebSearch优先)

**地缘政治事件**:
- 搜索: "war conflict escalation"
- 搜索: "sanctions impact gold"
- 搜索: "trade war tensions"
- 搜索: "geopolitical risk premium"

**重大事件**:
- 搜索: "election impact gold market"
- 搜索: "Brexit gold safe haven"
- 搜索: "Middle East tension gold"

---

## 数据更新频率与查询时机

### 实时数据 (WebSearch)
- 黄金现货价格
- VIX指数
- 美元指数
- 市场情绪指标

### 日更新数据 (WebFetch)
- 黄金ETF持仓
- 黄金期货持仓
- TIPS收益率
- 10年期国债收益率

### 周更新数据 (WebFetch)
- CFTC持仓报告 (每周五)
- 央行黄金储备更新
- 世界黄金协会数据

### 月度数据 (WebFetch)
- 各国央行黄金储备公告
- 世界黄金协会季度报告
- IMF黄金储备数据
- 中国人民银行外汇储备

---

## 合规性提醒

### 信息源使用原则

1. **仅使用公开权威数据源**
   - 官方机构网站
   - 权威媒体报道
   - 官方交易所数据

2. **禁止使用来源**
   - 未经验证的社交媒体
   - 论坛和社区帖子
   - 匿名消息来源

3. **信息核实**
   - 交叉验证多个来源
   - 标注数据来源
   - 说明数据时效性

4. **免责声明**
   - 历史数据不代表未来表现
   - 仅供参考，不构成投资建议
   - 明确标注报告生成日期

---

## 查询优化技巧

### 高效查询关键词

**利率因素**:
- "Federal Reserve FOMC decision"
- "real interest rates gold"
- "TIPS yield 10-year"
- "inflation expectations CPI"

**央行购金**:
- "central bank gold reserves"
- "World Gold Council demand"
- "PBOC gold reserves"
- "IMF gold statistics"

**系统性风险**:
- "VIX index volatility"
- "US Dollar Index DXY"
- "market fear index"
- "geopolitical risk gold"

### 时间范围限定

- "past 7 days" - 最新事件
- "this month" - 月度趋势
- "year to date" - 年度累计
- "historical comparison" - 历史对比

### 多源交叉验证

1. 先查官方数据源 (WebFetch)
2. 再查权威媒体报道 (WebSearch)
3. 交叉验证关键数据点
4. 标注数据来源和时效性
