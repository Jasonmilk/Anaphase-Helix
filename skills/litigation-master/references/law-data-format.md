# 法规检索数据格式

## 目录
- 输入格式（law_search_request.json）
- 输出格式（law_search_result.json）
- 使用说明
- 时效性字段说明

## 概览
本文档定义了法规检索脚本 `scripts/law_search.py` 的输入输出格式。

## 核心内容

### 输入格式（law_search_request.json）

法规检索请求使用 JSON 格式，支持多组关键词检索。

**格式定义**：
```json
[
  {
    "group_id": "1",
    "parameters": {
      "input": "检索关键词或描述"
    }
  },
  {
    "group_id": "2",
    "parameters": {
      "input": "另一组检索关键词"
    }
  }
]
```

**字段说明**：
- `group_id`: 检索组标识（字符串），用于区分不同的检索请求
- `parameters.input`: 检索关键词或描述（字符串），支持自然语言描述

**完整示例**：
```json
[
  {
    "group_id": "1",
    "parameters": {
      "title": "民法典",
      "fulltext": "租赁合同"
    }
  },
  {
    "group_id": "2",
    "parameters": {
      "title": "民法典",
      "fulltext": "婚姻"
    }
  }
]
```

### 输出格式（law_search_result.json）

检索结果返回符合条件法规的详细信息。

**格式定义**：
```json
{
  "code": "ok",
  "message": "成功",
  "data": [
    {
      "Gid": "f75f6b25c1bb285fbdfb",
      "Title": "最高人民法院关于借款合同纠纷案件管辖问题的复函",
      "DocumentNO": "[1990]法经函字第11号",
      "Category": {
        "101": "民事诉讼",
        "10103": "民事诉讼管辖",
        "017": "合同",
        "01701": "合同履行"
      },
      "SpecialType": {},
      "EffectivenessDic": {
        "XG04": "司法解释",
        "XG0402": "司法解释性质文件"
      },
      "ImplementDate": "1990.04.02",
      "IssueDate": "1990.04.02",
      "IssueDepartment": {
        "3": "最高人民法院"
      },
      "TimelinessDic": {
        "01": "现行有效"
      },
      "FullText": null,
      "Url": "[北大法宝](https://www.pkulaw.com/chl/f75f6b25c1bb285fbdfb.html?way=mcp)"
    },
    {
      "Gid": "a82343c1ce8d41c7bdfb",
      "Title": "最高人民法院发布19起合同纠纷典型案例",
      "DocumentNO": "",
      "Category": {},
      "SpecialType": {
        "010": "司法案例发布"
      },
      "EffectivenessDic": {
        "XG04": "司法解释",
        "XG0403": "两高工作文件"
      },
      "ImplementDate": "2015.12.04",
      "IssueDate": "2015.12.04",
      "IssueDepartment": {
        "3": "最高人民法院"
      },
      "TimelinessDic": {
        "01": "现行有效"
      },
      "FullText": null,
      "Url": "[北大法宝](https://www.pkulaw.com/chl/a82343c1ce8d41c7bdfb.html?way=mcp)"
    }
  ]
}
```

**关键字段说明**：

| 字段 | 说明 | 示例 |
|------|------|------|
| Gid | 法规唯一标识 | "f75f6b25c1bb285fbdfb" |
| Title | 法规标题 | "最高人民法院关于借款合同纠纷案件管辖问题的复函" |
| DocumentNO | 法规文号 | "[1990]法经函字第11号" |
| Category | 法规分类 | {"101": "民事诉讼", "10103": "民事诉讼管辖"} |
| IssueDepartment | 发布部门 | {"3": "最高人民法院"} |
| IssueDate | 发布日期 | "1990.04.02" |
| ImplementDate | 实施日期 | "1990.04.02" |
| TimelinessDic | **时效性**（关键） | {"01": "现行有效"} |
| Url | 法规原文链接 | 北大法宝链接 |

**时效性字段（TimelinessDic）说明**：
- `01`: 现行有效
- `02`: 废止或失效
- `03`: 已被修改

## 使用说明

### 调用脚本

```bash
python scripts/law_search.py --input law_search_request.json --output law_search_result.json
```

**参数说明**：
- `--input`: 输入 JSON 文件路径
- `--output`: 输出 JSON 文件路径

### 检索流程

1. **准备检索请求**：根据需要查询的法规，创建 `law_search_request.json` 文件
2. **调用脚本**：执行 `law_search.py` 脚本
3. **检查时效性**：查看返回结果的 `TimelinessDic` 字段，确认法规是否现行有效
4. **获取原文**：如需法规原文，使用 `Title` 和 `IssueDepartment` 通过公网搜索获取

### 验证规则

1. **输入文件必须包含**：
   - 至少一个检索组
   - 每个组必须有 `group_id` 和 `parameters.input`

2. **输出文件验证**：
   - 检查 `code` 字段是否为 "ok"
   - 检查 `TimelinessDic` 是否包含 "01"（现行有效）
   - 确认 `Title` 和 `IssueDepartment` 字段完整

3. **错误处理**：
   - JSON 格式错误会抛出异常
   - API 调用失败会返回错误信息
   - 网络超时为 600 秒

## 约束与注意事项

1. **时效性验证**：
   - 必须检查 `TimelinessDic` 字段
   - 仅使用 `01`（现行有效）的法规
   - `02` 和 `03` 的法规不得引用

2. **原文获取**：
   - API 不返回法规原文（`FullText` 为 null）
   - 需要通过公网搜索获取原文
   - 使用 `Title` 和 `IssueDepartment` 作为搜索关键词

3. **去重机制**：
   - 脚本自动根据 `Gid` 去重
   - 相同法规不会重复返回

4. **API 限制**：
   - 超时时间为 600 秒
   - 建议单次检索不超过 10 组关键词

## 示例

### 示例1：检索民法典租赁合同相关法规

**输入文件**（law_search_request.json）：
```json
[
  {
    "group_id": "1",
    "parameters": {
      "title": "民法典",
      "fulltext": "租赁合同"
    }
  }
]
```

**输出结果**（law_search_result.json）：
```json
[
  {
    "Gid": "f75f6b25c1bb285fbdfb",
    "Title": "中华人民共和国民法典",
    "DocumentNO": "",
    "Category": {
      "101": "民法",
      "10101": "民法典"
    },
    "IssueDepartment": {
      "1": "全国人民代表大会"
    },
    "TimelinessDic": {
      "01": "现行有效"
    },
    "Url": "[北大法宝](https://www.pkulaw.com/chl/f75f6b25c1bb285fbdfb.html?way=mcp)"
  }
]
```

### 示例2：多组检索

**输入文件**：
```json
[
  {
    "group_id": "1",
    "parameters": {
      "input": "民法典 租赁合同"
    }
  },
  {
    "group_id": "2",
    "parameters": {
      "input": "民事诉讼法 管辖"
    }
  }
]
```

**执行命令**：
```bash
python scripts/law_search.py --input law_search_request.json --output law_search_result.json
```

**结果处理**：
1. 检查返回法规的 `TimelinessDic` 字段
2. 确认是否为 "01"（现行有效）
3. 如需引用，使用 `Title` 和 `IssueDepartment` 通过公网获取原文
