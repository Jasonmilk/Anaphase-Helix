# 学术与文献

## 适用场景
- 检索论文、期刊、会议文章、预印本与引文数据
- 获取权威标识符（DOI、PMID、arXiv ID）与来源链接

## 常用方法
- API 优先：使用开放接口进行关键词、作者、DOI 精确检索
- GUI 备选：用于无法通过接口获取或需页面级筛选的场景

## GUI 操作建议
- 在官方网站的站内搜索框输入关键词或标识符
- 使用筛选器限定年份、类别、语言与主题
- 打开条目页并记录标题、作者、年份、标识符与来源链接
- 对关键信息执行交叉验证（例如在两个权威源对照）

## API 与程序化接入
- OpenAlex API 示例：/works?search=关键词
- Crossref REST 示例：/works?query=关键词
- arXiv API 示例：export API 的 query 接口
- PubMed E-Utilities 示例：esearch+efetch
- DOAJ API 示例：文章检索接口

## 权威入口与URL
- OpenAlex: https://openalex.org / API: https://api.openalex.org
- Crossref: https://www.crossref.org / API: https://api.crossref.org
- arXiv: https://arxiv.org / API: http://export.arxiv.org/api/query
- PubMed: https://pubmed.ncbi.nlm.nih.gov / E-Utilities: https://eutils.ncbi.nlm.nih.gov
- DOAJ: https://doaj.org / API: https://doaj.org/api

## 示例任务
- 查找“图神经网络在推荐系统中的综述”，返回标题、DOI 与来源链接
- 根据 DOI 获取条目详情与引文计数，并给出标识符
