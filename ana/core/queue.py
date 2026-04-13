"""
Anaphase 任务队列与调度系统核心实现。

包含四级队列 (Q0-Q3)、动态优先级公式、死锁检测、级联取消、
软/硬中断抢占及物理背压控制。
"""
import asyncio
import math
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

import structlog

logger = structlog.get_logger()


class QueueLevel(Enum):
    """四级优先级队列定义"""
    Q0 = 0  # 实时抢占 (P >= 80)
    Q1 = 1  # 高优先级 (60 <= P < 80)
    Q2 = 2  # 中等优先级 (30 <= P < 60)
    Q3 = 3  # 低优先级 (P < 30)


@dataclass(order=True)
class Task:
    """
    任务节点，按优先级排序。

    Attributes:
        priority_score: 用于排序的分数 (负值，因为 heapq 是最小堆)
        created_at: 创建时间戳
        task_id: 任务唯一 ID
        queue_level: 所属队列等级
        description: 任务描述
        depends_on: 依赖的任务 ID 列表
        blocked_by: 阻塞当前任务的未满足依赖 ID 列表
        state: 任务状态
        budget_tokens: 预算 Token 数
        tokens_consumed: 已消耗 Token 数
        steps_consumed: 已消耗认知步数
        started_at: 开始执行时间戳
    """
    priority_score: float = field(compare=True, default=0.0)

    created_at: float = field(compare=False, default_factory=time.time)
    task_id: str = field(compare=False, default="")
    queue_level: QueueLevel = field(compare=False, default=QueueLevel.Q3)

    description: str = field(compare=False, default="")
    depends_on: list[str] = field(compare=False, default_factory=list)
    blocked_by: list[str] = field(compare=False, default_factory=list)
    state: str = field(compare=False, default="PENDING")

    budget_tokens: int = field(compare=False, default=0)
    tokens_consumed: int = field(compare=False, default=0)

    steps_consumed: int = field(compare=False, default=0)
    started_at: float | None = field(compare=False, default=None)

    def __post_init__(self):
        if self.started_at is None:
            self.started_at = time.time()


class BackpressureController:
    """物理背压控制器，监控系统资源并决定降级模式"""

    def __init__(self):
        self.api_capacity: int = 3
        self.vram_pressure: float = 0.5
        self.mode: str = "normal"

    def update(self, api_capacity: int, vram_pressure: float) -> None:
        """更新资源水位"""
        self.api_capacity = api_capacity
        self.vram_pressure = vram_pressure

        if api_capacity == 0 or vram_pressure >= 0.95:
            self.mode = "degraded_l2"
        elif api_capacity <= 2 or vram_pressure >= 0.8:
            self.mode = "degraded_l1"
        else:
            self.mode = "normal"

        logger.debug("Backpressure mode updated", mode=self.mode)

    def allow_dequeue(self, task: "Task") -> bool:
        """判断是否允许任务出队"""
        real_priority = -task.priority_score
        if self.mode == "normal":
            return True
        elif self.mode == "degraded_l1":
            return task.queue_level == QueueLevel.Q0 or real_priority >= 80
        elif self.mode == "degraded_l2":
            return task.queue_level == QueueLevel.Q0
        return False


class MLFQScheduler:
    """多级反馈队列调度器 (MLFQ)"""

    def __init__(self, weights: dict[str, float] | None = None):
        self.queues: dict[QueueLevel, asyncio.PriorityQueue] = {
            level: asyncio.PriorityQueue() for level in QueueLevel
        }
        self.waiting_pool: dict[str, Task] = {}
        self.active_tasks: dict[str, Task] = {}
        self._completed_tasks: set[str] = set()
        self.depends_graph: dict[str, list[str]] = {}

        self.quantum_config: dict[QueueLevel, dict[str, Any]] = {
            QueueLevel.Q1: {"steps": 3, "seconds": 15, "tokens": 2000},
            QueueLevel.Q2: {"steps": 5, "seconds": 60, "tokens": 5000},
        }

        self.weights = weights or {"alpha": 0.35, "beta": 0.35, "gamma": 0.20, "delta": -0.10}
        self.backpressure = BackpressureController()
        self.deadlock_check_interval = 60.0
        self._last_deadlock_check = time.time()

        logger.info("MLFQScheduler initialized", weights=self.weights)

    @staticmethod
    def compute_dependency_score(blocked_tasks_count: int, max_log: float = 6.66) -> float:
        """计算依赖度评分 (对数压缩)"""
        if blocked_tasks_count == 0:
            return 0.0
        return 100.0 * min(1.0, math.log2(blocked_tasks_count + 1) / max_log)

    @staticmethod
    def compute_urgency(base_urgency: float, waiting_hours: float, lambda_u: float = 0.1) -> float:
        """计算衰减后的紧急度"""
        return base_urgency * math.exp(-lambda_u * waiting_hours)

    def calculate_priority(
        self,
        urgency: float,
        importance: float,
        blocked_count: int,
        cost_estimate: float
    ) -> float:
        """计算任务综合优先级 P(task)"""
        alpha = self.weights["alpha"]
        beta = self.weights["beta"]
        gamma = self.weights["gamma"]
        delta = self.weights["delta"]

        d_score = self.compute_dependency_score(blocked_count)
        c_score = 100.0 - min(100.0, max(0.0, cost_estimate))

        p = (alpha * urgency) + (beta * importance) + (gamma * d_score) + (delta * c_score)
        return max(0.0, min(100.0, p))

    def _get_queue_level(self, priority: float) -> QueueLevel:
        """根据优先级分数确定队列等级"""
        if priority >= 80:
            return QueueLevel.Q0
        elif priority >= 60:
            return QueueLevel.Q1
        elif priority >= 30:
            return QueueLevel.Q2
        return QueueLevel.Q3

    async def enqueue(self, task: Task) -> None:
        """任务入队"""
        if task.depends_on:
            unmet = [dep for dep in task.depends_on if dep not in self._completed_tasks]
            if unmet:
                task.state = "WAITING"
                task.blocked_by = unmet
                self.waiting_pool[task.task_id] = task
                self._update_depends_graph(task)
                logger.debug("Task waiting for dependencies", task_id=task.task_id, deps=unmet)
                return

        sort_priority = -task.priority_score
        level = self._get_queue_level(task.priority_score)
        task.queue_level = level
        task.state = "READY"

        queue_task = Task(
            priority_score=sort_priority,
            created_at=task.created_at,
            task_id=task.task_id,
            queue_level=level,
            description=task.description,
            depends_on=task.depends_on,
            blocked_by=task.blocked_by,
            state=task.state,
            budget_tokens=task.budget_tokens,
            tokens_consumed=task.tokens_consumed,
            steps_consumed=task.steps_consumed,
            started_at=task.started_at
        )

        await self.queues[level].put(queue_task)
        logger.debug("Task enqueued", task_id=task.task_id, level=level.name)

    async def get_next(self) -> Task | None:
        """出队下一个任务"""
        now = time.time()
        if now - self._last_deadlock_check > self.deadlock_check_interval:
            await self.detect_and_break_deadlock()
            self._last_deadlock_check = now

        backpressure_mode = self.backpressure.mode

        for level in [QueueLevel.Q0, QueueLevel.Q1, QueueLevel.Q2, QueueLevel.Q3]:
            if backpressure_mode == "degraded_l2" and level != QueueLevel.Q0:
                continue
            if backpressure_mode == "degraded_l1" and level not in [QueueLevel.Q0, QueueLevel.Q1]:
                continue

            if not self.queues[level].empty():
                try:
                    task = await asyncio.wait_for(self.queues[level].get(), timeout=0.05)

                    if not self.backpressure.allow_dequeue(task):
                        await self.queues[level].put(task)
                        continue

                    task.state = "ACTIVE"
                    task.started_at = time.time()
                    self.active_tasks[task.task_id] = task
                    logger.info("Task dequeued", task_id=task.task_id, level=level.name)
                    return task
                except TimeoutError:
                    continue

        return None

    def mark_task_completed(self, task_id: str) -> None:
        """标记任务完成"""
        self._completed_tasks.add(task_id)
        if task_id in self.active_tasks:
            del self.active_tasks[task_id]

        unlocked = []
        for tid, task in list(self.waiting_pool.items()):
            if task_id in task.blocked_by:
                task.blocked_by.remove(task_id)
                if not task.blocked_by:
                    unlocked.append(task)

        for task in unlocked:
            del self.waiting_pool[task.task_id]
            asyncio.create_task(self.enqueue(task))

        self._remove_from_depends_graph(task_id)

    def check_quantum_exhausted(self, task: Task) -> bool:
        """检查任务是否耗尽量子"""
        if task.queue_level not in [QueueLevel.Q1, QueueLevel.Q2]:
            return False

        config = self.quantum_config.get(task.queue_level)
        if not config:
            return False

        elapsed = time.time() - (task.started_at or time.time())
        return (
            task.steps_consumed >= config["steps"] or
            elapsed >= config["seconds"] or
            task.tokens_consumed >= config["tokens"]
        )

    async def demote_task(self, task: Task) -> None:
        """任务降级"""
        new_level = None
        if task.queue_level == QueueLevel.Q1:
            new_level = QueueLevel.Q2
        elif task.queue_level == QueueLevel.Q2:
            new_level = QueueLevel.Q3
        else:
            return

        task.queue_level = new_level
        task.steps_consumed = 0
        task.started_at = time.time()
        task.state = "READY"

        queue_task = Task(
            priority_score=task.priority_score,
            created_at=task.created_at,
            task_id=task.task_id,
            queue_level=new_level,
            description=task.description,
            depends_on=task.depends_on,
            blocked_by=task.blocked_by,
            state=task.state,
            budget_tokens=task.budget_tokens,
            tokens_consumed=task.tokens_consumed,
            steps_consumed=task.steps_consumed,
            started_at=task.started_at
        )

        await self.queues[new_level].put(queue_task)
        logger.info("Task demoted", task_id=task.task_id, new_level=new_level.name)

    async def hard_preempt(self, current_task: Task, new_task: Task) -> bool:
        """硬中断"""
        p_new = new_task.priority_score
        p_current = current_task.priority_score

        if p_new >= 90 and (p_new - p_current) >= 30:
            logger.warning(
                "Hard preemption triggered",
                current=current_task.task_id,
                new=new_task.task_id
            )
            await self._abort_llm_stream(current_task)
            self._save_partial_context(current_task)
            current_task.state = "SUSPENDED_HARD"

            if current_task.task_id in self.active_tasks:
                del self.active_tasks[current_task.task_id]
            await self.enqueue(current_task)
            return True
        return False

    async def _abort_llm_stream(self, task: Task) -> None:
        logger.debug("Aborting LLM stream", task_id=task.task_id)

    def _save_partial_context(self, task: Task) -> None:
        logger.debug("Saving partial context", task_id=task.task_id)

    async def cancel_task(self, task_id: str, reason: str, cascade: bool = True) -> list[str]:
        """取消任务"""
        cancelled = [task_id]
        task = None

        if task_id in self.active_tasks:
            task = self.active_tasks.pop(task_id)
            task.state = "CANCELLED"
            await self._abort_task(task)
        elif task_id in self.waiting_pool:
            task = self.waiting_pool.pop(task_id)
            task.state = "CANCELLED"
        else:
            logger.warning("Task not found for cancellation", task_id=task_id)
            return cancelled

        if not cascade:
            logger.info("Task cancelled (no cascade)", task_id=task_id, reason=reason)
            return cancelled

        children = self._find_all_dependents(task_id)
        for child_id in children:
            child = None
            if child_id in self.waiting_pool:
                child = self.waiting_pool.pop(child_id)
                child.state = "CANCELLED_DEPENDENCY_FAILED"
            elif child_id in self.active_tasks:
                child = self.active_tasks.pop(child_id)
                child.state = "CANCELLED_DEPENDENCY_FAILED"
                await self._abort_task(child)

            if child:
                cancelled.append(child_id)
                logger.info("Cascade cancelled", task_id=child_id, parent=task_id)

        self._remove_from_depends_graph(task_id)
        return cancelled

    def _find_all_dependents(self, task_id: str) -> set[str]:
        """BFS 找出所有依赖该任务的后继任务"""
        visited = set()
        queue = [task_id]

        while queue:
            current = queue.pop(0)
            for tid, task in self.waiting_pool.items():
                if current in task.depends_on and tid not in visited:
                    visited.add(tid)
                    queue.append(tid)
            for tid, task in self.active_tasks.items():
                if current in task.depends_on and tid not in visited:
                    visited.add(tid)
                    queue.append(tid)

        return visited

    async def _abort_task(self, task: Task) -> None:
        logger.debug("Aborting task", task_id=task.task_id)

    def _update_depends_graph(self, task: Task) -> None:
        self.depends_graph[task.task_id] = task.depends_on[:]

    def _remove_from_depends_graph(self, task_id: str) -> None:
        if task_id in self.depends_graph:
            del self.depends_graph[task_id]
        for deps in self.depends_graph.values():
            if task_id in deps:
                deps.remove(task_id)

    async def detect_and_break_deadlock(self) -> None:
        """死锁检测"""
        graph = {tid: task.depends_on[:] for tid, task in self.waiting_pool.items()}
        cycles = self._find_cycles_tarjan(graph)

        for cycle in cycles:
            logger.warning("Deadlock detected", cycle=cycle)
            valid_cycle = [tid for tid in cycle if tid in self.waiting_pool]
            if not valid_cycle:
                continue

            victim = min(valid_cycle, key=lambda tid: self.waiting_pool[tid].priority_score)
            await self.cancel_task(victim, "DEADLOCK_VICTIM", cascade=False)
            logger.critical("Deadlock broken by sacrificing task", victim=victim)

    def _find_cycles_tarjan(self, graph: dict[str, list[str]]) -> list[list[str]]:
        """Tarjan 强连通分量算法"""
        index_counter = [0]
        stack = []
        lowlink = {}
        index = {}
        on_stack = set()
        sccs = []

        def strongconnect(node):
            index[node] = index_counter[0]
            lowlink[node] = index_counter[0]
            index_counter[0] += 1
            stack.append(node)
            on_stack.add(node)

            for neighbor in graph.get(node, []):
                if neighbor not in index:
                    if neighbor in graph:
                        strongconnect(neighbor)
                        lowlink[node] = min(lowlink[node], lowlink[neighbor])
                elif neighbor in on_stack:
                    lowlink[node] = min(lowlink[node], index[neighbor])

            if lowlink[node] == index[node]:
                scc = []
                while True:
                    w = stack.pop()
                    on_stack.remove(w)
                    scc.append(w)
                    if w == node:
                        break
                if len(scc) > 1:
                    sccs.append(scc)

        for node in graph:
            if node not in index:
                strongconnect(node)

        return sccs

    async def merge_low_priority_tasks(self) -> None:
        """合并 Q3 中同类型的低价值任务"""
        q3_tasks = [t for t in self.waiting_pool.values() if t.queue_level == QueueLevel.Q3]

        groups: dict[str, list[Task]] = {}
        for task in q3_tasks:
            parts = task.description.split()
            task_type = parts[0] if parts else "unknown"
            if task_type not in groups:
                groups[task_type] = []
            groups[task_type].append(task)

        for task_type, tasks in groups.items():
            if len(tasks) >= 3:
                batch_id = f"batch_{task_type}_{int(time.time())}"
                batch_task = Task(
                    priority_score=min(t.priority_score for t in tasks),
                    task_id=batch_id,
                    description=f"批量处理 {len(tasks)} 个 {task_type} 任务",
                    budget_tokens=sum(t.budget_tokens for t in tasks)
                )

                for t in tasks:
                    del self.waiting_pool[t.task_id]
                    self._remove_from_depends_graph(t.task_id)

                await self.enqueue(batch_task)
                logger.info("Tasks merged", type=task_type, count=len(tasks), batch_id=batch_id)

    async def expire_stale_tasks(self) -> None:
        """过期任务丢弃"""
        stale_threshold = time.time() - 24 * 3600
        to_remove = []

        for task_id, task in self.waiting_pool.items():
            if (task.queue_level == QueueLevel.Q3 and
                task.created_at < stale_threshold and
                task.priority_score < 1.0):
                to_remove.append(task_id)

        for task_id in to_remove:
            task = self.waiting_pool.pop(task_id)
            task.state = "EXPIRED"
            self._remove_from_depends_graph(task_id)
            logger.info("Task expired", task_id=task_id)


class InterruptibleAgentLoop:
    """支持软/硬中断的 Agent Loop 包装器"""

    def __init__(self, loop_impl: Any):
        self.loop_impl = loop_impl
        self.keep_running = True
        self.pending_interrupt = False

    async def run(self, task: Task, scheduler: MLFQScheduler) -> dict[str, Any]:
        self.keep_running = True
        self.pending_interrupt = False

        result = {}
        while self.keep_running:
            step_result = await self._execute_step(task)
            task.steps_consumed += 1
            if "tokens_used" in step_result:
                task.tokens_consumed += step_result["tokens_used"]

            result = step_result

            if self.pending_interrupt:
                logger.info("Soft interrupt received", task_id=task.task_id)
                self._save_context(task)
                self.keep_running = False
                return {"status": "SUSPENDED", "reason": "preempted", "partial": result}

            if scheduler.check_quantum_exhausted(task):
                logger.info("Quantum exhausted", task_id=task.task_id)
                await scheduler.demote_task(task)
                self.keep_running = False
                return {"status": "SUSPENDED", "reason": "quantum_exhausted", "partial": result}

        return {"status": "COMPLETED", "result": result}

    async def _execute_step(self, task: Task) -> dict[str, Any]:
        return await self.loop_impl.step(task)

    def signal_interrupt(self) -> None:
        self.pending_interrupt = True

    def _save_context(self, task: Task) -> None:
        logger.debug("Saving L3 context", task_id=task.task_id)
