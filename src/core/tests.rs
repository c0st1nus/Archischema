#[cfg(test)]
mod tests {
    use crate::core::{
        Column, MySqlDataType, RelationshipOps, RelationshipType, TableNode, TableOps,
    };
    use petgraph::stable_graph::StableGraph;

    #[test]
    fn test_create_column() {
        let mut table = TableNode::new("users");

        let column = Column::new("id", "INT").primary_key();
        table.create_column(column.clone());

        assert_eq!(table.columns.len(), 1);
        assert_eq!(table.columns[0].name, "id");
        assert_eq!(table.columns[0].data_type, "INT");
        assert!(table.columns[0].is_primary_key);
        assert!(!table.columns[0].is_nullable);
    }

    #[test]
    fn test_create_multiple_columns() {
        let mut table = TableNode::new("users");

        table.create_column(Column::new("id", "INT").primary_key());
        table.create_column(Column::new("username", "VARCHAR(50)").not_null().unique());
        table.create_column(Column::new("email", "VARCHAR(255)").not_null().unique());

        assert_eq!(table.columns.len(), 3);
        assert_eq!(table.columns[1].name, "username");
        assert!(table.columns[1].is_unique);
        assert_eq!(table.columns[2].name, "email");
    }

    #[test]
    fn test_read_column() {
        let mut table = TableNode::new("users");
        table.create_column(Column::new("id", "INT").primary_key());
        table.create_column(Column::new("email", "VARCHAR(255)"));

        // Read by index
        let column = table.get_column(0);
        assert!(column.is_some());
        assert_eq!(column.unwrap().name, "id");

        // Read out of bounds
        let column = table.get_column(10);
        assert!(column.is_none());
    }

    #[test]
    fn test_find_column() {
        let mut table = TableNode::new("users");
        table.create_column(Column::new("id", "INT").primary_key());
        table.create_column(Column::new("email", "VARCHAR(255)"));
        table.create_column(Column::new("username", "VARCHAR(50)"));

        // Find existing column
        let result = table.find_column("email");
        assert!(result.is_some());
        let (index, column) = result.unwrap();
        assert_eq!(index, 1);
        assert_eq!(column.name, "email");

        // Find non-existing column
        let result = table.find_column("age");
        assert!(result.is_none());
    }

    #[test]
    fn test_update_column() {
        let mut table = TableNode::new("users");
        table.create_column(Column::new("email", "VARCHAR(255)"));

        // Update existing column
        let updated = Column::new("email", "VARCHAR(320)").not_null().unique();
        let result = table.update_column(0, updated);
        assert!(result.is_ok());

        let column = table.get_column(0).unwrap();
        assert_eq!(column.data_type, "VARCHAR(320)");
        assert!(!column.is_nullable);
        assert!(column.is_unique);

        // Update non-existing column
        let result = table.update_column(10, Column::new("test", "INT"));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_column() {
        let mut table = TableNode::new("users");
        table.create_column(Column::new("id", "INT"));
        table.create_column(Column::new("email", "VARCHAR(255)"));
        table.create_column(Column::new("username", "VARCHAR(50)"));

        assert_eq!(table.columns.len(), 3);

        // Delete middle column
        let result = table.delete_column(1);
        assert!(result.is_ok());
        let deleted = result.unwrap();
        assert_eq!(deleted.name, "email");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[1].name, "username");

        // Delete out of bounds
        let result = table.delete_column(10);
        assert!(result.is_err());
    }

    #[test]
    fn test_move_column() {
        let mut table = TableNode::new("users");
        table.create_column(Column::new("id", "INT"));
        table.create_column(Column::new("email", "VARCHAR(255)"));
        table.create_column(Column::new("username", "VARCHAR(50)"));
        table.create_column(Column::new("created_at", "TIMESTAMP"));

        // Move column from index 3 to index 1
        let result = table.move_column(3, 1);
        assert!(result.is_ok());

        assert_eq!(table.columns[0].name, "id");
        assert_eq!(table.columns[1].name, "created_at");
        assert_eq!(table.columns[2].name, "email");
        assert_eq!(table.columns[3].name, "username");

        // Move out of bounds
        let result = table.move_column(10, 1);
        assert!(result.is_err());

        let result = table.move_column(1, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_column_validate_name() {
        // Valid names
        assert!(Column::validate_name("id").is_ok());
        assert!(Column::validate_name("user_id").is_ok());
        assert!(Column::validate_name("email_address").is_ok());
        assert!(Column::validate_name("_internal").is_ok());
        assert!(Column::validate_name("field123").is_ok());

        // Invalid names
        assert!(Column::validate_name("").is_err()); // empty
        assert!(Column::validate_name("123field").is_err()); // starts with number
        assert!(Column::validate_name("user-id").is_err()); // invalid character
        assert!(Column::validate_name("user@email").is_err()); // invalid character
        assert!(Column::validate_name("user id").is_err()); // space

        // Too long (> 64 characters)
        let long_name = "a".repeat(65);
        assert!(Column::validate_name(&long_name).is_err());
    }

    #[test]
    fn test_column_validate_data_type() {
        assert!(Column::validate_data_type("INT").is_ok());
        assert!(Column::validate_data_type("VARCHAR(255)").is_ok());
        assert!(Column::validate_data_type("").is_err()); // empty
    }

    #[test]
    fn test_column_primary_key() {
        let column = Column::new("id", "INT").primary_key();

        assert!(column.is_primary_key);
        assert!(!column.is_nullable); // PK should be NOT NULL
    }

    #[test]
    fn test_column_not_null() {
        let column = Column::new("email", "VARCHAR(255)").not_null();

        assert!(!column.is_nullable);
        assert!(!column.is_primary_key);
    }

    #[test]
    fn test_column_unique() {
        let column = Column::new("email", "VARCHAR(255)").unique();

        assert!(column.is_unique);
    }

    #[test]
    fn test_column_with_default() {
        let column = Column::new("status", "VARCHAR(20)").with_default("'active'");

        assert_eq!(column.default_value, Some("'active'".to_string()));
    }

    #[test]
    fn test_column_chaining() {
        let column = Column::new("email", "VARCHAR(255)")
            .not_null()
            .unique()
            .with_default("'test@example.com'");

        assert_eq!(column.name, "email");
        assert_eq!(column.data_type, "VARCHAR(255)");
        assert!(!column.is_nullable);
        assert!(column.is_unique);
        assert_eq!(column.default_value, Some("'test@example.com'".to_string()));
    }

    #[test]
    fn test_mysql_data_type_to_string() {
        assert_eq!(MySqlDataType::Int.to_string(), "INT");
        assert_eq!(MySqlDataType::BigInt.to_string(), "BIGINT");
        assert_eq!(MySqlDataType::Varchar(255).to_string(), "VARCHAR(255)");
        assert_eq!(MySqlDataType::Char(10).to_string(), "CHAR(10)");
        assert_eq!(MySqlDataType::Text.to_string(), "TEXT");
        assert_eq!(MySqlDataType::Decimal(10, 2).to_string(), "DECIMAL(10,2)");
        assert_eq!(MySqlDataType::DateTime.to_string(), "DATETIME");
        assert_eq!(MySqlDataType::Timestamp.to_string(), "TIMESTAMP");
        assert_eq!(MySqlDataType::Boolean.to_string(), "BOOLEAN");
        assert_eq!(MySqlDataType::Json.to_string(), "JSON");
    }

    #[test]
    fn test_mysql_data_type_enum() {
        let values = vec![
            "active".to_string(),
            "inactive".to_string(),
            "pending".to_string(),
        ];
        let enum_type = MySqlDataType::Enum(values);
        assert_eq!(enum_type.to_string(), "ENUM('active','inactive','pending')");
    }

    #[test]
    fn test_mysql_data_type_set() {
        let values = vec![
            "read".to_string(),
            "write".to_string(),
            "execute".to_string(),
        ];
        let set_type = MySqlDataType::Set(values);
        assert_eq!(set_type.to_string(), "SET('read','write','execute')");
    }

    #[test]
    fn test_mysql_data_type_all_types() {
        let types = MySqlDataType::all_types();

        assert!(types.contains(&"INT"));
        assert!(types.contains(&"VARCHAR"));
        assert!(types.contains(&"TEXT"));
        assert!(types.contains(&"DATETIME"));
        assert!(types.contains(&"BOOLEAN"));
        assert!(types.contains(&"JSON"));

        // Should have all major types
        assert!(types.len() >= 20);
    }

    #[test]
    fn test_table_builder_pattern() {
        let table = TableNode::new("users")
            .with_position(100.0, 200.0)
            .add_column(Column::new("id", "INT").primary_key())
            .add_column(Column::new("username", "VARCHAR(50)").not_null().unique())
            .add_column(Column::new("email", "VARCHAR(255)").not_null().unique());

        assert_eq!(table.name, "users");
        assert_eq!(table.position, (100.0, 200.0));
        assert_eq!(table.columns.len(), 3);
        assert_eq!(table.columns[0].name, "id");
        assert!(table.columns[0].is_primary_key);
    }

    #[test]
    fn test_get_column_mut() {
        let mut table = TableNode::new("users");
        table.create_column(Column::new("email", "VARCHAR(255)"));

        // Modify column through mutable reference
        if let Some(column) = table.get_column_mut(0) {
            column.is_nullable = false;
            column.is_unique = true;
        }

        let column = table.get_column(0).unwrap();
        assert!(!column.is_nullable);
        assert!(column.is_unique);
    }

    #[test]
    fn test_table_crud_workflow() {
        // Create table
        let mut table = TableNode::new("products");

        // Create columns (C in CRUD)
        table.create_column(Column::new("id", "BIGINT").primary_key());
        table.create_column(Column::new("name", "VARCHAR(255)").not_null());
        table.create_column(Column::new("price", "DECIMAL(10,2)").not_null());
        table.create_column(Column::new("description", "TEXT"));

        assert_eq!(table.columns.len(), 4);

        // Read columns (R in CRUD)
        let id_col = table.get_column(0).unwrap();
        assert_eq!(id_col.name, "id");

        let (idx, name_col) = table.find_column("name").unwrap();
        assert_eq!(idx, 1);
        assert_eq!(name_col.data_type, "VARCHAR(255)");

        // Update column (U in CRUD)
        let updated_price = Column::new("price", "DECIMAL(12,2)")
            .not_null()
            .with_default("0.00");
        table.update_column(2, updated_price).unwrap();

        let price_col = table.get_column(2).unwrap();
        assert_eq!(price_col.data_type, "DECIMAL(12,2)");
        assert_eq!(price_col.default_value, Some("0.00".to_string()));

        // Delete column (D in CRUD)
        let deleted = table.delete_column(3).unwrap();
        assert_eq!(deleted.name, "description");
        assert_eq!(table.columns.len(), 3);
    }

    #[test]
    fn test_get_base_type() {
        let col1 = Column::new("id", "INT");
        assert_eq!(col1.get_base_type(), "INT");

        let col2 = Column::new("name", "VARCHAR(255)");
        assert_eq!(col2.get_base_type(), "VARCHAR");

        let col3 = Column::new("price", "DECIMAL(10,2)");
        assert_eq!(col3.get_base_type(), "DECIMAL");

        let col4 = Column::new("data", "TEXT");
        assert_eq!(col4.get_base_type(), "TEXT");
    }

    #[test]
    fn test_is_type_compatible_with_exact_match() {
        let col1 = Column::new("id", "INT");
        let col2 = Column::new("user_id", "INT");
        assert!(col1.is_type_compatible_with(&col2));
    }

    #[test]
    fn test_is_type_compatible_with_integer_types() {
        let col_int = Column::new("id", "INT");
        let col_bigint = Column::new("big_id", "BIGINT");
        let col_smallint = Column::new("small_id", "SMALLINT");
        let col_tinyint = Column::new("tiny_id", "TINYINT");

        assert!(col_int.is_type_compatible_with(&col_bigint));
        assert!(col_int.is_type_compatible_with(&col_smallint));
        assert!(col_int.is_type_compatible_with(&col_tinyint));
        assert!(col_bigint.is_type_compatible_with(&col_int));
    }

    #[test]
    fn test_is_type_compatible_with_string_types() {
        let col_varchar = Column::new("name", "VARCHAR(255)");
        let col_text = Column::new("description", "TEXT");
        let col_char = Column::new("code", "CHAR(10)");

        assert!(col_varchar.is_type_compatible_with(&col_text));
        assert!(col_varchar.is_type_compatible_with(&col_char));
        assert!(col_text.is_type_compatible_with(&col_varchar));
    }

    #[test]
    fn test_is_type_compatible_with_float_types() {
        let col_float = Column::new("price", "FLOAT");
        let col_double = Column::new("total", "DOUBLE");
        let col_decimal = Column::new("amount", "DECIMAL(10,2)");

        assert!(col_float.is_type_compatible_with(&col_double));
        assert!(col_float.is_type_compatible_with(&col_decimal));
        assert!(col_decimal.is_type_compatible_with(&col_float));
    }

    #[test]
    fn test_is_type_compatible_with_datetime_types() {
        let col_date = Column::new("birth_date", "DATE");
        let col_datetime = Column::new("created_at", "DATETIME");
        let col_timestamp = Column::new("updated_at", "TIMESTAMP");

        assert!(col_date.is_type_compatible_with(&col_datetime));
        assert!(col_datetime.is_type_compatible_with(&col_timestamp));
        assert!(col_timestamp.is_type_compatible_with(&col_date));
    }

    #[test]
    fn test_is_type_compatible_with_incompatible_types() {
        let col_int = Column::new("id", "INT");
        let col_varchar = Column::new("name", "VARCHAR(255)");
        let col_date = Column::new("created", "DATE");

        assert!(!col_int.is_type_compatible_with(&col_varchar));
        assert!(!col_int.is_type_compatible_with(&col_date));
        assert!(!col_varchar.is_type_compatible_with(&col_date));
    }

    #[test]
    fn test_relationship_ops_create() {
        let mut graph = StableGraph::new();

        let mut users = TableNode::new("users");
        users.create_column(Column::new("id", "INT").primary_key());
        users.create_column(Column::new("name", "VARCHAR(255)"));

        let mut posts = TableNode::new("posts");
        posts.create_column(Column::new("id", "INT").primary_key());
        posts.create_column(Column::new("user_id", "INT"));

        let users_idx = graph.add_node(users);
        let posts_idx = graph.add_node(posts);

        use crate::core::Relationship;
        let rel = Relationship::new(
            "fk_posts_users",
            RelationshipType::OneToMany,
            "user_id",
            "id",
        );

        let result = graph.create_relationship(posts_idx, users_idx, rel);
        assert!(result.is_ok());
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_relationship_ops_duplicate_prevention() {
        let mut graph = StableGraph::new();

        let mut users = TableNode::new("users");
        users.create_column(Column::new("id", "INT").primary_key());

        let mut posts = TableNode::new("posts");
        posts.create_column(Column::new("user_id", "INT"));

        let users_idx = graph.add_node(users);
        let posts_idx = graph.add_node(posts);

        use crate::core::Relationship;
        let rel1 = Relationship::new(
            "fk_posts_users",
            RelationshipType::OneToMany,
            "user_id",
            "id",
        );

        let rel2 = Relationship::new(
            "fk_posts_users_duplicate",
            RelationshipType::OneToMany,
            "user_id",
            "id",
        );

        graph
            .create_relationship(posts_idx, users_idx, rel1)
            .unwrap();
        let result = graph.create_relationship(posts_idx, users_idx, rel2);

        assert!(result.is_err());
        assert_eq!(graph.edge_count(), 1);
    }

    // ===== TableOps Tests =====

    #[test]
    fn test_create_table() {
        let mut graph = StableGraph::new();

        let result = graph.create_table("users", (100.0, 200.0));
        assert!(result.is_ok());

        let node_idx = result.unwrap();
        let table = graph.node_weight(node_idx).unwrap();
        assert_eq!(table.name, "users");
        assert_eq!(table.position, (100.0, 200.0));
        assert_eq!(table.columns.len(), 0);
    }

    #[test]
    fn test_create_table_empty_name() {
        let mut graph = StableGraph::new();

        let result = graph.create_table("", (0.0, 0.0));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Name cannot be empty");
    }

    #[test]
    fn test_create_table_duplicate_name() {
        let mut graph = StableGraph::new();

        graph.create_table("users", (0.0, 0.0)).unwrap();
        let result = graph.create_table("users", (100.0, 100.0));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Table 'users' already exists");
    }

    #[test]
    fn test_create_table_auto() {
        let mut graph = StableGraph::new();

        let node_idx1 = graph.create_table_auto((0.0, 0.0));
        let table1 = graph.node_weight(node_idx1).unwrap();
        assert_eq!(table1.name, "new_table");

        let node_idx2 = graph.create_table_auto((100.0, 100.0));
        let table2 = graph.node_weight(node_idx2).unwrap();
        assert_eq!(table2.name, "new_table_2");

        let node_idx3 = graph.create_table_auto((200.0, 200.0));
        let table3 = graph.node_weight(node_idx3).unwrap();
        assert_eq!(table3.name, "new_table_3");
    }

    #[test]
    fn test_rename_table() {
        let mut graph = StableGraph::new();

        let node_idx = graph.create_table("users", (0.0, 0.0)).unwrap();
        let result = graph.rename_table(node_idx, "customers");

        assert!(result.is_ok());
        let table = graph.node_weight(node_idx).unwrap();
        assert_eq!(table.name, "customers");
    }

    #[test]
    fn test_rename_table_empty_name() {
        let mut graph = StableGraph::new();
        let users = graph.create_table("users", (0.0, 0.0)).unwrap();

        let result = graph.rename_table(users, "");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Name cannot be empty");
    }

    #[test]
    fn test_rename_table_duplicate_name() {
        let mut graph = StableGraph::new();

        graph.create_table("users", (0.0, 0.0)).unwrap();
        let node_idx2 = graph.create_table("posts", (100.0, 100.0)).unwrap();

        let result = graph.rename_table(node_idx2, "users");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Table 'users' already exists");
    }

    #[test]
    fn test_rename_table_same_name() {
        let mut graph = StableGraph::new();

        let node_idx = graph.create_table("users", (0.0, 0.0)).unwrap();
        let result = graph.rename_table(node_idx, "users");

        assert!(result.is_ok());
        let table = graph.node_weight(node_idx).unwrap();
        assert_eq!(table.name, "users");
    }

    #[test]
    fn test_delete_table() {
        let mut graph = StableGraph::new();

        let node_idx = graph.create_table("users", (0.0, 0.0)).unwrap();
        assert_eq!(graph.node_count(), 1);

        let result = graph.delete_table(node_idx);
        assert!(result.is_ok());

        let deleted_table = result.unwrap();
        assert_eq!(deleted_table.name, "users");
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_delete_table_with_relationships() {
        let mut graph = StableGraph::new();

        let users_idx = graph.create_table("users", (0.0, 0.0)).unwrap();
        let posts_idx = graph.create_table("posts", (100.0, 100.0)).unwrap();

        // Add columns for relationship
        graph
            .node_weight_mut(users_idx)
            .unwrap()
            .create_column(Column::new("id", "INTEGER").primary_key());
        graph
            .node_weight_mut(posts_idx)
            .unwrap()
            .create_column(Column::new("user_id", "INTEGER"));

        let rel = crate::core::Relationship::new(
            "user_posts",
            RelationshipType::OneToMany,
            "id",
            "user_id",
        );
        graph
            .create_relationship(users_idx, posts_idx, rel)
            .unwrap();

        assert_eq!(graph.edge_count(), 1);

        let result = graph.delete_table(users_idx);
        assert!(result.is_ok());

        // Relationships are automatically removed when node is deleted
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_table_exists() {
        let mut graph = StableGraph::new();

        assert!(!graph.table_exists("users"));

        graph.create_table("users", (0.0, 0.0)).unwrap();
        assert!(graph.table_exists("users"));
        assert!(!graph.table_exists("posts"));
    }

    #[test]
    fn test_find_table_by_name() {
        let mut graph = StableGraph::new();

        let node_idx = graph.create_table("users", (0.0, 0.0)).unwrap();

        let found = graph.find_table_by_name("users");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), node_idx);

        let not_found = graph.find_table_by_name("posts");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_generate_unique_table_name() {
        let mut graph = StableGraph::new();

        let name1 = graph.generate_unique_table_name("my_table");
        assert_eq!(name1, "my_table");

        graph.create_table("my_table", (0.0, 0.0)).unwrap();

        let name2 = graph.generate_unique_table_name("my_table");
        assert_eq!(name2, "my_table_2");

        graph.create_table("my_table_2", (0.0, 0.0)).unwrap();

        let name3 = graph.generate_unique_table_name("my_table");
        assert_eq!(name3, "my_table_3");
    }

    #[test]
    fn test_table_operations_workflow() {
        let mut graph = StableGraph::new();

        // Create multiple tables
        let users_idx = graph.create_table_auto((0.0, 0.0));
        let posts_idx = graph.create_table_auto((100.0, 100.0));
        let comments_idx = graph.create_table_auto((200.0, 200.0));

        assert_eq!(graph.node_count(), 3);

        // Rename tables
        graph.rename_table(users_idx, "users").unwrap();
        graph.rename_table(posts_idx, "posts").unwrap();
        graph.rename_table(comments_idx, "comments").unwrap();

        // Check all exist
        assert!(graph.table_exists("users"));
        assert!(graph.table_exists("posts"));
        assert!(graph.table_exists("comments"));

        // Delete one table
        graph.delete_table(comments_idx).unwrap();
        assert_eq!(graph.node_count(), 2);
        assert!(!graph.table_exists("comments"));

        // Can now create a new table with the same name
        let new_comments_idx = graph.create_table("comments", (300.0, 300.0));
        assert!(new_comments_idx.is_ok());
    }
}
