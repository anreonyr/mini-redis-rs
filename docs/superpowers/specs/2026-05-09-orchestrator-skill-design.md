# Orchestrator Skill — 多 Agent 层级编排系统

## 概述

Orchestrator 是 Superpowers 技能体系的一个扩展技能，实现"主 Agent → 计划 Agent → 代码 Agent → 审查 Agent"的层级式协作编排。主 Agent 作为编排器，按声明式配置和文档驱动的工作流，自动协调各阶段和 Agent 类型。

## 整体架构

Orchestrator 是一个主 Agent，按三个阶段推进工作。

Phase 0 分为两步：需求收集 → PRD + OUTLINE 文档产出。这两步串行执行，由主 Agent 直接操控，生成项目所需的完整规格文档。

Phase 1..N 是循环执行阶段。在进入每个阶段前，主 Agent 先根据 OUTLINE 中的 @depends 标签构建依赖图，计算出哪些阶段可以并行，哪些必须串行。每个阶段内部是一个四步流水线：Plan Agent 制定计划、Code Agent 实现代码、Review Agent 审查质量、Archive Agent（可选）归档 Bug 知识。Review 不通过时循环回到 Code Agent 重试，超限后进入人工介入。Code Agent 启动时自动注入 docs/bugs 中匹配的历史 Bug 归档，避免重复错误。

### Agent 类型

| Agent 类型 | subagent_type | 权限 | 用途 |
|-----------|--------------|------|------|
| Plan Agent | `Plan` | 只读 + 写入 plan.md | 基于 PRD/OUTLINE 制定实现计划 |
| Code Agent | `general-purpose` | 读写项目源码 | 实现代码 |
| Review Agent | `general-purpose` | 只读代码 diff + plan | 审查代码质量 |
| Archive Agent | `general-purpose` | 写入归档 | 将 Bug 根本原因归档到知识库 |

### 模型分配

| Agent 类型 | 模型 | 理由 |
|-----------|------|------|
| Plan Agent | opus | 计划需要深度推理和全面考虑 |
| Code Agent | sonnet | 代码实现速度快 |
| Review Agent | haiku | 审查轻量级，成本低 |

## 配置文件 (`orchestrator.yaml`)

放在项目根目录，声明式定义编排配置：

```yaml
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
  - name: "01-core"
    description: "核心功能"

  - name: "02-advanced"
    description: "高级特性"
    plan_agent_count: 1
    code_agent_count: 1

  - name: "03-polish"
    description: "打磨优化"
    code_agent_count: 1

parallelism:
  max_concurrent_phases: 2
  auto_detect: true

archive:
  enabled: true
  dir: "docs/bugs"
  auto_archive: true
  inject_for_code: true
```

## 文档体系

### PRD (`docs/prd.md`)
产品需求文档，包含：背景、目标、功能列表、非功能需求、技术栈、架构图、验收标准。

### OUTLINE (`docs/outline.md`)
任务拆分文档，按阶段组织，每任务含依赖声明：

```markdown
## 阶段 01-core

### 任务 1.1: 实现 RESP 解析器 <!-- @depends: none -->
...

### 任务 1.2: 实现命令路由 <!-- @depends: 1.1 -->
...

## 阶段 03-logging

### 任务 3.1: 实现日志系统 <!-- @depends: none -->
（可与 Phase 01 并行执行）
```

依赖标签支持 `@code_agents:N`、`@max_retries:N` 等配置覆盖。

### Phase 计划 (`docs/plans/<phase>/plan.md`)
Plan Agent 产出，包含：任务拆分、接口定义、测试策略、风险点。

### Bug 归档 (`docs/bugs/<phase>/<issue>.md`)
面向 Agent 的知识沉淀：

```markdown
---
phase: 01-core
issue_id: race-condition-db-access
keywords: [mutex, deadlock, await, db, async]
files: [src/db.rs, src/cmd/dispatch.rs]
severity: high
---

## 错误现象
## 根本原因
## 修复方案
## 禁止模式
```

## 编排流程

编排流程分为 6 个步骤，按顺序执行：

步骤 0 — 初始化。加载 orchestrator.yaml 配置，扫描 docs/bugs/ 目录构建归档索引。

步骤 1 — 需求收集。复用 brainstorming 技能，与用户对话明确需求，产出 docs/prd.md。

步骤 2 — OUTLINE 拆分。基于 PRD 将任务拆分为多个阶段和子任务，标注每个任务的 @depends 依赖标签，产出 docs/outline.md。

步骤 3 — 构建依赖图。解析 OUTLINE 中的 @depends 标签，计算哪些阶段可并行执行（如日志阶段不依赖核心阶段可提前启动）。

步骤 4 — 阶段循环。按依赖图从就绪阶段开始执行，最多并行数由 max_concurrent_phases 控制。每个阶段内部是一个四步流水线：

  4a. Plan Agent 读取 PRD 和 OUTLINE 对应章节，产出 docs/plans/<phase>/plan.md。

  4b. Code Agent(s) 启动时注入 anti-patterns 和匹配的 Bug 归档，按 plan.md 实现代码。

  4c. Review Agent(s) 审阅代码 diff 和 plan，产出 review report。通过则进入 4d，不通过且重试次数未达上限则退回 4b，达到上限则暂停等待用户决策。

  4d. 若当前是 Bug 修复任务，Archive Agent 将根因分析归档到 docs/bugs/。

步骤 5 — 完成报告。汇总所有阶段产出、归档统计、审查通过率。

### 人工介入条件

| 条件 | 动作 |
|------|------|
| 重试次数超限 | 暂停，列出失败信息和 review report，等待用户决策 |
| 用户 Ctrl-C | 保存进度到 `docs/.orchestrator-progress`，可恢复 |
| 用户主动介入 | 可在任意时刻对话干预 |

## code agent 知识注入

Code Agent 启动时：
1. 根据任务关键词/模块路径检索 `docs/bugs/` 中匹配归档
2. 将匹配归档摘要注入 prompt 前缀

注入格式：
```
## 历史 Bug 参考（请避免重复以下错误）

### Bug: race-condition-db-access
- 原因：跨 await 持有 Mutex 锁导致死锁
- 模式：在 with_db() 闭包内调用 .await
- 禁止：不要在 DB 锁的作用域内进行任何异步操作
```

## 技能文件结构

```
skills/orchestrator/
  skill.md                  # 编排主入口
  knowledge/
    inject-bugs.md          # Bug 知识注入 prompt 模板
    anti-patterns.md        # 通用编码禁止模式
  templates/
    prd.md                  # PRD 文档模板
    outline.md              # OUTLINE 任务拆分模板
    phase-plan.md           # 阶段计划模板
    review-report.md        # 审查报告模板
    bug-archive.md          # Bug 归档模板
```

### 与现有 Superpowers 技能的集成

orchestrator 不重写 agent 能力，只定义编排调度 + 文档模板 + 知识流转。每个阶段依赖已有的 Superpowers 技能：

- Phase 0-1 需求收集：复用 brainstorming 技能
- Phase 0-2 PRD 产出：主 Agent 直接写入 + 用户确认
- Phase 0-3 OUTLINE 拆分：主 Agent 基于 PRD 手动拆分
- Phase 1..N 阶段循环内的 Plan 步骤：调用 writing-plans 技能
- Phase 1..N 阶段循环内的 Code 步骤：调用 subagent-driven-development 技能，同时注入 inject-bugs.md 和匹配的历史 Bug 归档
- Phase 1..N 阶段循环内的 Review 步骤：调用 requesting-code-review 技能
- Phase 1..N 阶段循环内的归档步骤：Archive Agent 按 bug-archive.md 模板产出 Bug 归档

orchestrator 不重写 agent 能力，只定义编排调度 + 文档模板 + 知识流转。

## 实现路线

分 3 个 Phase 自举（用 orchestrator 技能构建 orchestrator 技能）：

| Phase | 内容  | 产出 |
|-------|-------|------|
| 01-scaffold | skill.md 骨架 + orchestrator.yaml schema + 6 模板 + 目录 | 可加载的空技能 |
| 02-core | 完整编排流程：配置加载→PRD→OUTLINE→依赖图→阶段循环→完成报告 | 功能完整的技能 |
| 03-polish | 并行调度 + 断点恢复 + 错误处理 + 用 Redis 项目跑通验证 | 生产可用 |

## 验证方式

- 用 codecrafters-redis-rust 项目作为首个 dogfood 案例跑通完整流程
- 配置缺省/错误时引导初始化
- 验证依赖拓扑计算正确
- 验证 Bug 归档的注入和检索
- 验证重试循环和人工介入
- 验证断点恢复
