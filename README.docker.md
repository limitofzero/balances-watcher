# Docker Setup для Token Balances Updater

## Быстрый старт

### 1. Сборка образа
```bash
docker build -t token-balances-updater .
```

### 2. Запуск с переменными окружения

#### Вариант A: Через docker-compose (рекомендуется)
```bash
# Создайте .env файл (см. .env.example)
docker-compose up -d
```

#### Вариант B: Через docker run
```bash
docker run -d \
  -p 4000:8080 \
  -e HTTP_BIND=0.0.0.0:8080 \
  -e ETH_RPC=https://eth.llamarpc.com \
  -e ARBITRUM_RPC=https://arb1.arbitrum.io/rpc \
  --name token-balances-updater \
  token-balances-updater
```

#### Вариант C: Через .env файл
```bash
# Создайте .env файл с переменными:
# ETH_RPC=https://eth.llamarpc.com
# ARBITRUM_RPC=https://arb1.arbitrum.io/rpc
# HTTP_BIND=0.0.0.0:8080

docker run -d \
  -p 4000:8080 \
  --env-file .env \
  --name token-balances-updater \
  token-balances-updater
```

## Оптимизации для быстрой пересборки

### 1. Multi-stage build
Dockerfile использует multi-stage build:
- **Stage 1 (builder)**: Компилирует Rust код
- **Stage 2 (runtime)**: Минимальный образ только с бинарником

Это уменьшает финальный размер образа в ~10 раз.

### 2. Кэширование зависимостей Cargo
Dockerfile копирует `Cargo.toml` и `Cargo.lock` отдельно от кода:
- Если зависимости не изменились, Docker использует кэш
- Пересборка занимает секунды вместо минут

### 3. .dockerignore
Исключает ненужные файлы из контекста сборки:
- `/target` директория (build artifacts)
- Git файлы
- IDE конфигурации

## Переменные окружения

Приложение использует следующие переменные (через clap):

| Переменная | Описание | По умолчанию |
|-----------|----------|--------------|
| `HTTP_BIND` | Адрес и порт для HTTP сервера | `0.0.0.0:8080` |
| `ETH_RPC` | URL Ethereum RPC ноды | (пусто) |
| `ARBITRUM_RPC` | URL Arbitrum RPC ноды | (пусто) |

## Полезные команды

```bash
# Просмотр логов
docker-compose logs -f

# Остановка
docker-compose down

# Пересборка после изменений кода
docker-compose build --no-cache
docker-compose up -d

# Вход в контейнер (для отладки)
docker exec -it token-balances-updater /bin/bash
```

## Оптимизация размера образа

Финальный образ использует:
- `debian:bookworm-slim` (минимальный базовый образ)
- Только runtime зависимости (SSL библиотеки)
- Непривилегированный пользователь для безопасности
- Размер: ~50-80MB (вместо ~1GB с полным Rust toolchain)

