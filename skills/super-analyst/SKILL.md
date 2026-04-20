---
name: super-analyst
description: Elite analytical consulting system combining deep thinking, comprehensive research, and structured analysis. Features dual Sequential Thinking integration for intelligent search planning and framework selection, web-based intelligence gathering, and adaptive complexity management. Version 2.0 (October 2025).
dependency:
  python:
    - requests>=2.28.0
---

# Super Analyst 2.0 - Elite Analytical Consulting System

## Core Capabilities

1. Deep Thinking - Sequential Thinking at two critical decision points
2. Intelligence Gathering - Web search and comprehensive research
3. Structured Analysis - 12 professional analytical frameworks

**Value Proposition:** Deep Thinking + Comprehensive Research + Structured Analysis = Consulting-Grade Insights

---

## 7-Stage Systematic Workflow

```
Stage 1: Problem Understanding & Assessment
    ↓
Stage 2: Intelligence Planning (Sequential Thinking #1) *
    ↓
Stage 3: Intelligence Gathering
    ↓
Stage 4: Framework Selection (Sequential Thinking #2) **
    ↓
Stage 5: Prompt Retrieval
    ↓
Stage 6: Structured Analysis
    ↓
Stage 7: Integrated Output
```

* Stage 2-3 are skipped if no external information is needed
** Stage 4 always executes - this is the core decision point

---

## STAGE 1: Problem Understanding

**Duration:** 10-30 seconds
**Objective:** Assess problem and determine information needs

### Quick Assessment Checklist

1. **Problem Type:** Strategic / Operational / Innovative / Diagnostic / Decision-making
2. **Complexity:** Level 1 (Simple) / Level 2 (Medium) / Level 3 (Complex)
3. **Information Needs:** Complete / Requires External Data

### Complexity Auto-Detection

**Level 1 - Simple (1-2 min total):**
- Conceptual explanations, Framework descriptions, Single-dimension problems
- Example: "What is SWOT analysis?"

**Level 2 - Medium (3-6 min total):**
- Some external info needed, 2-3 analytical dimensions, Clear problem scope
- Example: "Analyze Tesla's competitive advantages"

**Level 3 - Complex (10-20 min total):**
- Deep research required, Multi-dimensional, Strategic decisions
- Example: "Should we enter Indian market?"

### Information Need Triggers

**NEEDS EXTERNAL INFO → Proceed to Stage 2:**
- Specific companies/products/markets/industries
- Current data (prices, rankings, trends)
- Cases, best practices, competitive intel
- Recent events/developments
- Explicit research requests

**NO EXTERNAL INFO → Skip to Stage 4:**
- Pure concepts/theory
- User provided complete context
- General knowledge
- Framework explanations

### User Communication Template

```
I'll analyze this [Level X] [problem type] question.

[If research needed] I'll gather relevant intelligence first, then apply optimal analytical framework(s).

[If no research] I'll directly apply the optimal framework(s) to address your question.
```

---

## STAGE 2: Intelligence Planning

**Duration:** 1-3 minutes
**Tool:** Sequential Thinking #1
**Trigger:** Only if Stage 1 determined external information is needed

### Sequential Thinking Focus Areas

Use `sequential-thinking:sequentialthinking` to think deeply about:

1. **Information Requirements Analysis**
   - What specific info is needed? (Quantitative data, qualitative insights, background context)

2. **Information Prioritization**
   - Essential vs important vs supplementary
   - Sequential dependencies

3. **Search Keyword Strategy**
   - Chinese keywords (domestic info)
   - English keywords (international info)
   - Specific vs broad balance

4. **Search Execution Plan**
   - Number of searches (2-10 based on complexity)
   - web_search vs web_fetch
   - Chinese-English coordination

### Thinking Depth by Level

- **Level 1:** Skip this stage
- **Level 2:** 5-7 thoughts → 2-4 searches planned
- **Level 3:** 8-12 thoughts → 5-10 searches + 2-3 fetches planned

### Output to User

```markdown
## Intelligence Planning

Let me think about what information we need...

<details>
<summary>Search Plan (click to expand thinking)</summary>

[Sequential Thinking process here]

</details>

**Search Strategy:**
1. [Info Type 1] - [Language] - Keywords: [X]
2. [Info Type 2] - [Language] - Keywords: [Y]
...

Estimated: [X] searches
```

---

## STAGE 3: Intelligence Gathering

**Duration:** 2-8 minutes
**Strategy:** Balanced (4-6 searches for most problems)

### Execution Guidelines

**Search Distribution:**
- **Level 2:** 2-4 searches
- **Level 3:** 5-10 searches + 2-3 fetches
- Adjust ±1-2 based on information quality

**Chinese-English Coordination:**
- Search both languages when topic has domestic + international dimensions
- Examples:
  - "特斯拉竞争优势" + "Tesla competitive advantage"
  - "印度电商市场" + "India ecommerce market"

**Tool Selection:**
- **web_search:** Quick overviews, news, data points
- **web_fetch:** Detailed reports, in-depth articles, research

**Dynamic Adjustment:**
- Info insufficient → Add 1-2 searches
- New angle discovered → Adjust plan
- Conflicting data → Verification search

### Progress Communication

```markdown
## Intelligence Gathering

[1/6] Searching: 印度电商市场规模 2024
   Found: Market size ~$XXX billion, CAGR XX%

[2/6] Searching: amazon flipkart market share
   Found: Key players and distribution

[3/6] Fetching: [Report title]
   Retrieved: Detailed segmentation

...

Complete! Sources: [X] | Data points: [Y] | Quality: High
```

---

## STAGE 4: Framework Selection

**Duration:** 1-3 minutes
**Tool:** Sequential Thinking #2
**Always Execute:** This is THE critical decision point

### Sequential Thinking Focus Areas

Use `sequential-thinking:sequentialthinking` to think deeply about:

1. **Problem Essence Analysis**
   - Diagnostic / Strategic / Innovative / Decision / Understanding
   - Single vs multi-dimensional
   - Core challenge identification

2. **Information-Problem Matching**
   - Info completeness
   - Certainty vs uncertainty
   - Time horizon
   - Stakeholder scope

3. **Framework Suitability Evaluation**
   - Match with problem type
   - Compatibility with available info
   - Strengths for this case
   - Limitations

4. **Combination Strategy**
   - Single vs multiple frameworks
   - Sequential vs parallel application
   - Integration approach

### Thinking Depth by Level

- **Level 1:** 3-5 thoughts → 1 framework
- **Level 2:** 5-8 thoughts → 1-2 frameworks
- **Level 3:** 10-15 thoughts → 2-4 frameworks

### Output to User

```markdown
## Framework Selection

Analyzing which framework(s) will best address this...

<details>
<summary>Selection Analysis (click to expand)</summary>

[Sequential Thinking process here]

</details>

**Selected Framework(s):**

1. **[Framework]** - [Role]
   - Rationale: [Why this fits]
   - Focus: [What to emphasize]

2. **[Framework 2]** (if applicable)
   - Rationale: [Complementary value]

**Strategy:** [Sequential/Parallel] | Integration: [How]
```

---

## STAGE 5: Prompt Retrieval

**Duration:** 10-30 seconds
**Method:** Read from local `references/` directory

### Available Frameworks

All 12 frameworks stored locally as Markdown files:

| Framework | File Name |
|-----------|-----------|
| First Principles | `First Principles Thinking to analyze problems.md` |
| 5 Whys | `5 Whys method to analyze problems.md` |
| SWOT | `SWOT to analyze problems.md` |
| Porter's Five Forces | `Porter's Five Forces model to analyze problems.md` |
| Cost-Benefit | `Cost-Benefit Analysis to analyze problems.md` |
| Design Thinking | `Design Thinking to analyze problems.md` |
| Systems Thinking | `Systems Thinking to analyze problems.md` |
| Socratic Method | `Socratic Method to analyze problems.md` |
| Pareto | `Pareto Analysis to analyze problems.md` |
| MECE | `MECE Principle to analyze problems.md` |
| Hypothesis-Driven | `Hypothesis-Driven Analysis to analyze problems.md` |
| Scenario Planning | `Scenario Planning to analyze problems.md` |

**Process:** Read the corresponding `.md` file from `references/` directory. Silently retrieve - only show user if error occurs

---

## STAGE 6: Structured Analysis

**Duration:** 3-10 minutes

### Execution Principles

1. **Follow Framework Prompts Strictly**
   - Each has 6-step structure
   - Meet quantitative requirements (5-7 points/section)
   - Typical: 800-1500 words/framework

2. **Integrate Intelligence**
   - Reference Stage 3 data
   - Cite sources
   - Evidence-based reasoning

3. **Quality Standards**
   - Objective and factual
   - Clear Markdown formatting
   - Logical flow

4. **Multi-Framework Coordination**
   - Sequential: Build on prior insights
   - Parallel: Independent perspectives
   - Clear transitions

### Output Format

```markdown
## Structured Analysis

### [Framework 1 Name]

[Complete 6-step analysis]

**Key Findings:**
- [Critical insight 1]
- [Critical insight 2]

---

### [Framework 2] (if applicable)

[Complete analysis]

**Key Findings:**
- [Insights]
```

---

## STAGE 7: Integrated Output

**Duration:** 1-2 minutes

### Final Report Structure

```markdown
# Analysis Report: [Topic]

## Executive Summary (TL;DR)

[1-2 paragraphs: Direct answer + key conclusion]

---

## Detailed Analysis

[Framework analyses from Stage 6]

---

## Integrated Insights

[Cross-framework synthesis]
- Pattern connections
- Key contradictions resolved
- Deeper understanding

---

## Action Recommendations

**Immediate (0-3 months):**
- [ ] Action 1
- [ ] Action 2

**Short-term (3-6 months):**
- [ ] Action 3

**Medium-term (6-18 months):**
- [ ] Action 4

---

## Information Sources

**Intelligence:**
- [Search sources with URLs]

**Frameworks:**
- [Frameworks used]

---

Complete!
Level [X] | Sources: [X] | Frameworks: [X] | Time: ~[X] min

Questions? Let me know!
```

---

## Framework Quick Reference

### 1. First Principles Thinking
**For:** Innovation, breakthroughs, fundamental redesign
**Info Needs:** Core assumptions, constraints, historical approaches
**Complexity:** High | Time: Long

### 2. 5 Whys
**For:** Root cause diagnosis, failures, operational issues
**Info Needs:** Incident data, process docs, failure history
**Complexity:** Low | Time: Short

### 3. SWOT Analysis
**For:** Strategic assessment, business planning, positioning
**Info Needs:** Internal capabilities, market conditions, competition
**Complexity:** Medium | Time: Medium

### 4. Porter's Five Forces
**For:** Industry analysis, competitive strategy, market entry
**Info Needs:** Industry structure, competitors, power dynamics
**Complexity:** Medium-High | Time: Medium

### 5. Cost-Benefit Analysis
**For:** Investment decisions, project evaluation, resource allocation
**Info Needs:** Financial data, cost/benefit projections, risks
**Complexity:** Medium | Time: Medium

### 6. Design Thinking
**For:** User innovation, product development, service design
**Info Needs:** User research, pain points, usage patterns
**Complexity:** High | Time: Long

### 7. Systems Thinking
**For:** Complex systems, interconnections, long-term strategy
**Info Needs:** Components, relationships, feedback loops
**Complexity:** High | Time: Long

### 8. Socratic Method
**For:** Deep understanding, assumptions, philosophical inquiry
**Info Needs:** Core beliefs, definitions, logical connections
**Complexity:** Medium | Time: Medium

### 9. Pareto Analysis
**For:** Prioritization, efficiency, identifying key drivers
**Info Needs:** Frequency data, impact metrics, performance
**Complexity:** Low-Medium | Time: Short

### 10. MECE Principle
**For:** Problem decomposition, structured thinking, options
**Info Needs:** Problem scope, categories, interdependencies
**Complexity:** Medium | Time: Short-Medium

### 11. Hypothesis-Driven
**For:** Testing ideas, research, validation, uncertainty
**Info Needs:** Hypotheses, test data, evidence, criteria
**Complexity:** Medium-High | Time: Medium

### 12. Scenario Planning
**For:** Future planning, uncertainty, strategic flexibility
**Info Needs:** Uncertainties, driving forces, trends, precedents
**Complexity:** High | Time: Long

---

## Framework Combinations (Max 3-4)

**Strategy & Competition:**
- SWOT + Porter's Five Forces
- Scenario Planning + Cost-Benefit

**Problem-Solving & Innovation:**
- 5 Whys + First Principles
- Design Thinking + Systems Thinking

**Decision Support:**
- MECE + Hypothesis-Driven
- Pareto + Cost-Benefit

**Complex Analysis:**
- Systems + Scenario Planning
- First Principles + Socratic
- Porter's + SWOT + Cost-Benefit

---

## Best Practices

1. **Transparency:** Show complexity level, thinking process (collapsed), search progress
2. **Quality:** Don't rush, thorough analysis, evidence-based, meet requirements
3. **Information:** Multi-source verification, note credibility, flag conflicts
4. **UX:** Clear structure, scannable, executive summary, actionable
5. **Adaptability:** Auto-detect complexity, adjust if needed, ready for more frameworks

---

## Technical Integration

**Sequential Thinking:**
- Tool: `sequential-thinking:sequentialthinking`
- Two mandatory points: Intelligence Planning + Framework Selection
- Depth adapts to complexity
- Collapsed display by default

**Web Search:**
- Balanced: 4-6 searches for most problems
- Chinese-English coordination
- Dynamic adjustment
- web_search (overview) + web_fetch (depth)

**Framework Prompts:**
- All 12 frameworks stored locally in `references/` directory
- Read corresponding `.md` file in Stage 5
- Strict prompt adherence in Stage 6

**Framework Selector Script:**
- Optional helper: `scripts/framework_selector.py`
- Usage: `python scripts/framework_selector.py '<problem description>'`
- Returns top 3 recommended frameworks based on keyword analysis

---

## Example Triggers

**"Should we enter Indian e-commerce?"** → Level 3, comprehensive research, multi-framework

**"Why is customer churn increasing?"** → Level 2, some research, diagnostic frameworks

**"Explain first principles thinking"** → Level 1, no research, conceptual frameworks

---

## Resource Index

- **Frameworks:** 12 analytical framework prompts in `references/` directory
- **References:** 
  - `references/framework-mapping.md` - Detailed profiles
  - `references/search-strategy-guide.md` - Search best practices
  - `references/sequential-thinking-prompts.md` - Thinking templates
  - `references/usage-examples.md` - Complete scenarios
- **Scripts:** `scripts/framework_selector.py` - Framework recommendation tool

---

**Super Analyst 2.0** - Elite Analytical Consulting at Your Fingertips
