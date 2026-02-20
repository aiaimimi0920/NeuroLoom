//! GraphRAG - 空间拓扑记忆
//!
//! 维护代码库 AST 的空间拓扑结构。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 图节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    /// 文件
    File,
    /// 函数
    Function,
    /// 结构体
    Struct,
    /// 模块
    Module,
    /// 依赖
    Dependency,
}

/// 图节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// 节点 ID
    pub id: Uuid,
    /// 节点名称
    pub name: String,
    /// 节点类型
    pub node_type: NodeType,
    /// 文件路径 (如果适用)
    pub path: Option<String>,
    /// 代码位置
    pub location: Option<CodeLocation>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 代码位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// 图边类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    /// 调用关系
    Calls,
    /// 导入关系
    Imports,
    /// 定义关系
    Defines,
    /// 依赖关系
    DependsOn,
}

/// 图边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// 源节点 ID
    pub source: Uuid,
    /// 目标节点 ID
    pub target: Uuid,
    /// 边类型
    pub edge_type: EdgeType,
}

/// GraphRAG 图数据库
pub struct GraphRAG {
    /// 节点集合
    nodes: HashMap<Uuid, GraphNode>,
    /// 边集合
    edges: Vec<GraphEdge>,
    /// 名称索引
    name_index: HashMap<String, Uuid>,
    /// 路径索引
    path_index: HashMap<String, Uuid>,
}

impl GraphRAG {
    /// 创建新的 GraphRAG
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            name_index: HashMap::new(),
            path_index: HashMap::new(),
        }
    }

    /// 添加节点
    pub fn add_node(&mut self, node: GraphNode) {
        if let Some(path) = &node.path {
            self.path_index.insert(path.clone(), node.id);
        }
        self.name_index.insert(node.name.clone(), node.id);
        self.nodes.insert(node.id, node);
    }

    /// 添加边
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// 通过名称查找节点
    pub fn find_by_name(&self, name: &str) -> Option<&GraphNode> {
        self.name_index.get(name).and_then(|id| self.nodes.get(id))
    }

    /// 通过路径查找节点
    pub fn find_by_path(&self, path: &str) -> Option<&GraphNode> {
        self.path_index.get(path).and_then(|id| self.nodes.get(id))
    }

    /// 获取节点的所有出边
    pub fn get_outgoing_edges(&self, node_id: &Uuid) -> Vec<&GraphEdge> {
        self.edges
            .iter()
            .filter(|e| &e.source == node_id)
            .collect()
    }

    /// 获取节点的所有入边
    pub fn get_incoming_edges(&self, node_id: &Uuid) -> Vec<&GraphEdge> {
        self.edges
            .iter()
            .filter(|e| &e.target == node_id)
            .collect()
    }

    /// 查找调用者
    pub fn find_callers(&self, node_id: &Uuid) -> Vec<&GraphNode> {
        self.get_incoming_edges(node_id)
            .iter()
            .filter(|e| e.edge_type == EdgeType::Calls)
            .filter_map(|e| self.nodes.get(&e.source))
            .collect()
    }

    /// 查找被调用者
    pub fn find_callees(&self, node_id: &Uuid) -> Vec<&GraphNode> {
        self.get_outgoing_edges(node_id)
            .iter()
            .filter(|e| e.edge_type == EdgeType::Calls)
            .filter_map(|e| self.nodes.get(&e.target))
            .collect()
    }

    /// 获取节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取边数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for GraphRAG {
    fn default() -> Self {
        Self::new()
    }
}
