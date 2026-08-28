//! 任务 DAG 分支拓扑（P10c T2，DNA 原则 3 + 架构蓝图 v11.0）。
//!
//! Helix 可自主创建"新 DAG 宇宙分支"：面对复杂目标（如"研究选购数码产品"），
//! 自主建立一个独立任务节点（从自画像延伸，反向链接到 Mind 知识库），
//! 并向下分化出调研、比价、决策等子节点。
//!
//! 绝对边界：所有自主生长与分化，**绝不越过 L0（基因锁）和 L1（自画像）**。
//! 任务知识归属 L2（经验/Wiki），不可反向污染底层人格。
//!
//! 哲学：如无必要勿增实体；要增就必须是 DAG 节点。

use chrono::Utc;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 任务 DAG 节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskNodeKind {
    /// 任务分支根（dag_branch_create 创建）
    TaskRoot,
    /// 子任务 / 调研 / 比价 / 决策等
    SubTask,
    /// 附着内容（思考 / 搜索结果 / 工具输出）
    Leaf,
}

/// 任务 DAG 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub branch_name: String,
    pub intent: String,
    pub kind: TaskNodeKind,
    /// 反向链接至 Helix-Mind 知识库节点（可选，契约语义）
    pub knowledge_ref: Option<String>,
    pub created_at: String,
}

/// 任务 DAG：有向无环图，节点为 TaskNode
pub struct TaskDag {
    graph: DiGraph<TaskNode, ()>,
    /// L0/L1 保护区：这些节点不可作为任务分支父节点（绝对边界）
    protected_roots: Vec<String>,
}

impl TaskDag {
    pub fn new(protected_roots: Vec<String>) -> Self {
        Self {
            graph: DiGraph::new(),
            protected_roots,
        }
    }

    fn find_node(&self, id: &str) -> Option<NodeIndex> {
        self.graph
            .node_indices()
            .find(|&i| self.graph.node_weight(i).map(|n| n.id == id).unwrap_or(false))
    }

    /// 创建任务分支 DAG 根节点。
    /// - `parent`：父节点 id（可选；None 或未知则挂到全局任务根）
    /// - 边界守卫：`parent` 不允许是 L0/L1 保护区节点
    /// - `knowledge_ref`：反向链接至 Mind 知识库节点（可选）
    pub fn dag_branch_create(
        &mut self,
        parent: Option<&str>,
        branch_name: &str,
        intent: &str,
        knowledge_ref: Option<String>,
    ) -> Result<String, String> {
        if let Some(p) = parent {
            if self.protected_roots.iter().any(|root| root == p) {
                return Err(format!(
                    "L0/L1 边界守卫：{} 是基因锁/自画像保护区，不可作为任务分支父节点",
                    p
                ));
            }
            if self.find_node(p).is_none() {
                return Err(format!("父节点不存在：{}", p));
            }
        }

        let node = TaskNode {
            id: Uuid::new_v4().to_string(),
            branch_name: branch_name.to_string(),
            intent: intent.to_string(),
            kind: TaskNodeKind::TaskRoot,
            knowledge_ref,
            created_at: Utc::now().to_rfc3339(),
        };
        let idx = self.graph.add_node(node.clone());
        if let Some(p) = parent {
            if let Some(p_idx) = self.find_node(p) {
                self.graph.add_edge(p_idx, idx, ());
            }
        }
        Ok(node.id)
    }

    /// 在任务分支下挂子任务节点（自主分化）
    pub fn add_subtask(&mut self, branch: &str, name: &str, intent: &str) -> Result<String, String> {
        let b_idx = self
            .find_node(branch)
            .ok_or_else(|| format!("任务分支不存在：{}", branch))?;
        let node = TaskNode {
            id: Uuid::new_v4().to_string(),
            branch_name: name.to_string(),
            intent: intent.to_string(),
            kind: TaskNodeKind::SubTask,
            knowledge_ref: None,
            created_at: Utc::now().to_rfc3339(),
        };
        let idx = self.graph.add_node(node.clone());
        self.graph.add_edge(b_idx, idx, ());
        Ok(node.id)
    }

    /// 附着叶子节点（思考 / 搜索结果 / 工具输出）
    pub fn attach_leaf(&mut self, parent: &str, content: &str) -> Result<String, String> {
        let p_idx = self
            .find_node(parent)
            .ok_or_else(|| format!("父节点不存在：{}", parent))?;
        let node = TaskNode {
            id: Uuid::new_v4().to_string(),
            branch_name: String::new(),
            intent: content.to_string(),
            kind: TaskNodeKind::Leaf,
            knowledge_ref: None,
            created_at: Utc::now().to_rfc3339(),
        };
        let idx = self.graph.add_node(node.clone());
        self.graph.add_edge(p_idx, idx, ());
        Ok(node.id)
    }

    /// 是否包含某节点
    pub fn contains(&self, id: &str) -> bool {
        self.find_node(id).is_some()
    }

    /// 节点总数
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_branch_create_links_to_parent() {
        let mut dag = TaskDag::new(vec!["L0::gene_lock".to_string(), "L1::self_portrait".to_string()]);
        // 先建一个根（无 parent）
        let root = dag.dag_branch_create(None, "root", "global root", None).unwrap();
        // 在根下建任务分支
        let branch = dag
            .dag_branch_create(Some(&root), "shopping_research", "研究选购数码产品", Some("mind::node-123".into()))
            .unwrap();
        assert!(dag.contains(&branch));
        assert_eq!(dag.len(), 2);
        // 挂子任务 + 叶子
        let subtask = dag.add_subtask(&branch, "compare", "比价").unwrap();
        let leaf = dag.attach_leaf(&subtask, "候选：A 型号 / B 型号").unwrap();
        assert!(dag.contains(&subtask) && dag.contains(&leaf));
        assert_eq!(dag.len(), 4);
    }

    #[test]
    fn l0_l1_boundary_guard() {
        let mut dag = TaskDag::new(vec!["L0::gene_lock".to_string(), "L1::self_portrait".to_string()]);
        // 越界：以 L0/L1 为父 → Err
        let err = dag.dag_branch_create(Some("L0::gene_lock"), "bad", "越界", None).unwrap_err();
        assert!(err.contains("边界守卫"));
        let err2 = dag.dag_branch_create(Some("L1::self_portrait"), "bad2", "越界", None).unwrap_err();
        assert!(err2.contains("边界守卫"));
        // 未知父节点 → Err
        let err3 = dag.dag_branch_create(Some("ghost"), "bad3", "不存在", None).unwrap_err();
        assert!(err3.contains("不存在"));
    }

    #[test]
    fn leaf_requires_existing_parent() {
        let mut dag = TaskDag::new(vec![]);
        assert!(dag.attach_leaf("ghost", "x").is_err());
    }
}
