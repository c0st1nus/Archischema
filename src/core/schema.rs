use petgraph::Directed;
use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};

/// Узел графа - таблица базы данных
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TableNode {
    pub name: String,
    pub columns: Vec<Column>,
    /// Позиция на канвасе (x, y)
    pub position: (f64, f64),
}

impl TableNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            position: (0.0, 0.0),
        }
    }

    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = (x, y);
        self
    }

    pub fn add_column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }
}

/// Колонка таблицы
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub default_value: Option<String>,
}

impl Column {
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            is_primary_key: false,
            is_nullable: true,
            is_unique: false,
            default_value: None,
        }
    }

    pub fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self.is_nullable = false;
        self
    }

    pub fn not_null(mut self) -> Self {
        self.is_nullable = false;
        self
    }

    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }
}

/// Ребро графа - связь между таблицами
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Relationship {
    pub name: String,
    pub relationship_type: RelationshipType,
    /// Имя колонки в таблице-источнике
    pub from_column: String,
    /// Имя колонки в таблице-цели
    pub to_column: String,
}

impl Relationship {
    pub fn new(
        name: impl Into<String>,
        relationship_type: RelationshipType,
        from_column: impl Into<String>,
        to_column: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            relationship_type,
            from_column: from_column.into(),
            to_column: to_column.into(),
        }
    }
}

/// Тип связи между таблицами
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum RelationshipType {
    /// Один к одному
    OneToOne,
    /// Один ко многим
    OneToMany,
    /// Многие ко многим
    ManyToMany,
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipType::OneToOne => write!(f, "1:1"),
            RelationshipType::OneToMany => write!(f, "1:N"),
            RelationshipType::ManyToMany => write!(f, "N:M"),
        }
    }
}

/// Тип графа: узлы - таблицы, ребра - связи
pub type SchemaGraph = StableGraph<TableNode, Relationship, Directed>;

/// Создать демо-граф для тестирования
pub fn create_demo_graph() -> SchemaGraph {
    let mut graph = SchemaGraph::new();

    // Создаем таблицы
    let users = graph.add_node(
        TableNode::new("users")
            .with_position(100.0, 100.0)
            .add_column(Column::new("id", "INTEGER").primary_key())
            .add_column(Column::new("username", "VARCHAR(255)").not_null().unique())
            .add_column(Column::new("email", "VARCHAR(255)").not_null().unique())
            .add_column(Column::new("created_at", "TIMESTAMP").not_null()),
    );

    let posts = graph.add_node(
        TableNode::new("posts")
            .with_position(400.0, 100.0)
            .add_column(Column::new("id", "INTEGER").primary_key())
            .add_column(Column::new("user_id", "INTEGER").not_null())
            .add_column(Column::new("title", "VARCHAR(255)").not_null())
            .add_column(Column::new("content", "TEXT"))
            .add_column(Column::new("created_at", "TIMESTAMP").not_null()),
    );

    let comments = graph.add_node(
        TableNode::new("comments")
            .with_position(400.0, 400.0)
            .add_column(Column::new("id", "INTEGER").primary_key())
            .add_column(Column::new("post_id", "INTEGER").not_null())
            .add_column(Column::new("user_id", "INTEGER").not_null())
            .add_column(Column::new("content", "TEXT").not_null())
            .add_column(Column::new("created_at", "TIMESTAMP").not_null()),
    );

    // Создаем связи
    graph.add_edge(
        users,
        posts,
        Relationship::new("user_posts", RelationshipType::OneToMany, "id", "user_id"),
    );

    graph.add_edge(
        posts,
        comments,
        Relationship::new(
            "post_comments",
            RelationshipType::OneToMany,
            "id",
            "post_id",
        ),
    );

    graph.add_edge(
        users,
        comments,
        Relationship::new(
            "user_comments",
            RelationshipType::OneToMany,
            "id",
            "user_id",
        ),
    );

    graph
}
