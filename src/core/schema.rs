use petgraph::Directed;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::validation;

/// Стандартные типы данных MySQL
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum MySqlDataType {
    // Числовые типы
    TinyInt,
    SmallInt,
    MediumInt,
    Int,
    BigInt,
    Decimal(u8, u8), // (precision, scale)
    Float,
    Double,

    // Строковые типы
    Char(u16),    // до 255
    Varchar(u16), // до 65535
    TinyText,
    Text,
    MediumText,
    LongText,

    // Бинарные типы
    Binary(u16),
    Varbinary(u16),
    TinyBlob,
    Blob,
    MediumBlob,
    LongBlob,

    // Дата и время
    Date,
    DateTime,
    Timestamp,
    Time,
    Year,

    // Другие типы
    Enum(Vec<String>),
    Set(Vec<String>),
    Json,
    Boolean,
}

impl fmt::Display for MySqlDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MySqlDataType::TinyInt => write!(f, "TINYINT"),
            MySqlDataType::SmallInt => write!(f, "SMALLINT"),
            MySqlDataType::MediumInt => write!(f, "MEDIUMINT"),
            MySqlDataType::Int => write!(f, "INT"),
            MySqlDataType::BigInt => write!(f, "BIGINT"),
            MySqlDataType::Decimal(p, s) => write!(f, "DECIMAL({},{})", p, s),
            MySqlDataType::Float => write!(f, "FLOAT"),
            MySqlDataType::Double => write!(f, "DOUBLE"),
            MySqlDataType::Char(len) => write!(f, "CHAR({})", len),
            MySqlDataType::Varchar(len) => write!(f, "VARCHAR({})", len),
            MySqlDataType::TinyText => write!(f, "TINYTEXT"),
            MySqlDataType::Text => write!(f, "TEXT"),
            MySqlDataType::MediumText => write!(f, "MEDIUMTEXT"),
            MySqlDataType::LongText => write!(f, "LONGTEXT"),
            MySqlDataType::Binary(len) => write!(f, "BINARY({})", len),
            MySqlDataType::Varbinary(len) => write!(f, "VARBINARY({})", len),
            MySqlDataType::TinyBlob => write!(f, "TINYBLOB"),
            MySqlDataType::Blob => write!(f, "BLOB"),
            MySqlDataType::MediumBlob => write!(f, "MEDIUMBLOB"),
            MySqlDataType::LongBlob => write!(f, "LONGBLOB"),
            MySqlDataType::Date => write!(f, "DATE"),
            MySqlDataType::DateTime => write!(f, "DATETIME"),
            MySqlDataType::Timestamp => write!(f, "TIMESTAMP"),
            MySqlDataType::Time => write!(f, "TIME"),
            MySqlDataType::Year => write!(f, "YEAR"),
            MySqlDataType::Enum(values) => write!(f, "ENUM('{}')", values.join("','")),
            MySqlDataType::Set(values) => write!(f, "SET('{}')", values.join("','")),
            MySqlDataType::Json => write!(f, "JSON"),
            MySqlDataType::Boolean => write!(f, "BOOLEAN"),
        }
    }
}

impl MySqlDataType {
    /// Получить список всех доступных типов для выбора
    pub fn all_types() -> &'static [&'static str] {
        &[
            "INT",
            "BIGINT",
            "TINYINT",
            "SMALLINT",
            "MEDIUMINT",
            "DECIMAL",
            "FLOAT",
            "DOUBLE",
            "VARCHAR",
            "CHAR",
            "TEXT",
            "TINYTEXT",
            "MEDIUMTEXT",
            "LONGTEXT",
            "BLOB",
            "TINYBLOB",
            "MEDIUMBLOB",
            "LONGBLOB",
            "DATE",
            "DATETIME",
            "TIMESTAMP",
            "TIME",
            "YEAR",
            "BOOLEAN",
            "JSON",
            "ENUM",
            "SET",
        ]
    }
}

/// Узел графа - таблица базы данных
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TableNode {
    /// Stable UUID for identifying tables across LiveShare clients
    pub uuid: Uuid,
    pub name: String,
    pub columns: Vec<Column>,
    /// Позиция на канвасе (x, y)
    pub position: (f64, f64),
}

impl TableNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            uuid: Uuid::new_v4(),
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

    /// Добавить новую колонку (CRUD: Create)
    pub fn create_column(&mut self, column: Column) {
        self.columns.push(column);
    }

    /// Обновить колонку по индексу (CRUD: Update)
    pub fn update_column(&mut self, index: usize, column: Column) -> Result<(), String> {
        if index < self.columns.len() {
            self.columns[index] = column;
            Ok(())
        } else {
            Err(format!("Column index {} out of bounds", index))
        }
    }

    /// Удалить колонку по индексу (CRUD: Delete)
    pub fn delete_column(&mut self, index: usize) -> Result<Column, String> {
        if index < self.columns.len() {
            Ok(self.columns.remove(index))
        } else {
            Err(format!("Column index {} out of bounds", index))
        }
    }

    /// Получить колонку по индексу (CRUD: Read)
    pub fn get_column(&self, index: usize) -> Option<&Column> {
        self.columns.get(index)
    }

    /// Получить мутабельную ссылку на колонку по индексу
    pub fn get_column_mut(&mut self, index: usize) -> Option<&mut Column> {
        self.columns.get_mut(index)
    }

    /// Найти колонку по имени
    pub fn find_column(&self, name: &str) -> Option<(usize, &Column)> {
        self.columns
            .iter()
            .enumerate()
            .find(|(_, col)| col.name == name)
    }

    /// Переместить колонку на новую позицию
    pub fn move_column(&mut self, from_index: usize, to_index: usize) -> Result<(), String> {
        if from_index >= self.columns.len() {
            return Err(format!("Source index {} out of bounds", from_index));
        }
        if to_index >= self.columns.len() {
            return Err(format!("Target index {} out of bounds", to_index));
        }

        let column = self.columns.remove(from_index);
        self.columns.insert(to_index, column);
        Ok(())
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

    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }

    /// Валидация имени колонки
    pub fn validate_name(name: &str) -> Result<(), String> {
        validation::validate_column_name(name)
    }

    /// Валидация типа данных
    pub fn validate_data_type(data_type: &str) -> Result<(), String> {
        if data_type.is_empty() {
            return Err("Data type cannot be empty".to_string());
        }
        Ok(())
    }

    /// Извлекает базовый тип из строки типа данных
    /// Например: VARCHAR(255) -> VARCHAR, DECIMAL(10,2) -> DECIMAL
    pub fn get_base_type(&self) -> String {
        let data_type = self.data_type.to_uppercase();

        // Если есть скобки, берём только то, что до них
        if let Some(paren_pos) = data_type.find('(') {
            data_type[..paren_pos].trim().to_string()
        } else {
            data_type.trim().to_string()
        }
    }

    /// Статические группы типов для проверки совместимости (без аллокаций)
    const INTEGER_TYPES: &'static [&'static str] =
        &["TINYINT", "SMALLINT", "MEDIUMINT", "INT", "BIGINT"];
    const STRING_TYPES: &'static [&'static str] = &[
        "CHAR",
        "VARCHAR",
        "TINYTEXT",
        "TEXT",
        "MEDIUMTEXT",
        "LONGTEXT",
    ];
    const FLOAT_TYPES: &'static [&'static str] = &["FLOAT", "DOUBLE", "DECIMAL"];
    const BINARY_TYPES: &'static [&'static str] = &[
        "BINARY",
        "VARBINARY",
        "TINYBLOB",
        "BLOB",
        "MEDIUMBLOB",
        "LONGBLOB",
    ];
    const DATETIME_TYPES: &'static [&'static str] = &["DATE", "DATETIME", "TIMESTAMP", "TIME"];

    /// Проверяет совместимость типов данных между PK и FK
    /// Оптимизировано: использует статические массивы вместо создания Vec при каждом вызове
    #[inline]
    pub fn is_type_compatible_with(&self, other: &Column) -> bool {
        let self_base = self.get_base_type();
        let other_base = other.get_base_type();

        // Точное совпадение базового типа
        if self_base == other_base {
            return true;
        }

        let self_str = self_base.as_str();
        let other_str = other_base.as_str();

        // Проверяем совместимость по группам типов
        Self::INTEGER_TYPES.contains(&self_str) && Self::INTEGER_TYPES.contains(&other_str)
            || Self::STRING_TYPES.contains(&self_str) && Self::STRING_TYPES.contains(&other_str)
            || Self::FLOAT_TYPES.contains(&self_str) && Self::FLOAT_TYPES.contains(&other_str)
            || Self::BINARY_TYPES.contains(&self_str) && Self::BINARY_TYPES.contains(&other_str)
            || Self::DATETIME_TYPES.contains(&self_str) && Self::DATETIME_TYPES.contains(&other_str)
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
    /// Многие к одному
    ManyToOne,
    /// Многие ко многим (через промежуточную таблицу)
    ManyToMany,
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipType::OneToOne => write!(f, "1:1"),
            RelationshipType::OneToMany => write!(f, "1:N"),
            RelationshipType::ManyToOne => write!(f, "N:1"),
            RelationshipType::ManyToMany => write!(f, "N:M"),
        }
    }
}

/// Тип графа: узлы - таблицы, ребра - связи
pub type SchemaGraph = StableGraph<TableNode, Relationship, Directed>;

/// Расширение SchemaGraph для работы с таблицами
pub trait TableOps {
    /// Создать новую таблицу с уникальным именем
    fn create_table(
        &mut self,
        name: impl Into<String>,
        position: (f64, f64),
    ) -> Result<petgraph::graph::NodeIndex, String>;

    /// Создать таблицу с автоматическим именем (new_table_1, new_table_2, и т.д.)
    fn create_table_auto(&mut self, position: (f64, f64)) -> petgraph::graph::NodeIndex;

    /// Переименовать таблицу
    fn rename_table(
        &mut self,
        node_idx: petgraph::graph::NodeIndex,
        new_name: impl Into<String>,
    ) -> Result<(), String>;

    /// Удалить таблицу (и все связанные с ней связи)
    fn delete_table(&mut self, node_idx: petgraph::graph::NodeIndex) -> Result<TableNode, String>;

    /// Проверить, существует ли таблица с таким именем
    fn table_exists(&self, name: &str) -> bool;

    /// Найти таблицу по имени
    fn find_table_by_name(&self, name: &str) -> Option<petgraph::graph::NodeIndex>;

    /// Сгенерировать уникальное имя таблицы
    fn generate_unique_table_name(&self, base_name: &str) -> String;
}

impl TableOps for SchemaGraph {
    fn create_table(
        &mut self,
        name: impl Into<String>,
        position: (f64, f64),
    ) -> Result<petgraph::graph::NodeIndex, String> {
        let name = name.into();

        // Валидируем имя таблицы
        validation::validate_table_name(&name)?;

        // Проверяем уникальность имени
        if self.table_exists(&name) {
            return Err(format!("Table '{}' already exists", name));
        }

        // Создаем таблицу
        let table = TableNode::new(name).with_position(position.0, position.1);
        Ok(self.add_node(table))
    }

    fn create_table_auto(&mut self, position: (f64, f64)) -> petgraph::graph::NodeIndex {
        let name = self.generate_unique_table_name("new_table");
        let table = TableNode::new(name).with_position(position.0, position.1);
        self.add_node(table)
    }

    fn rename_table(
        &mut self,
        node_idx: petgraph::graph::NodeIndex,
        new_name: impl Into<String>,
    ) -> Result<(), String> {
        let new_name = new_name.into();

        // Валидируем новое имя
        validation::validate_table_name(&new_name)?;

        // Получаем текущее имя
        let current_name = self
            .node_weight(node_idx)
            .map(|n| n.name.clone())
            .ok_or_else(|| "Table not found".to_string())?;

        // Если имя не изменилось, просто возвращаем Ok
        if current_name == new_name {
            return Ok(());
        }

        // Проверяем уникальность нового имени
        if self.table_exists(&new_name) {
            return Err(format!("Table '{}' already exists", new_name));
        }

        // Переименовываем
        if let Some(node) = self.node_weight_mut(node_idx) {
            node.name = new_name;
            Ok(())
        } else {
            Err("Table not found".to_string())
        }
    }

    fn delete_table(&mut self, node_idx: petgraph::graph::NodeIndex) -> Result<TableNode, String> {
        if let Some(table) = self.remove_node(node_idx) {
            Ok(table)
        } else {
            Err("Table not found".to_string())
        }
    }

    fn table_exists(&self, name: &str) -> bool {
        self.node_weights().any(|node| node.name == name)
    }

    fn find_table_by_name(&self, name: &str) -> Option<petgraph::graph::NodeIndex> {
        self.node_indices().find(|&idx| {
            self.node_weight(idx)
                .map(|node| node.name == name)
                .unwrap_or(false)
        })
    }

    fn generate_unique_table_name(&self, base_name: &str) -> String {
        // Сначала проверяем базовое имя без суффикса
        if !self.table_exists(base_name) {
            return base_name.to_string();
        }

        // Собираем максимальный суффикс за один проход по графу
        let prefix = format!("{}_", base_name);
        let max_suffix: u32 = self
            .node_weights()
            .filter_map(|node| {
                node.name
                    .strip_prefix(&prefix)
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(1);

        format!("{}_{}", base_name, max_suffix + 1)
    }
}

/// Расширение SchemaGraph для работы со связями
pub trait RelationshipOps {
    /// Создать новую связь между таблицами
    fn create_relationship(
        &mut self,
        from_table: petgraph::graph::NodeIndex,
        to_table: petgraph::graph::NodeIndex,
        relationship: Relationship,
    ) -> Result<petgraph::graph::EdgeIndex, String>;

    /// Получить связь по индексу
    fn get_relationship(&self, edge_idx: petgraph::graph::EdgeIndex) -> Option<&Relationship>;

    /// Обновить связь
    fn update_relationship(
        &mut self,
        edge_idx: petgraph::graph::EdgeIndex,
        relationship: Relationship,
    ) -> Result<(), String>;

    /// Удалить связь
    fn delete_relationship(&mut self, edge_idx: petgraph::graph::EdgeIndex) -> Result<(), String>;

    /// Найти все связи от указанной таблицы
    fn find_relationships_from(
        &self,
        from_table: petgraph::graph::NodeIndex,
    ) -> Vec<(
        petgraph::graph::EdgeIndex,
        petgraph::graph::NodeIndex,
        &Relationship,
    )>;

    /// Найти все связи к указанной таблице
    fn find_relationships_to(
        &self,
        to_table: petgraph::graph::NodeIndex,
    ) -> Vec<(
        petgraph::graph::EdgeIndex,
        petgraph::graph::NodeIndex,
        &Relationship,
    )>;

    /// Найти связь между двумя таблицами по колонкам
    fn find_relationship_by_columns(
        &self,
        from_table: petgraph::graph::NodeIndex,
        to_table: petgraph::graph::NodeIndex,
        from_column: &str,
        to_column: &str,
    ) -> Option<petgraph::graph::EdgeIndex>;
}

impl RelationshipOps for SchemaGraph {
    fn create_relationship(
        &mut self,
        from_table: petgraph::graph::NodeIndex,
        to_table: petgraph::graph::NodeIndex,
        relationship: Relationship,
    ) -> Result<petgraph::graph::EdgeIndex, String> {
        // Проверяем, что обе таблицы существуют
        if !self.contains_node(from_table) {
            return Err("Source table not found".to_string());
        }
        if !self.contains_node(to_table) {
            return Err("Target table not found".to_string());
        }

        // Проверяем, что колонки существуют
        let from_node = self.node_weight(from_table).unwrap();
        if from_node.find_column(&relationship.from_column).is_none() {
            return Err(format!(
                "Column '{}' not found in source table",
                relationship.from_column
            ));
        }

        let to_node = self.node_weight(to_table).unwrap();
        if to_node.find_column(&relationship.to_column).is_none() {
            return Err(format!(
                "Column '{}' not found in target table",
                relationship.to_column
            ));
        }

        // Проверяем, не существует ли уже такая связь
        if self
            .find_relationship_by_columns(
                from_table,
                to_table,
                &relationship.from_column,
                &relationship.to_column,
            )
            .is_some()
        {
            return Err("Relationship already exists".to_string());
        }

        // Создаём связь
        let edge_idx = self.add_edge(from_table, to_table, relationship);
        Ok(edge_idx)
    }

    fn get_relationship(&self, edge_idx: petgraph::graph::EdgeIndex) -> Option<&Relationship> {
        self.edge_weight(edge_idx)
    }

    fn update_relationship(
        &mut self,
        edge_idx: petgraph::graph::EdgeIndex,
        relationship: Relationship,
    ) -> Result<(), String> {
        // Сначала проверяем, что связь существует и колонки валидны
        if self.edge_weight(edge_idx).is_none() {
            return Err("Relationship not found".to_string());
        }

        if let Some((from_idx, to_idx)) = self.edge_endpoints(edge_idx) {
            let from_node = self.node_weight(from_idx).unwrap();
            if from_node.find_column(&relationship.from_column).is_none() {
                return Err(format!(
                    "Column '{}' not found in source table",
                    relationship.from_column
                ));
            }

            let to_node = self.node_weight(to_idx).unwrap();
            if to_node.find_column(&relationship.to_column).is_none() {
                return Err(format!(
                    "Column '{}' not found in target table",
                    relationship.to_column
                ));
            }
        }

        // Теперь обновляем связь
        if let Some(edge) = self.edge_weight_mut(edge_idx) {
            *edge = relationship;
            Ok(())
        } else {
            Err("Relationship not found".to_string())
        }
    }

    fn delete_relationship(&mut self, edge_idx: petgraph::graph::EdgeIndex) -> Result<(), String> {
        if self.edge_weight(edge_idx).is_some() {
            self.remove_edge(edge_idx);
            Ok(())
        } else {
            Err("Relationship not found".to_string())
        }
    }

    fn find_relationships_from(
        &self,
        from_table: petgraph::graph::NodeIndex,
    ) -> Vec<(
        petgraph::graph::EdgeIndex,
        petgraph::graph::NodeIndex,
        &Relationship,
    )> {
        let mut result = Vec::new();
        let mut edges = self
            .neighbors_directed(from_table, petgraph::Direction::Outgoing)
            .detach();
        while let Some((edge_idx, target_idx)) = edges.next(self) {
            if let Some(relationship) = self.edge_weight(edge_idx) {
                result.push((edge_idx, target_idx, relationship));
            }
        }
        result
    }

    fn find_relationships_to(
        &self,
        to_table: petgraph::graph::NodeIndex,
    ) -> Vec<(
        petgraph::graph::EdgeIndex,
        petgraph::graph::NodeIndex,
        &Relationship,
    )> {
        let mut result = Vec::new();
        let mut edges = self
            .neighbors_directed(to_table, petgraph::Direction::Incoming)
            .detach();
        while let Some((edge_idx, source_idx)) = edges.next(self) {
            if let Some(relationship) = self.edge_weight(edge_idx) {
                result.push((edge_idx, source_idx, relationship));
            }
        }
        result
    }

    fn find_relationship_by_columns(
        &self,
        from_table: petgraph::graph::NodeIndex,
        to_table: petgraph::graph::NodeIndex,
        from_column: &str,
        to_column: &str,
    ) -> Option<petgraph::graph::EdgeIndex> {
        self.edges_connecting(from_table, to_table)
            .find(|edge_ref| {
                let rel = edge_ref.weight();
                rel.from_column == from_column && rel.to_column == to_column
            })
            .map(|edge_ref| edge_ref.id())
    }
}

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
