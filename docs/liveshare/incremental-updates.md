# Инкрементальные обновления в LiveShare

## Обзор

Система инкрементальных обновлений позволяет значительно снизить нагрузку на сеть путем отправки только измененных элементов графа вместо полного состояния при каждом изменении.

## Архитектура

### Компоненты

1. **BroadcastManager** (`src/core/liveshare/broadcast_manager.rs`)
   - Отслеживает версии элементов для каждого пользователя
   - Определяет, какие обновления нужно отправить
   - Управляет периодическими full sync (по умолчанию каждые 20 секунд)

2. **Версионирование элементов** (`src/core/liveshare/protocol.rs`)
   - `TableSnapshot.version: u64` - версия таблицы
   - `RelationshipSnapshot.version: u64` - версия связи
   - Версия инкрементируется при каждом изменении элемента

3. **Room интеграция** (`src/core/liveshare/room.rs`)
   - Автоматическая регистрация пользователей
   - Методы для отправки инкрементальных и полных обновлений

## Использование

### 1. Отправка инкрементальных обновлений

При изменении графа на клиенте:

```rust
// В WebSocket обработчике при получении GraphOp
async fn handle_graph_op(&self, op: GraphOperation) -> Result<(), String> {
    if let Some(ref room) = self.room {
        // 1. Применить операцию к локальному состоянию
        let snapshot = apply_operation_to_state(op)?;
        
        // 2. Инкрементировать версии измененных элементов
        // (это должно делаться на стороне, которая управляет состоянием)
        
        // 3. Отправить инкрементальное обновление
        room.broadcast_incremental_update(&self.user_id, &snapshot).await;
    }
    Ok(())
}
```

### 2. Отправка полного состояния новому пользователю

При подключении нового пользователя:

```rust
async fn handle_auth(&mut self, auth: AuthMessage) -> Result<(), String> {
    // ... authentication logic ...
    
    if let Some(ref room) = self.room {
        // Получить текущее состояние графа
        let snapshot = get_current_graph_state();
        
        // Отправить полное состояние новому пользователю
        room.send_full_graph_state(&user_id, &snapshot).await;
    }
    
    Ok(())
}
```

### 3. Периодический full sync

Система автоматически отправляет полное состояние каждому пользователю каждые 20 секунд:

```rust
// Это происходит автоматически в broadcast_incremental_update
// Но можно настроить интервал при создании Room:
let broadcast_manager = BroadcastManager::with_interval(Duration::from_secs(30));
```

## Протокол обновлений

### Инкрементальное обновление

```json
{
  "GraphState": {
    "state": {
      "tables": [
        {
          "node_id": 1,
          "name": "users",
          "position": [100.0, 200.0],
          "columns": [...],
          "version": 5  // Изменилась с версии 4
        }
      ],
      "relationships": []
    },
    "target_user_id": "uuid-of-recipient"
  }
}
```

**Важно:** В инкрементальном обновлении отправляются только элементы, чья версия больше последней отправленной этому пользователю.

### Полное обновление

```json
{
  "GraphState": {
    "state": {
      "tables": [
        // ВСЕ таблицы, независимо от версий
      ],
      "relationships": [
        // ВСЕ связи, независимо от версий
      ]
    },
    "target_user_id": "uuid-of-recipient"
  }
}
```

## Управление версиями

### На клиенте (TypeScript)

```typescript
interface TableState {
  nodeId: number;
  name: string;
  position: [number, number];
  columns: Column[];
  version: number;  // Добавлено
}

class GraphState {
  tables: Map<number, TableState> = new Map();
  relationships: Map<number, RelationshipState> = new Map();
  
  // При локальном изменении - инкрементировать версию
  updateTable(nodeId: number, updates: Partial<TableState>) {
    const table = this.tables.get(nodeId);
    if (table) {
      table.version++;  // Важно!
      Object.assign(table, updates);
      
      // Отправить на сервер
      this.sendGraphUpdate();
    }
  }
  
  // При получении обновления с сервера
  applyServerUpdate(snapshot: GraphStateSnapshot) {
    for (const table of snapshot.tables) {
      const existing = this.tables.get(table.node_id);
      
      // Применять только если версия новее
      if (!existing || table.version > existing.version) {
        this.tables.set(table.node_id, table);
      }
    }
  }
}
```

### На сервере (Rust)

```rust
// При создании нового элемента
let table = TableSnapshot {
    node_id: 1,
    name: "users".to_string(),
    position: (0.0, 0.0),
    columns: vec![],
    version: 1,  // Начальная версия
};

// При изменении элемента
fn update_table(table: &mut TableSnapshot, new_name: String) {
    table.name = new_name;
    table.version += 1;  // Инкрементировать!
}
```

## Оптимизация пропускной способности

### Пример расчета

**Без инкрементальных обновлений:**
- Полное состояние: 10 таблиц × 2KB = 20KB
- Частота: 10 обновлений/сек
- Пропускная способность: 200KB/сек на пользователя
- Для 10 пользователей: 2MB/сек

**С инкрементальными обновлениями:**
- Типичное изменение: 1 таблица = 2KB
- Частота: 10 обновлений/сек
- Пропускная способность: 20KB/сек на пользователя
- Для 10 пользователей: 200KB/сек
- **Экономия: 90%**

## Обработка конфликтов версий

### Стратегия "Last Write Wins"

Текущая реализация использует простую стратегию:

```rust
// Клиент принимает обновление если версия больше
if server_version > local_version {
    apply_update(server_data);
}
```

### Будущие улучшения (TODO)

- [ ] Lamport timestamps для полного упорядочивания
- [ ] Vector clocks для обнаружения конфликтов
- [ ] CRDT для автоматического разрешения конфликтов

## Тестирование

### Unit тесты

```bash
# Тесты BroadcastManager
cargo test --lib liveshare::broadcast_manager

# Тесты протокола с версиями
cargo test --lib liveshare::protocol
```

### Integration тесты

```rust
#[tokio::test]
async fn test_incremental_updates() {
    let room = Room::new(/*...*/);
    
    // Симулировать подключение пользователя
    room.add_user(user1, "Alice".to_string()).unwrap();
    
    // Первое обновление - должен получить полное состояние
    let snapshot1 = create_snapshot_with_versions();
    room.broadcast_incremental_update(&sender_id, &snapshot1).await;
    assert!(received_full_state());
    
    // Второе обновление - только изменения
    let mut snapshot2 = snapshot1.clone();
    snapshot2.tables[0].version += 1;
    room.broadcast_incremental_update(&sender_id, &snapshot2).await;
    assert!(received_only_changed_table());
}
```

## Мониторинг

### Метрики для отслеживания

1. **Размер сообщений**
   - Средний размер инкрементального обновления
   - Средний размер полного обновления
   - Соотношение инкрементальных к полным

2. **Частота обновлений**
   - Количество инкрементальных обновлений/сек
   - Количество полных синхронизаций/сек
   - Количество пропущенных обновлений

3. **Состояние BroadcastManager**
   - Количество отслеживаемых пользователей
   - Количество отслеживаемых элементов на пользователя
   - Использование памяти

## Troubleshooting

### Проблема: Пользователь не получает обновления

**Проверить:**
1. Зарегистрирован ли пользователь в BroadcastManager?
   ```rust
   room.broadcast_manager.has_user(user_id)
   ```
2. Корректно ли инкрементируются версии?
3. Не истек ли интервал full sync?

### Проблема: Слишком частые full sync

**Решение:**
```rust
// Увеличить интервал
let manager = BroadcastManager::with_interval(Duration::from_secs(60));
```

### Проблема: Десинхронизация состояния

**Решение:**
```rust
// Принудительный сброс состояния пользователя
room.reset_user_broadcast_state(user_id).await;
// Следующее обновление будет полным
```

## См. также

- [API документация BroadcastManager](../../src/core/liveshare/broadcast_manager.rs)
- [Протокол LiveShare](../../src/core/liveshare/protocol.rs)
- [Room управление](../../src/core/liveshare/room.rs)
- [WebSocket обработчик](../../src/core/liveshare/websocket.rs)