# Orchestrator Skill 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 创建可在任意项目中使用的 orchestrator Superpowers 技能，实现主 Agent → Plan Agent → Code Agent → Review Agent 的层级编排。

**Architecture:** orchestrator 是一个元技能，不重写 agent 能力，而是定义编排调度 + 文档模板 + 知识流转。内部复用 brainstorming、writing-plans、subagent-driven-development、requesting-code-review 等现有 Superpowers 技能。配置驱动（orchestrator.yaml），阶段内 Plan/Code/Review/Archive 四步流水线，重试上限后人工介入，Code Agent 启动时注入历史 Bug 知识。

**Tech Stack:** Markdown 技能文件 + YAML 配置

---

## 文件结构

```
skills/orchestrator/                          ← 技能自身目录
  skill.md                                    ← 主入口：编排流程逻辑
  knowledge/
    inject-bugs.md                            ← Bug 知识注入 Code Agent 的 prompt 模板
    anti-patterns.md                          ← 通用编码禁止模式（跨项目共享）
  templates/
    prd.md                                    ← PRD 文档模板
    outline.md                                ← OUTLINE 任务拆分模板
    phase-plan.md                             ← 阶段计划模板
    review-report.md                          ← 审查报告模板
    bug-archive.md                            ← Bug 归档模板

项目使用方创建：
  orchestrator.yaml                           ← 项目级编排配置
  docs/prd.md                                 ← 产品需求文档
  docs/outline.md                             ← 任务拆分文档
  docs/plans/<phase>/plan.md                  ← 各阶段实现计划
  docs/bugs/<phase>/<issue>.md                ← Bug 归档
```

## 技能文件位置

技能文件放在 `~/.claude/skills/orchestrator/` 下，由 Claude Code 的 Skill 工具自动发现。技能安装脚本记录在 `orchestrator.yaml` 的 `_install` 字段中，方便重新部署。

---

## Phase 01: Scaffold — 骨架搭建

### Task 1.1: 创建技能目录结构和 orchestrator.yaml

**Files:**
- Create: `~/.claude/skills/orchestrator/skill.md`
- Create: `~/.claude/skills/orchestrator/knowledge/inject-bugs.md`
- Create: `~/.claude/skills/orchestrator/knowledge/anti-patterns.md`
- Create: `~/.claude/skills/orchestrator/templates/prd.md`
- Create: `~/.claude/skills/orchestrator/templates/outline.md`
- Create: `~/.claude/skills/orchestrator/templates/phase-plan.md`
- Create: `~/.claude/skills/orchestrator/templates/review-report.md`
- Create: `~/.claude/skills/orchestrator/templates/bug-archive.md`
- Create: `orchestrator.yaml` (在当前项目根目录)

- [ ] **Step 1: 确认目标目录不存在**

Run: `ls ~/.claude/skills/orchestrator`
Expected: "No such file or directory" — 干净创建

- [ ] **Step 2: 创建目录结构**

Run: 
```bash
mkdir -p ~/.claude/skills/orchestrator/knowledge
mkdir -p ~/.claude/skills/orchestrator/templates
```

- [ ] **Step 3: 创建 orchestrator.yaml**

**File:** `orchestrator.yaml`（在当前项目根目录）

```yaml
# orchestrator.yaml — 项目级编排配置
# 安装指引：将 skills/orchestrator/ 复制到 ~/.claude/skills/orchestrator/
version: "1.0"

defaults:
  plan_agent_count: 2
  code_agent_count: 2
  review_agent_count: 1
  max_retries: 3
  models:
    plan: opus
    code: sonnet
    review: haiku

phases:
  - name: "01-scaffold"
    description: "骨架搭建"

  - name: "02-core"
    description: "核心实现"

  - name: "03-polish"
    description: "打磨优化"

parallelism:
  max_concurrent_phases: 2
  auto_detect: true

archive:
  enabled: true
  dir: "docs/bugs"
  auto_archive: true
  inject_for_code: true
```

- [ ] **Step 4: 创建 skill.md 骨架**

**File:** `~/.claude/skills/orchestrator/skill.md`

```markdown
---
name: orchestrator
description: 多 Agent 层级编排系统 - 主 Agent 协调 Plan/Code/Review/Archive Agent 完成需求开发
---

# Orchestrator Skill

> 主 Agent 编排器。加载此技能后，主 Agent 按 orchestrator.yaml 配置驱动完整开发流程。

## 前提

1. 项目根目录存在 `orchestrator.yaml`
2. 不存在时引导用户：调用子技能 `init` 或手动创建最小配置
3. 已安装的子技能：brainstorming、writing-plans、subagent-driven-development、requesting-code-review

## 流程概览

本技能将主 Agent 转变为编排器，按以下步骤执行：

### Phase 0: 需求与文档

1. **需求收集** — 复用 brainstorming 技能，与用户对话明确需求
2. **产出 PRD** — 写入 `docs/prd.md`，用 templates/prd.md 模板，用户确认
3. **拆分 OUTLINE** — 写入 `docs/outline.md`，用 templates/outline.md 模板，标注 @depends 依赖

### Phase 1..N: 阶段循环

按 orchestrator.yaml 定义的 phases 列表循环。每阶段内部：

1. **Plan** — 调用 writing-plans 技能，输入 PRD + OUTLINE 对应章节，输出 `docs/plans/<phase>/plan.md`
2. **Code** — 调用 subagent-driven-development 技能，注入 knowledge/anti-patterns.md + 匹配的 Bug 归档
3. **Review** — 调用 requesting-code-review 技能，审阅代码 diff + plan
4. **Archive** — 若为 Bug 修复任务，写入 `docs/bugs/<phase>/<issue>.md`

### 重试与人工介入

- Review 不通过 → 退回 Code 步骤，最多 max_retries 轮
- 超限 → 暂停并列出 review report，等待用户决策（继续重试 / 跳过 / 人工修改）
- Ctrl-C → 保存进度到 `docs/.orchestrator-progress`，下次加载可恢复

### 完成报告

所有阶段通过后汇总：阶段数、文件变更数、归档数、审查通过率。

## 配置文件加载

首次加载时自动执行：
1. 读取 `orchestrator.yaml`
2. 扫描 `docs/bugs/` 构建归档索引（供 Code Agent 注入）
3. 解析 OUTLINE 中的 @depends 标签构建依赖图（如有 OUTLINE）

## 任务粒度参考

对于复杂任务，主 Agent 可启动 Plan Agent 进一步拆分。输出的 plan.md 应符合 writing-plans 技能要求的粒度（每个步骤 2-5 分钟，单文件变更）。
```

- [ ] **Step 5: 创建 knowledge/anti-patterns.md**

**File:** `~/.claude/skills/orchestrator/knowledge/anti-patterns.md`

```markdown
# 通用编码禁止模式

以下模式是 Code Agent 在任何项目中都应避免的，注入到每个 Code Agent 的 prompt 中。

## 并发安全

- 禁止在持有 Mutex 锁的作用域内执行 .await 或任何异步操作，这会导致死锁
- 禁止在异步上下文中使用 std::thread::sleep — 使用 tokio::time::sleep 替代
- 禁止无限制的递归调用 — 确保存在明确的终止条件

## 错误处理

- 禁止吞掉错误 — 每个 Result 必须被处理或显式传播（? 运算符）
- 禁止使用 unwrap() / expect() 在库代码中 — 仅在 main() / test 中允许
- 禁止将内部错误细节直接暴露给外部 API 调用者 — 做错误边界转换

## 安全

- 禁止在代码中硬编码密钥/令牌 — 通过环境变量或配置文件注入
- 禁止 SQL 字符串拼接 — 使用参数化查询
- 禁止对外部输入做未校验的路径拼接 — 做路径规范化检查

## 代码组织

- 禁止在单个文件中塞入超过 600 行代码 — 模块化拆分
- 禁止在接口定义已经确定后随意修改公共 API 签名 — 向后兼容优先
- 禁止在非测试代码中引入未使用的依赖
```

- [ ] **Step 6: 创建 knowledge/inject-bugs.md**

**File:** `~/.claude/skills/orchestrator/knowledge/inject-bugs.md`

```markdown
# Bug 知识注入模板

此模板在 Code Agent 启动时使用。主 Agent 应在启动 Code Agent 前完成两件事：

1. 根据任务关键词扫描 `docs/bugs/<phase>/` 中的所有归档
2. 提取匹配的归档摘要，填入下方模板的 `{BUG_LIST}` 占位符

## 注入 Prompt

```
## 历史 Bug 参考（请避免重复以下错误）

以下是当前项目之前曾出现过的 Bug，涉及 {MATCHED_KEYWORDS} 相关代码。
请特别注意这些禁止模式，避免产生相同的错误。

{for each matching bug archive:}

### Bug: {issue_id}
- 涉及文件：{files}
- 严重程度：{severity}
- 根本原因：{root_cause_summary}
- 禁止模式：{anti_pattern}
```

## 匹配逻辑

- 关键词匹配：使用任务描述文本与归档 frontmatter 中的 keywords 字段做交集
- 文件路径匹配：使用任务涉及的文件与归档 frontmatter 中的 files 字段做交集
- 全部匹配：若任务描述包含 "bug fix" / "fix" / "修复" 等关键词，注入所有归档
- 无匹配时：跳过注入，不插入任何内容
```

- [ ] **Step 7: 创建 6 个模板文件的占位内容**

**File:** `~/.claude/skills/orchestrator/templates/prd.md`

```markdown
# PRD: {项目名称}

## 背景
{为什么做这个，解决了什么问题}

## 目标
{可衡量的成功标准}

## 功能列表
1. {功能名称} — {简要描述}
2. {功能名称} — {简要描述}

## 非功能需求
- 性能：
- 安全性：
- 可维护性：

## 技术栈
{语言、框架、数据库等}

## 验收标准
1. {具体可验证的标准}
2. {具体可验证的标准}
```

**File:** `~/.claude/skills/orchestrator/templates/outline.md`

```markdown
# OUTLINE: {项目名称}

## 阶段 01-{name}

### 任务 1.1: {任务名} <!-- @depends: none @code_agents:2 -->
{任务描述，包含输入/输出}

### 任务 1.2: {任务名} <!-- @depends: 1.1 -->
{任务描述}

## 阶段 02-{name}

### 任务 2.1: {任务名} <!-- @depends: 1.2 -->
{任务描述}

### 任务 2.2: {任务名} <!-- @depends: none -->
{与 Phase 01 无依赖，可并行}
```

**File:** `~/.claude/skills/orchestrator/templates/phase-plan.md`

```markdown
# Phase {name} 实现计划

> **Agent:** Plan Agent 产出
> **输入:** PRD.md + OUTLINE.md 对应章节

## 涉及文件
- {文件路径}: {改动说明}
- {文件路径}: {改动说明}

## 实现步骤

### Step 1: {步骤名}
{具体操作 + 代码/命令}

### Step 2: {步骤名}
{具体操作 + 代码/命令}

## 测试策略
{如何验证}

## 风险点
{已知风险}
```

**File:** `~/.claude/skills/orchestrator/templates/review-report.md`

```markdown
# Review Report: Phase {name}

**审查结果:** ✅ 通过 / ❌ 不通过
**审查轮次:** {current}/{max}

## 问题列表

### {severity}: {问题简述}
- 位置：{文件}:{行号}
- 说明：{详细描述}
- 建议：{修复建议}

## 总结
{通过/不通过的总体评价}
```

**File:** `~/.claude/skills/orchestrator/templates/bug-archive.md`

```markdown
---
phase: {phase_name}
issue_id: {kebab-case-id}
keywords: [{tag1}, {tag2}]
files: [{file_path}]
severity: {low|medium|high}
---

## 错误现象
{触发条件和错误输出}

## 根本原因
{根因分析}

## 修复方案
{最终采用的修复代码变更}

## 禁止模式
{Code Agent 必须避免的模式，用 MUST NOT 语言描述}
```

- [ ] **Step 8: 提交 Phase 01**

```bash
git add orchestrator.yaml
git add docs/superpowers/specs/2026-05-09-orchestrator-skill-design.md
git add docs/superpowers/plans/2026-05-09-orchestrator-implementation.md
git commit -m "feat: add orchestrator skill scaffold and design docs"
```

---

## Phase 02: Core — 完整编排流程

### Task 2.1: 完善 skill.md — Phase 0 需求收集与文档产出

**Files:**
- Modify: `~/.claude/skills/orchestrator/skill.md`

在 skill.md 的 "Phase 0" 部分展开以下详细流程：

- [ ] **Step 1: 需求收集阶段流程**

追加到 skill.md Phase 0 需求收集部分：

```markdown
### Phase 0 执行细节

#### 步骤 1: 需求收集

1. 加载 brainstorming 技能
2. 按 brainstorming 的 Checklist 逐项执行：
   - 探索项目上下文（当前项目结构、现有文件、近期提交）
   - 问澄清问题（一次一个）
   - 提出 2-3 种方案（含推荐）
   - 逐节呈现设计并获得用户确认
3. 产出：设计文档写入 `docs/superpowers/specs/<date>-<topic>-design.md`

#### 步骤 2: 产出 PRD

1. 基于设计文档和用户确认的需求，用 templates/prd.md 模板编写 `docs/prd.md`
2. 每个板块必须填写完整，下划线占位符替换为实际内容
3. 展示给用户确认
4. 用户不同意则修改后重新展示，直到确认

#### 步骤 3: 拆分 OUTLINE

1. 基于 PRD 功能列表和非功能需求，拆分阶段和任务
2. 每个任务标注 @depends 依赖标签、@code_agents 数量、@max_retries 覆盖
3. 保证每个阶段有明确的交付物和验收标准
4. 写入 `docs/outline.md`，用 templates/outline.md 模板
5. 展示给用户确认
```

- [ ] **Step 2: 依赖图构建与并行调度**

追加到 skill.md 的 "Phase 1..N: 阶段循环" 之前：

```markdown
### 依赖图构建

1. 读取 `docs/outline.md`
2. 正则提取每个任务的 @depends 标签：
   - `@depends: none` → 无依赖，可最早启动
   - `@depends: 1.1, 2.3` → 依赖任务 1.1 和 2.3 完成后才可启动
3. 构建有向无环图（DAG）：节点 = 任务，边 = 依赖关系
4. 按阶段聚合：一个阶段的所有任务依赖就绪时，该阶段可启动
5. 阶段级并行度受 `orchestrator.yaml > parallelism > max_concurrent_phases` 限制

#### 执行顺序

1. 从 DAG 中找出所有入度为 0 的阶段（无依赖）
2. 从中选取不超过 max_concurrent_phases 个阶段启动
3. 阶段完成后，重新计算剩余阶段的入度
4. 重复直到所有阶段完成
```

- [ ] **Step 3: 阶段循环内部四步流水线**

替换 skill.md 中 Phase 1..N 的概要描述为：

```markdown
### 阶段循环执行细则

对每个就绪阶段执行以下四步：

#### 4a. Plan 步骤

1. 调用 writing-plans 技能
2. 输入：`docs/prd.md` 全文 + `docs/outline.md` 对应章节
3. Plan Agent(subagent_type=Plan, model=opus) 产出 plan.md
4. Plan Agent 输出写入 `docs/plans/<phase>/plan.md`，使用 templates/phase-plan.md 模板
5. 步骤等待 Plan Agent 完成（串行，plan 完成前不进 code）

#### 4b. Code 步骤

1. 收集 Bug 归档注入：
   2. 扫描 `docs/bugs/<phase>/` 下所有归档
   3. 提取归档 matching keywords 与任务描述的交集
   4. 注入 `inject-bugs.md` 模板（无匹配时跳过）
2. 调用 subagent-driven-development 技能
3. Code Agent(subagent_type=general-purpose, model=sonnet) 启动
4. 注入 anti-patterns.md 作为 prompt 前缀
5. 注入匹配的 Bug 归档摘要作为 prompt 第二部分
6. 输入：`docs/plans/<phase>/plan.md`
7. 多 Code Agent 可以并行（数量由配置决定），每个负责 plan.md 中的不同任务
8. 产出：代码文件变更

#### 4c. Review 步骤

1. 收集 Code 步骤产生的所有 git diff
2. 调用 requesting-code-review 技能
3. Review Agent(subagent_type=general-purpose, model=haiku) 启动
4. 输入：git diff + `docs/plans/<phase>/plan.md`
5. 产出：review report，用 templates/review-report.md 模板
6. 流转判定：
   - 通过 → 进入 4d
   - 不通过 + 已重试次数 < max_retries → 退回 4b
   - 不通过 + 已重试次数 = max_retries → 暂停，列出 review report，等待用户决策
7. 同一 Code Agent 的修复行为：收到重试通知后，根据 review report 中的问题列表逐一修复
8. 修复后再次进入 Review 步骤，review report 记录轮次号

#### 4d. Archive 步骤

1. 判断当前阶段是否包含 Bug 修复任务（检查 plan.md 中是否有 "fix"/"bug"/"修复" 关键词）
2. 如果是 Bug 修复：
   3. 启动 Archive Agent(subagent_type=general-purpose)
   4. 输入：git diff + review report + plan.md
   5. 产出：`docs/bugs/<phase>/<issue-id>.md`，使用 templates/bug-archive.md 模板
   6. 写入 frontmatter：phase、issue_id、keywords、files、severity
   7. 写入正文：错误现象、根本原因、修复方案、禁止模式
3. 如果不是 Bug 修复，跳过此步骤
```

- [ ] **Step 4: 完成报告**

追加到 skill.md 末尾前：

```markdown
### 完成报告

所有阶段通过后，主 Agent 汇总以下信息：

1. 阶段统计：总阶段数、通过数、重试总数
2. 文件统计：创建文件数、修改文件数
3. 归档统计：总 Bug 归档数（提供关键词分布）
4. 审查统计：总审查轮次、一次通过率（首轮通过数 / 总阶段数）

展示格式：

```
===== Orchestrator 完成报告 =====
阶段: 3/3 通过
重试: 共 0 次
文件: +12 创建, +8 修改
归档: 2 个 Bug 归档
审查: 5 轮, 一次通过率 67%
==============================
```

报告展示后结束技能执行。
```

- [ ] **Step 5: 提交 Phase 02**

```bash
git add ~/.claude/skills/orchestrator/skill.md
git commit -m "feat: implement orchestrator full orchestration flow"
```

---

## Phase 03: Polish — 打磨与验证

### Task 3.1: 并行调度实现

**Files:**
- Modify: `~/.claude/skills/orchestrator/skill.md`

在依赖图构建部分之后，追加并行调度逻辑的详细描述：

- [ ] **Step 1: 在依赖图构建后追加并行调度伪代码**

追加到 skill.md 中依赖图构建部分之后：

```markdown
#### 并行调度算法

```
phase_queue = 所有阶段
ready_phases = get_ready_phases(phase_queue)  // 入度为 0 的阶段
running_phases = []
completed_phases = []
max_concurrent = orchestrator.yaml.parallelism.max_concurrent_phases

while len(completed_phases) < len(total_phases):
    // 填充运行槽
    while len(running_phases) < max_concurrent and len(ready_phases) > 0:
        phase = ready_phases.pop(0)
        running_phases.append(phase)
        // 异步启动阶段流水线
        start_phase(phase)
    
    // 等待任一阶段完成（非阻塞轮询）
    finished = wait_any(running_phases)
    running_phases.remove(finished)
    completed_phases.append(finished)
    
    // 更新依赖图
    mark_completed(finished)
    new_ready = get_ready_phases(phase_queue)
    ready_phases.extend(new_ready)
```

注意：由于主 Agent 在同一线程执行，实际"并行"指：
- 阶段间并行 = 串行启动各阶段，在等待 Plan Agent 返回时切到另一个就绪阶段
- 阶段内并行 = 多个 Code Agent 同时启动（通过 subagent-driven-development 并行子任务）
```

- [ ] **Step 2: 断点恢复**

在 skill.md 开头追加进度保存逻辑：

```markdown
### 进度保存与恢复

每次阶段完成或用户中断时，写入 `docs/.orchestrator-progress`：

```yaml
# 自动生成，勿手动修改
version: "1.0"
timestamp: "2026-05-09T10:30:00Z"
completed_phases:
  - name: "01-scaffold"
    status: passed
    files_created: 9
    review_rounds: 1
running_phases: []
remaining_phases:
  - "02-core"
  - "03-polish"
```

主 Agent 启动时自动检测：
- 若 `docs/.orchestrator-progress` 存在 → 询问用户是否恢复
- 是 → 从断点继续，跳过已完成阶段
- 否 → 删除进度文件从头开始
```

- [ ] **Step 3: 错误处理边界**

在 skill.md 的人共介入部分补充：

```markdown
#### 错误场景处理

| 场景 | 处理方式 |
|------|---------|
| orchestrator.yaml 不存在 | 引导用户创建最小配置，提供模板 |
| PRD 尚未创建 | 自动进入 Phase 0 需求收集 |
| Plan Agent 返回空 plan | 重试一次，仍为空则暂停人工介入 |
| Code Agent 产生编译错误 | Review Agent 应捕获，退回修复。若连续 3 次编译失败，暂停人工介入 |
| OUTLINE 依赖循环检测 | 解析 @depends 时检测循环，提示用户修复 |
| 归档注入无匹配 | 静默跳过，不中断流程 |
```

- [ ] **Step 4: 用 Redis 项目 dogfood 验证**

Files: 无新建，使用 project 中的现有文件

- [ ] **Step 4.1: 在当前 Redis 项目根目录准备 orchestrator.yaml**

内容同 Task 1.1 Step 3 的 orchestrator.yaml，但修改 phases 为一个简单阶段测试：
```yaml
phases:
  - name: "01-test-hash"
    description: "实现 Redis HASH 基础命令"

parallelism:
  max_concurrent_phases: 1
  auto_detect: false
```

- [ ] **Step 4.2: 手动创建简化 PRD 和 OUTLINE 用于验证**

**File:** `docs/prd.md`（临时验证用）

```markdown
# PRD: Redis HASH 命令实现

## 功能列表
1. HSET — 设置 hash 字段
2. HGET — 获取 hash 字段
3. HGETALL — 获取所有字段

## 验收标准
1. HSET 返回 1（新建）或 0（覆盖）
2. HGET 返回字段值或 nil
3. HGETALL 返回所有 field-value 对
```

**File:** `docs/outline.md`

```markdown
# OUTLINE: Redis HASH 命令

## 阶段 01-test-hash

### 任务 1.1: HSET/HGET 实现 <!-- @depends: none @code_agents:1 -->
实现 HSET 和 HGET 命令的解析、存储和返回。

### 任务 1.2: HGETALL 实现 <!-- @depends: 1.1 -->
实现 HGETALL 命令。
```

- [ ] **Step 4.3: 按照编排流程执行验证**

1. 加载 orchestrator 技能
2. 验证能否正确读取 orchestrator.yaml
3. 验证能否进入 Phase 0 → 跳过（PRD/OUTLINE 已存在）
4. 验证能否进入 Phase 1 并启动 Plan Agent（读取 PRD + OUTLINE）
5. 验证 Plan Agent 能产出 plan.md 到 `docs/plans/01-test-hash/plan.md`
6. 验证 Code Agent 能读取 plan.md 并注入 anti-patterns + Bug 归档
7. 验证 Code Agent 能实现 HASH 命令代码变更
8. 验证 Review Agent 能产出 review report
9. 验证完成后归档备份并清理临时文件

- [ ] **Step 5: 提交 Phase 03**

```bash
git add ~/.claude/skills/orchestrator/
git add docs/prd.md docs/outline.md
git add orchestrator.yaml
git commit -m "feat: orchestrator polish - parallel, recovery, error handling"
```

---

## 自检检查

**1. Spec 覆盖度：**
- 整体架构设计 → Task 1.1 (skill.md 骨架) + Task 2.1-2.3 (完整编排)
- orchestrator.yaml schema → Task 1.1 Step 3
- 文档体系 (PRD/OUTLINE/plan/归档) → Task 1.1 Steps 5-7 (模板) + Task 2.1 (产出流程)
- 四步流水线 → Task 2.3 Step 3
- Code Agent 知识注入 → Task 1.1 Steps 5-6 (知识文件) + Task 2.3 Step 3 (注入逻辑)
- 并行调度 → Task 3.1 Step 1
- 断点恢复 → Task 3.1 Step 2
- 人工介入 → Task 2.3 Step 3 (重试判定) + Task 3.1 Step 3 (错误处理)
- 完成报告 → Task 2.4
- 验证 → Task 3.1 Step 4

**2. 占位符扫描：** 无 TBD/TODO 占位符，所有模板内容已填充。

**3. 类型一致性：** orchestrator.yaml 字段名在各阶段保持一致。模型分配（plan: opus, code: sonnet, review: haiku）始终一致。
