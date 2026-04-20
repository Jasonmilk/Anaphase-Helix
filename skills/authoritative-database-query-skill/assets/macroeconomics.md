# 统计与宏观经济

## 适用场景
- 国家与区域经济指标时间序列检索与下载
- 记录指标代码与单位，输出可核验来源链接

## 常用方法
- API 优先：使用官方接口获取结构化时间序列数据
- GUI 备选：需要可视化、下载或页面筛选时使用

## GUI 操作建议
- 在官方网站搜索指标名称与国家/地区
- 使用页面筛选器限定时间范围与频率
- 下载或复制最近值、单位与指标代码

## API 与程序化接入
- World Bank API：country/{code}/indicator/{indicator}
- FRED API：series/observations（需 API Key）
- OECD/IMF SDMX 接口：按数据集与维度检索

## 权威入口与URL
- World Bank Data: https://data.worldbank.org / API: http://api.worldbank.org
- FRED: https://fred.stlouisfed.org / API: https://api.stlouisfed.org
- OECD Data: https://data.oecd.org / SDMX: https://stats.oecd.org/sdmx-json
- IMF Data: https://www.imf.org/en/Data / SDMX: https://sdmx.imf.org
- UNData: https://data.un.org

## 示例任务
- 查询德国近十年失业率，返回时间序列、指标代码与来源链接
