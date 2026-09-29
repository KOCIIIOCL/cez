#!/usr/bin/env python3
"""
FREE-HOSTS // TUI & Parser для поиска бесплатной инфраструктуры без кредитных карт (No CC).
Категории: VPS & Shell, Базы Данных, S3 Хранилища, PaaS & Хостинг.
Сортировки: Скрытый алмаз (Мощный + Непопулярный), Самое мощное, Самое популярное, Underground.
"""

import os
import sys
import json
import re
import urllib.request
import subprocess
import curses
from pathlib import Path

# ============================================================================
# Встроенная выверенная база сервисов (100% БЕЗ КРЕДИТНЫХ КАРТ)
# ============================================================================

CURATED_HOSTS = [
    # --- VPS / Shell / Compute ---
    {
        "name": "Serv00",
        "category": "VPS",
        "type_desc": "FreeBSD 14 Cloud Shell & Web Hosting",
        "ram_mb": 512,
        "cpu_cores": 1.0,
        "disk_mb": 3072,  # 3 GB
        "bandwidth_gb": 1000,
        "has_ssh": True,
        "has_containers": False,
        "always_on": True,
        "popularity": 3,
        "verification": "Только Email (Без карт и номеров)",
        "url": "https://www.serv00.com/",
        "sleep_policy": "Всегда активен (требует вход по SSH раз в 3 месяца)",
        "pros": [
            "Полный доступ по SSH, ZFS хранилище 3 ГБ",
            "Свой порт для входящих подключений (любой софт/демон)",
            "Поддержка cron, tmux, screen, Python, Node.js, Go, Rust, C, Cez",
            "Бесплатно выдается на 10 лет без скрытых списаний",
            "Rootless менеджер пакетов (Devil)"
        ],
        "cons": [
            "ОС FreeBSD, а не Linux (но есть Linux binary compat)",
            "Периодически регистрация закрыта из-за наплыва (нужно ловить волну)",
            "512 МБ лимит памяти на аккаунт"
        ],
        "notes": "Абсолютный топ среди скрытых алмазов. Идеально под ботов, прокси, Cez-компилятор и фоновые демоны."
    },
    {
        "name": "GitHub Codespaces",
        "category": "VPS",
        "type_desc": "Ubuntu 24.04 Cloud VM (Full Sudo / Docker)",
        "ram_mb": 8192,   # 8 GB RAM
        "cpu_cores": 2.0,
        "disk_mb": 32768, # 32 GB NVMe
        "bandwidth_gb": 100,
        "has_ssh": True,
        "has_containers": True,
        "always_on": False,
        "popularity": 7,
        "verification": "GitHub Аккаунт (Без карт)",
        "url": "https://github.com/features/codespaces",
        "sleep_policy": "Засыпает при простое (120 core-hours/мес = 60ч работы в месяц)",
        "pros": [
            "Полноценная Ubuntu машина с полным sudo / root правами",
            "8 ГБ RAM, 2 vCPU, 32 ГБ скоростного диска",
            "Предустановленный Docker, Python, Node, C++, Rust, Git",
            "Доступ по SSH (через 'gh cs ssh') или прямо из браузера/VS Code",
            "Никаких кредитных карт: лимит $0 залочен по умолчанию"
        ],
        "cons": [
            "Не для постоянных 24/7 демонов (лимит 60 часов работы в месяц)",
            "Засыпает через 30 минут бездействия"
        ],
        "notes": "Лучший бесплатный сервер для компиляции проектов, тестов и Docker без ввода карты."
    },
    {
        "name": "CT8.pl",
        "category": "VPS",
        "type_desc": "FreeBSD Cloud Shell (Сестра Serv00)",
        "ram_mb": 512,
        "cpu_cores": 1.0,
        "disk_mb": 3072,
        "bandwidth_gb": 1000,
        "has_ssh": True,
        "has_containers": False,
        "always_on": True,
        "popularity": 2,
        "verification": "Только Email (Без карт)",
        "url": "https://www.ct8.pl/",
        "sleep_policy": "Работает 24/7 (заход по SSH раз в 3 месяца)",
        "pros": [
            "Аналог Serv00 от тех же разработчиков (Польша)",
            "Полный SSH доступ, открытые порты, cron, tmux",
            "3 ГБ диска, MySQL, PostgreSQL, веб-сервер"
        ],
        "cons": [
            "Регистрация открывается партиями",
            "ОС FreeBSD"
        ],
        "notes": "Если на Serv00 закончились слоты — чекай CT8, железо и возможности идентичны."
    },
    {
        "name": "Koyeb",
        "category": "VPS",
        "type_desc": "Serverless Micro-VM & Docker Containers",
        "ram_mb": 512,
        "cpu_cores": 0.5,
        "disk_mb": 2048,
        "bandwidth_gb": 100,
        "has_ssh": False,
        "has_containers": True,
        "always_on": True,
        "popularity": 6,
        "verification": "GitHub / Email (Карта НЕ требуется на тарифе Free)",
        "url": "https://www.koyeb.com/",
        "sleep_policy": "Всегда активен (550 бесплатных часов в месяц = 1 сервис 24/7)",
        "pros": [
            "Работает на быстрых MicroVM (Firecracker)",
            "Деплой прямо из GitHub или Docker Hub",
            "Встроенная бесплатная база Postgres",
            "Глобальная сеть CDN и SSL"
        ],
        "cons": [
            "Только 1 инстанс в бесплатном тарифе",
            "Нет сырого SSH"
        ],
        "notes": "Современная замена старого Heroku. Отличный раннер для веб-приложений."
    },
    {
        "name": "AlwaysData",
        "category": "VPS",
        "type_desc": "Cloud Hosting с SSH и Cron",
        "ram_mb": 512,
        "cpu_cores": 1.0,
        "disk_mb": 1024,
        "bandwidth_gb": 100,
        "has_ssh": True,
        "has_containers": False,
        "always_on": True,
        "popularity": 4,
        "verification": "Email (Без карты)",
        "url": "https://www.alwaysdata.com/",
        "sleep_policy": "Всегда включен",
        "pros": [
            "Честный SSH, WebDAV, FTP, cron",
            "1 ГБ диска под любые файлы",
            "Поддержка C, Python, Go, Rust, Node.js, PHP, Ruby, Elixir",
            "Встроенные базы MySQL и PostgreSQL"
        ],
        "cons": [
            "Трафик ограничен 100 МБ в сутки на высоких скоростях, далее шейпинг",
            "Французский хостинг со строгой политикой спама"
        ],
        "notes": "Очень уютный и стабильный европейский хостинг с честным шеллом."
    },
    {
        "name": "Render (Web & Workers)",
        "category": "PaaS",
        "type_desc": "Cloud Application Platform",
        "ram_mb": 512,
        "cpu_cores": 0.5,
        "disk_mb": 1024,
        "bandwidth_gb": 100,
        "has_ssh": False,
        "has_containers": True,
        "always_on": False,
        "popularity": 9,
        "verification": "GitHub / GitLab / Email (Без карты)",
        "url": "https://render.com/",
        "sleep_policy": "Засыпает через 15 мин простоя (холодный старт ~30-50 сек)",
        "pros": [
            "Деплой Dockerfile или Git репозитория в 1 клик",
            "Качественная инфраструктура и CDN",
            "Автоматический HTTPS на кастомных доменах"
        ],
        "cons": [
            "Засыпает при отсутствии входящих запросов",
            "Лимит 750 часов в месяц"
        ],
        "notes": "Один из самых популярных PaaS сервисов. Идеален для пет-проектов и API."
    },
    {
        "name": "Hugging Face (Static)",
        "category": "PaaS",
        "type_desc": "Static Web Hosting & Documentation",
        "ram_mb": 0,
        "cpu_cores": 0.0,
        "disk_mb": 10240, # 10 GB storage
        "bandwidth_gb": 500,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 8,
        "verification": "GitHub / Почта (Без карты)",
        "url": "https://huggingface.co/spaces",
        "sleep_policy": "Всегда активен (статический CDN)",
        "pros": [
            "Бесплатный хостинг HTML/JS/CSS сайтов и документации",
            "Публичный HTTPS URL вида *.hf.space",
            "Управление через Git push"
        ],
        "cons": [
            "Только статический фронтенд",
            "Docker и Gradio недавно закрыли за платной подпиской PRO"
        ],
        "notes": "Подходит для сайта, документации или блога проекта. Вычислительные контейнеры теперь только в Pro."
    },
    {
        "name": "Glitch",
        "category": "PaaS",
        "type_desc": "Interactive Fullstack Node/Linux Container",
        "ram_mb": 512,
        "cpu_cores": 0.5,
        "disk_mb": 200,
        "bandwidth_gb": 50,
        "has_ssh": False,
        "has_containers": True,
        "always_on": False,
        "popularity": 7,
        "verification": "GitHub / Google / Email (Без карты)",
        "url": "https://glitch.com/",
        "sleep_policy": "Засыпает через 5 минут простоя",
        "pros": [
            "Встроенный веб-терминал с полным доступом к bash",
            "Мгновенный онлайн-редактор кода",
            "Поддержка любых Node.js / Python / C бинарников"
        ],
        "cons": [
            "Мало места на диске (200 МБ)",
            "Быстро засыпает без стороннего пинга"
        ],
        "notes": "Удобно для быстрого прототипирования и тестов прямо в браузере."
    },
    {
        "name": "DomCloud",
        "category": "VPS",
        "type_desc": "Linux Web & Shell Hosting",
        "ram_mb": 512,
        "cpu_cores": 1.0,
        "disk_mb": 1024,
        "bandwidth_gb": 10,
        "has_ssh": True,
        "has_containers": False,
        "always_on": True,
        "popularity": 3,
        "verification": "GitHub OAuth (Без карты)",
        "url": "https://domcloud.co/",
        "sleep_policy": "Всегда активен",
        "pros": [
            "SSH и SFTP доступ",
            "Поддержка PHP, Node, Python, MariaDB",
            "Готовые шаблоны развертывания"
        ],
        "cons": [
            "Небольшой лимит трафика на бесплатном тарифе (10 ГБ)"
        ],
        "notes": "Малоизвестный, но приятный хостинг с честным SSH."
    },

    # --- Базы Данных (Databases) ---
    {
        "name": "Turso (LibSQL)",
        "category": "DB",
        "type_desc": "Distributed Edge SQLite / LibSQL",
        "ram_mb": 512,
        "cpu_cores": 1.0,
        "disk_mb": 9216,  # 9 GB!
        "bandwidth_gb": 50,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 5,
        "verification": "GitHub OAuth (Без карты)",
        "url": "https://turso.tech/",
        "sleep_policy": "Не засыпает! Мгновенный отклик < 10 мс",
        "pros": [
            "Гигантские 9 ГБ дискового пространства бесплатно!",
            "До 500 независимых баз данных на аккаунте",
            "1 миллиард чтений строк в месяц",
            "Репликация по всему миру на edge-серверах",
            "Работает по HTTP, WebSocket и нативным клиентам"
        ],
        "cons": [
            "Диалект SQLite (LibSQL), а не тяжелый Postgres"
        ],
        "notes": "Главный алмаз среди баз данных. Забудь про лимиты SQLite — здесь дают 9 ГБ и 500 баз!"
    },
    {
        "name": "CockroachDB Serverless",
        "category": "DB",
        "type_desc": "Distributed SQL (PostgreSQL-compatible)",
        "ram_mb": 1024,
        "cpu_cores": 1.0,
        "disk_mb": 10240, # 10 GB!
        "bandwidth_gb": 50,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 4,
        "verification": "Email / GitHub (Без карты)",
        "url": "https://www.cockroachlabs.com/",
        "sleep_policy": "Всегда активен, авто-масштабирование без сна",
        "pros": [
            "10 ГБ постоянного хранилища бесплатно!",
            "50 миллионов Request Units в месяц",
            "Полная совместимость с PostgreSQL драйверами",
            "Автоматическая репликация и отказоустойчивость"
        ],
        "cons": [
            "Специфика распределенного SQL (немного медленнее на простых одиночных селектах)"
        ],
        "notes": "10 ГБ настоящей SQL базы без кредитки — это один из самых щедрых тарифов на рынке."
    },
    {
        "name": "TiDB Cloud",
        "category": "DB",
        "type_desc": "Serverless MySQL-Compatible DB",
        "ram_mb": 1024,
        "cpu_cores": 1.0,
        "disk_mb": 5120,  # 5 GB
        "bandwidth_gb": 50,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 3,
        "verification": "Email / GitHub (Без карты)",
        "url": "https://tidbcloud.com/",
        "sleep_policy": "Всегда активен",
        "pros": [
            "5 ГБ диска бесплатно",
            "50 миллионов Request Units в месяц",
            "100% совместимость с протоколом MySQL (можно подключать любой mysql-клиент)"
        ],
        "cons": [
            "Ограничение на 5 одновременных подключений в пике"
        ],
        "notes": "Если нужен именно MySQL, а не Postgres — TiDB Serverless вне конкуренции."
    },
    {
        "name": "Neon.tech",
        "category": "DB",
        "type_desc": "Serverless PostgreSQL",
        "ram_mb": 1024,
        "cpu_cores": 1.0,
        "disk_mb": 512,
        "bandwidth_gb": 20,
        "has_ssh": False,
        "has_containers": False,
        "always_on": False,
        "popularity": 8,
        "verification": "GitHub / Email (Без карты)",
        "url": "https://neon.tech/",
        "sleep_policy": "Авто-пауза при неактивности (старт за ~500 мс)",
        "pros": [
            "Честный современный PostgreSQL 16/17",
            "Ветвление баз данных как в Git (Branching)",
            "Быстрое просыпание без ошибок таймаута",
            "Поддержка pgvector для AI эмбеддингов"
        ],
        "cons": [
            "500 МБ хранилища",
            "Вычислительные часы ограничены 100 часами работы"
        ],
        "notes": "Стандарт де-факто для современных пет-проектов на Postgres."
    },
    {
        "name": "Supabase",
        "category": "DB",
        "type_desc": "PostgreSQL Backend-as-a-Service",
        "ram_mb": 1024,
        "cpu_cores": 1.0,
        "disk_mb": 500,
        "bandwidth_gb": 5,
        "has_ssh": False,
        "has_containers": False,
        "always_on": False,
        "popularity": 10,
        "verification": "GitHub (Без карты)",
        "url": "https://supabase.com/",
        "sleep_policy": "Пауза после 7 дней бездействия (разбудить можно в 1 клик)",
        "pros": [
            "Полноценный Postgres 500 МБ",
            "Встроенная авторизация, realtime подписки, REST и GraphQL API",
            "1 ГБ S3-хранилища файлов в комплекте",
            "2 активных проекта бесплатно"
        ],
        "cons": [
            "Засыпает через неделю, если нет обращений"
        ],
        "notes": "Самый известный и комбайновый сервис, швейцарский нож."
    },
    {
        "name": "Upstash (Redis & Kafka)",
        "category": "DB",
        "type_desc": "Serverless Redis & Vector DB",
        "ram_mb": 256,
        "cpu_cores": 1.0,
        "disk_mb": 256,
        "bandwidth_gb": 10,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 7,
        "verification": "GitHub / Google (Без карты)",
        "url": "https://upstash.com/",
        "sleep_policy": "Всегда активен, оплата по запросам",
        "pros": [
            "10 000 запросов к Redis в день бесплатно",
            "Работает по REST API (удобно из скриптов и edge worker'ов)",
            "Поддержка очередей QStash и Vector DB"
        ],
        "cons": [
            "Лимит 256 МБ данных"
        ],
        "notes": "Лучший бесплатный кеш и Redis без головной боли."
    },
    {
        "name": "MongoDB Atlas",
        "category": "DB",
        "type_desc": "Managed NoSQL MongoDB Cluster",
        "ram_mb": 512,
        "cpu_cores": 0.5,
        "disk_mb": 512,
        "bandwidth_gb": 10,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 9,
        "verification": "Email / Google (Без карты)",
        "url": "https://www.mongodb.com/cloud/atlas",
        "sleep_policy": "Кластер M0 никогда не засыпает",
        "pros": [
            "512 МБ хранилища навсегда",
            "Репликационный сет из 3 нод бесплатно",
            "Официальный MongoDB драйвер"
        ],
        "cons": [
            "Общий пул ресурсов (shared cluster)",
            "Нет доступа к системным командам сервера"
        ],
        "notes": "Классика NoSQL. Работает годами без выключений."
    },

    # --- S3 / Object Storage ---
    {
        "name": "Storj (Tardigrade)",
        "category": "S3",
        "type_desc": "Decentralized S3-Compatible Storage",
        "ram_mb": 0,
        "cpu_cores": 0,
        "disk_mb": 25600, # 25 GB!
        "bandwidth_gb": 25,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 3,
        "verification": "Только Email (БЕЗ ПРИВЯЗКИ КАРТЫ)",
        "url": "https://www.storj.io/",
        "sleep_policy": "Всегда онлайн",
        "pros": [
            "25 ГБ БЕСПЛАТНОГО S3 ХРАНИЛИЩА!",
            "25 ГБ бесплатного исходящего трафика каждый месяц",
            "100% S3-совместимый API (работает с aws-cli, rclone, s3cmd)",
            "Шифрование на стороне клиента и распределенная отказоустойчивость"
        ],
        "cons": [
            "При превышении 25 ГБ аккаунт блокирует загрузку до очистки"
        ],
        "notes": "Абсолютный чемпион среди бесплатных S3 без карт. 25 ГБ чистого S3-бакета!"
    },
    {
        "name": "Supabase Storage",
        "category": "S3",
        "type_desc": "S3-Compatible Object Storage",
        "ram_mb": 0,
        "cpu_cores": 0,
        "disk_mb": 1024,  # 1 GB
        "bandwidth_gb": 2,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 8,
        "verification": "GitHub (Без карты)",
        "url": "https://supabase.com/storage",
        "sleep_policy": "Всегда онлайн",
        "pros": [
            "1 ГБ S3-совместимого хранилища",
            "Встроенная трансформация изображений",
            "Интеграция с Row Level Security"
        ],
        "cons": [
            "2 ГБ трафика в месяц"
        ],
        "notes": "Удобно, если уже используется база Supabase."
    },
    {
        "name": "Filen.io",
        "category": "S3",
        "type_desc": "Zero-Knowledge Cloud & WebDAV",
        "ram_mb": 0,
        "cpu_cores": 0,
        "disk_mb": 10240, # 10 GB
        "bandwidth_gb": 50,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 4,
        "verification": "Только Email (Без карты)",
        "url": "https://filen.io/",
        "sleep_policy": "Всегда онлайн",
        "pros": [
            "10 ГБ бесплатно (расширяется рефералами до 40 ГБ)",
            "Полное E2E шифрование с нулевым знанием",
            "Официальный CLI и WebDAV шлюз для интеграции"
        ],
        "cons": [
            "Не прямой S3, а через WebDAV / CLI мост"
        ],
        "notes": "Надежное немецкое облако для бэкапов и файлов."
    },
    {
        "name": "Mega.nz",
        "category": "S3",
        "type_desc": "Cloud Storage с CLI / S3 Gateway",
        "ram_mb": 0,
        "cpu_cores": 0,
        "disk_mb": 20480, # 20 GB
        "bandwidth_gb": 50,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 8,
        "verification": "Email (Без карты)",
        "url": "https://mega.io/",
        "sleep_policy": "Всегда онлайн",
        "pros": [
            "20 ГБ бесплатного места",
            "Мощный CLI (`mega-cmd`), позволяющий поднять локальный WebDAV/S3 шлюз",
            "Высокая скорость загрузки"
        ],
        "cons": [
            "Динамический лимит скачивания по IP"
        ],
        "notes": "Для бэкапов через rclone / mega-cmd — отличный вариант на 20 ГБ."
    },

    # --- PaaS / Web Hosting / Serverless ---
    {
        "name": "Vercel",
        "category": "PaaS",
        "type_desc": "Frontend & Edge Serverless Platform",
        "ram_mb": 1024,
        "cpu_cores": 1.0,
        "disk_mb": 512,
        "bandwidth_gb": 100,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 10,
        "verification": "GitHub / GitLab / Email (Без карты)",
        "url": "https://vercel.com/",
        "sleep_policy": "Serverless (отклик за пару миллисекунд)",
        "pros": [
            "100 ГБ исходящего трафика",
            "Сверхбыстрый глобальный Anycast CDN",
            "Деплой сайтов, SSR и бекенд serverless-функций",
            "Мгновенный откат и превью PR"
        ],
        "cons": [
            "Таймаут функции 10 секунд на Free плане",
            "Нельзя запускать постоянные фоновые демоны"
        ],
        "notes": "Король фронтенда и легких бекендов. Быстрее всех в мире отдает статику."
    },
    {
        "name": "Cloudflare Pages & Workers",
        "category": "PaaS",
        "type_desc": "Edge Workers & Unlimited Static Hosting",
        "ram_mb": 128,
        "cpu_cores": 1.0,
        "disk_mb": 1024,
        "bandwidth_gb": 10000, # Безлимит по сути
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 9,
        "verification": "Email (Без карты)",
        "url": "https://pages.cloudflare.com/",
        "sleep_policy": "Всегда активен на Edge",
        "pros": [
            "НЕОГРАНИЧЕННЫЙ трафик для статических сайтов",
            "100 000 запросов к Workers в день бесплатно",
            "Встроенная база KV и D1 (SQLite на Edge)",
            "Самая мощная DDoS-защита в мире"
        ],
        "cons": [
            "10 мс процессорного времени на один вызов Worker"
        ],
        "notes": "Если надо захостить сайт, лендинг или доки — лучше Cloudflare Pages ничего нет."
    },
    {
        "name": "Deno Deploy",
        "category": "PaaS",
        "type_desc": "Global Modern Edge JavaScript/Wasm Runtime",
        "ram_mb": 512,
        "cpu_cores": 1.0,
        "disk_mb": 512,
        "bandwidth_gb": 100,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 5,
        "verification": "GitHub (Без карты)",
        "url": "https://deno.com/deploy",
        "sleep_policy": "Мгновенный Edge холодный старт (< 10 мс)",
        "pros": [
            "100 000 запросов в день бесплатно",
            "100 ГБ трафика в месяц",
            "Встроенный Deno KV storage (1 ГБ)",
            "Поддержка WebAssembly (можно запускать Cez скомпилированный в wasm!)"
        ],
        "cons": [
            "Специфика рантайма Deno (не чистый Linux контейнер)"
        ],
        "notes": "Очень быстрый современный edge-сервер."
    },
    {
        "name": "Netlify",
        "category": "PaaS",
        "type_desc": "Web Platform & Serverless Functions",
        "ram_mb": 1024,
        "cpu_cores": 1.0,
        "disk_mb": 1024,
        "bandwidth_gb": 100,
        "has_ssh": False,
        "has_containers": False,
        "always_on": True,
        "popularity": 9,
        "verification": "GitHub / Email (Без карты)",
        "url": "https://www.netlify.com/",
        "sleep_policy": "Serverless",
        "pros": [
            "100 ГБ трафика в месяц",
            "Формы, аутентификация и функции из коробки",
            "Удобный CLI для деплоя из терминала"
        ],
        "cons": [
            "Лимит 300 минут сборки в месяц"
        ],
        "notes": "Прямой конкурент Vercel, отлично работает со статикой."
    }
]

# ============================================================================
# Алгоритм скоринга и сортировок
# ============================================================================

def calculate_scores(host):
    ram = host.get("ram_mb", 0)
    disk = host.get("disk_mb", 0)
    cpu = host.get("cpu_cores", 0)
    bw = host.get("bandwidth_gb", 0)
    ssh = 1 if host.get("has_ssh") else 0
    containers = 1 if host.get("has_containers") else 0
    always = 1 if host.get("always_on") else 0
    pop = max(1, host.get("popularity", 5))

    # Формула полезной мощности:
    # 1. Память (128МБ = 1 балл)
    # 2. Диск (256МБ = 1 балл)
    # 3. Наличие полноценного SSH (+35 баллов — огромный плюс для системщика)
    # 4. Наличие кастомных Docker-контейнеров (+30 баллов)
    # 5. Always-on без сна (+25 баллов)
    # 6. Трафик (макс 30 баллов, чтобы не забивал вычислительную мощность)
    bw_score = min(30.0, bw / 10.0)
    power = (ram / 64.0) + (disk / 256.0) + (cpu * 12.0) + (ssh * 45.0) + (containers * 35.0) + (always * 25.0) + bw_score

    # 1. Скрытый алмаз (Hidden Gem) = Мощность / (Популярность ^ 0.65)
    # Чем мощнее сервис и чем МЕНЬШЕ о нем знают массы, тем выше он в топе!
    gem_score = power / (pop ** 0.65)

    host["_power_score"] = power
    host["_gem_score"] = gem_score
    return host

for h in CURATED_HOSTS:
    calculate_scores(h)

# ============================================================================
# Онлайн-парсер из репозиториев (free-for-dev и др.)
# ============================================================================

def fetch_and_parse_online():
    url = "https://raw.githubusercontent.com/ripienaar/free-for-dev/master/README.md"
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Cez-Free-Hosts-Parser/1.0'})
        with urllib.request.urlopen(req, timeout=10) as resp:
            content = resp.read().decode('utf-8')
    except Exception as e:
        return None, f"Ошибка сети: {e}"

    lines = content.splitlines()
    current_section = ""
    discovered = []

    # Секции, которые нас интересуют
    target_sections = {
        "PaaS": "PaaS",
        "Web Hosting": "VPS",
        "Major Cloud Providers": "VPS",
        "Storage and Media Processing": "S3",
        "Database": "DB",
    }

    # Стоп-слова (если сервис требует карту или триал — выкидываем)
    banned_keywords = [
        "credit card required", "card required", "requires credit card",
        "free trial", "14-day trial", "30-day trial", "requires payment method",
        "valid credit card", "cc required"
    ]

    for line in lines:
        line_s = line.strip()
        if line_s.startswith("## "):
            sec_name = line_s.lstrip("# ").strip()
            current_section = target_sections.get(sec_name, "")
            continue

        if not current_section:
            continue

        if line_s.startswith("* [") or line_s.startswith("- ["):
            # Формат: * [Name](URL) - Description
            m = re.match(r'^[\*\-]\s+\[([^\]]+)\]\(([^)]+)\)\s*[-–—:]\s*(.*)$', line_s)
            if m:
                name, link, desc = m.group(1), m.group(2), m.group(3)
                desc_lower = desc.lower()

                # Проверяем на стоп-слова карт
                if any(kw in desc_lower for kw in banned_keywords):
                    continue

                # Извлекаем примерные параметры
                ram_mb = 512
                disk_mb = 1024
                m_ram = re.search(r'(\d+)\s*(mb|mib|gb|gib)\s*ram', desc_lower)
                if m_ram:
                    val = int(m_ram.group(1))
                    unit = m_ram.group(2)
                    ram_mb = val * 1024 if "g" in unit else val

                m_disk = re.search(r'(\d+)\s*(mb|mib|gb|gib)\s*(storage|disk|space)', desc_lower)
                if m_disk:
                    val = int(m_disk.group(1))
                    unit = m_disk.group(2)
                    disk_mb = val * 1024 if "g" in unit else val

                has_ssh = "ssh" in desc_lower or "shell" in desc_lower

                item = {
                    "name": name,
                    "category": current_section,
                    "type_desc": desc[:60] + "...",
                    "ram_mb": ram_mb,
                    "cpu_cores": 1.0,
                    "disk_mb": disk_mb,
                    "bandwidth_gb": 50,
                    "has_ssh": has_ssh,
                    "has_containers": "docker" in desc_lower or "container" in desc_lower,
                    "always_on": "sleep" not in desc_lower and "pause" not in desc_lower,
                    "popularity": 4,
                    "verification": "Онлайн парсер (No CC filter)",
                    "url": link,
                    "sleep_policy": "Согласно описанию тарифа",
                    "pros": [desc[:120]],
                    "cons": ["Спарсено автоматически из free-for-dev"],
                    "notes": desc
                }
                calculate_scores(item)
                discovered.append(item)

    return discovered, None

# ============================================================================
# Curses TUI Интерфейс
# ============================================================================

SORT_MODES = [
    ("gem", "💎 Скрытый алмаз (Мощный + Underground)"),
    ("power", "⚡ Самое мощное (RAM / Диск / SSH)"),
    ("popular", "⭐ Самое популярное (Мейнстрим)"),
    ("underground", "🕵️ Underground (Малоизвестные / Редкие)"),
]

CATEGORIES = [
    ("ALL", "Все"),
    ("VPS", "VPS & Shell"),
    ("DB", "Базы Данных"),
    ("S3", "S3 Хранилища"),
    ("PaaS", "PaaS & Сайты"),
]

class FreeHostsTUI:
    def __init__(self, stdscr):
        self.stdscr = stdscr
        self.hosts = list(CURATED_HOSTS)
        self.current_cat_idx = 0
        self.current_sort_idx = 0  # 0 = Скрытый алмаз по умолчанию!
        self.selected_idx = 0
        self.search_query = ""
        self.status_msg = "Готово. Нажми [S] для смены сортировки, [1-5] для категорий."
        self.status_color = 2

    def get_filtered_sorted_hosts(self):
        cat_key = CATEGORIES[self.current_cat_idx][0]
        res = self.hosts

        # Фильтр по категории
        if cat_key != "ALL":
            res = [h for h in res if h.get("category") == cat_key]

        # Фильтр по поисковой строке
        if self.search_query:
            q = self.search_query.lower()
            res = [h for h in res if q in h["name"].lower() or q in h.get("type_desc", "").lower() or q in h.get("notes", "").lower()]

        # Сортировка
        sort_mode = SORT_MODES[self.current_sort_idx][0]
        if sort_mode == "gem":
            res = sorted(res, key=lambda x: x.get("_gem_score", 0), reverse=True)
        elif sort_mode == "power":
            res = sorted(res, key=lambda x: x.get("_power_score", 0), reverse=True)
        elif sort_mode == "popular":
            res = sorted(res, key=lambda x: x.get("popularity", 5), reverse=True)
        elif sort_mode == "underground":
            res = sorted(res, key=lambda x: x.get("popularity", 5))

        return res

    def safe_addstr(self, y, x, s, attr=0):
        try:
            h, w = self.stdscr.getmaxyx()
            if y >= h or x >= w or y < 0 or x < 0:
                return
            max_len = w - x
            if y == h - 1:
                max_len = max(0, max_len - 1)
            if max_len <= 0:
                return
            substr = s[:max_len]
            if attr:
                self.stdscr.addstr(y, x, substr, attr)
            else:
                self.stdscr.addstr(y, x, substr)
        except Exception:
            pass

    def run(self):
        try:
            curses.curs_set(0)
        except Exception:
            pass
        self.stdscr.nodelay(False)
        self.stdscr.keypad(True)

        # Настройка цветовых пар
        if curses.has_colors():
            try:
                curses.start_color()
                curses.use_default_colors()
            except Exception:
                pass
            try:
                curses.init_pair(1, curses.COLOR_CYAN, -1)     # Заголовки
                curses.init_pair(2, curses.COLOR_GREEN, -1)    # Успех / статус
                curses.init_pair(3, curses.COLOR_YELLOW, -1)   # Акцент / предупреждение
                curses.init_pair(4, curses.COLOR_WHITE, curses.COLOR_BLUE)  # Выделение строки
                curses.init_pair(5, curses.COLOR_MAGENTA, -1)  # Категория
                curses.init_pair(6, curses.COLOR_RED, -1)      # Ошибки
                curses.init_pair(7, curses.COLOR_BLACK, curses.COLOR_CYAN) # Табы активные
            except Exception:
                pass

        while True:
            self.draw()
            try:
                ch = self.stdscr.getch()
            except KeyboardInterrupt:
                break

            if ch in (ord('q'), ord('Q'), 27): # q или Esc
                break
            elif ch in (curses.KEY_UP, ord('k'), ord('K')):
                if self.selected_idx > 0:
                    self.selected_idx -= 1
            elif ch in (curses.KEY_DOWN, ord('j'), ord('J')):
                filtered = self.get_filtered_sorted_hosts()
                if self.selected_idx + 1 < len(filtered):
                    self.selected_idx += 1
            elif ch == curses.KEY_PPAGE: # Page Up
                self.selected_idx = max(0, self.selected_idx - 10)
            elif ch == curses.KEY_NPAGE: # Page Down
                filtered = self.get_filtered_sorted_hosts()
                self.selected_idx = min(len(filtered) - 1, self.selected_idx + 10)
            elif ch in (ord('s'), ord('S'), ord('ы'), ord('Ы')): # Переключение сортировки
                self.current_sort_idx = (self.current_sort_idx + 1) % len(SORT_MODES)
                self.selected_idx = 0
                self.status_msg = f"Режим сортировки изменен: {SORT_MODES[self.current_sort_idx][1]}"
                self.status_color = 3
            elif ch in (ord('1'), ord('2'), ord('3'), ord('4'), ord('5')):
                self.current_cat_idx = int(chr(ch)) - 1
                self.selected_idx = 0
            elif ch in (ord('\t'),): # Tab переключает категорию
                self.current_cat_idx = (self.current_cat_idx + 1) % len(CATEGORIES)
                self.selected_idx = 0
            elif ch in (ord('/'),): # Поиск
                self.prompt_search()
            elif ch in (ord('c'), ord('C'), ord('с'), ord('С')): # Копировать URL
                self.copy_url()
            elif ch in (ord('o'), ord('O'), ord('щ'), ord('Щ'), 10, 13): # Открыть URL
                self.open_url()
            elif ch in (ord('u'), ord('U'), ord('г'), ord('Г')): # Обновить базу из сети
                self.update_online()

    def prompt_search(self):
        try:
            curses.curs_set(1)
        except Exception:
            pass
        h, w = self.stdscr.getmaxyx()
        prompt_y = h - 2
        self.stdscr.move(prompt_y, 0)
        self.stdscr.clrtoeol()
        self.stdscr.attron(curses.color_pair(3) | curses.A_BOLD)
        prompt = "Поиск (Esc/Enter для подтверждения): "
        self.stdscr.addstr(prompt_y, 2, prompt)
        self.stdscr.attroff(curses.color_pair(3) | curses.A_BOLD)

        query = []
        while True:
            self.stdscr.refresh()
            k = self.stdscr.getch()
            if k in (10, 13): # Enter
                break
            elif k in (27,): # Esc
                query = []
                break
            elif k in (curses.KEY_BACKSPACE, 127, 8):
                if query:
                    query.pop()
                    self.stdscr.move(prompt_y, 2 + len(prompt) + len(query))
                    self.stdscr.delch()
            elif 32 <= k <= 126:
                query.append(chr(k))
                self.stdscr.addch(prompt_y, 2 + len(prompt) + len(query) - 1, k)

        try:
            curses.curs_set(0)
        except Exception:
            pass
        self.search_query = "".join(query).strip()
        self.selected_idx = 0
        if self.search_query:
            self.status_msg = f"Фильтр: '{self.search_query}' (всего найдено: {len(self.get_filtered_sorted_hosts())})"
        else:
            self.status_msg = "Поисковый фильтр очищен."
        self.status_color = 2

    def copy_url(self):
        filtered = self.get_filtered_sorted_hosts()
        if 0 <= self.selected_idx < len(filtered):
            item = filtered[self.selected_idx]
            url = item.get("url", "")
            try:
                # Пробуем wl-copy (Wayland), затем xclip
                proc = subprocess.Popen(["wl-copy"], stdin=subprocess.PIPE)
                proc.communicate(input=url.encode("utf-8"))
                self.status_msg = f"Ссылка скопирована в буфер: {url}"
                self.status_color = 2
            except Exception:
                self.status_msg = f"URL: {url}"
                self.status_color = 3

    def open_url(self):
        filtered = self.get_filtered_sorted_hosts()
        if 0 <= self.selected_idx < len(filtered):
            item = filtered[self.selected_idx]
            url = item.get("url", "")
            try:
                subprocess.Popen(["xdg-open", url], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                self.status_msg = f"Открываем в браузере: {url}"
                self.status_color = 2
            except Exception as e:
                self.status_msg = f"Не удалось открыть браузер: {e}"
                self.status_color = 6

    def update_online(self):
        self.status_msg = "Скачиваем и парсим live-базу из GitHub (free-for-dev)... Пожалуйста подождите..."
        self.status_color = 3
        self.draw()
        self.stdscr.refresh()

        new_items, err = fetch_and_parse_online()
        if err:
            self.status_msg = f"Ошибка обновления: {err}"
            self.status_color = 6
        else:
            existing_names = {h["name"].lower() for h in self.hosts}
            added = 0
            for item in new_items:
                if item["name"].lower() not in existing_names:
                    self.hosts.append(item)
                    existing_names.add(item["name"].lower())
                    added += 1
            self.status_msg = f"Успешно обновлено! Добавлено {added} новых проверенных сервисов (всего в базе: {len(self.hosts)})."
            self.status_color = 2

    def draw(self):
        self.stdscr.erase()
        h, w = self.stdscr.getmaxyx()
        if h < 16 or w < 60:
            self.safe_addstr(0, 0, "Окно терминала слишком маленькое. Увеличьте размер окна!")
            self.stdscr.refresh()
            return

        # 1. Шапка
        title = " 🚀 FREE-HOSTS // Бесплатная инфраструктура без кредитных карт (NO CC) "
        self.stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
        self.safe_addstr(0, 0, "┌" + "─" * (w - 2) + "┐")
        self.safe_addstr(1, 0, "│" + title.center(w - 2) + "│")
        self.safe_addstr(2, 0, "├" + "─" * (w - 2) + "┤")
        self.stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)

        # 2. Табы категорий (строка 3)
        self.safe_addstr(3, 0, "│ Категории: ")
        col = 13
        for idx, (cat_code, cat_title) in enumerate(CATEGORIES):
            tag = f" [{idx+1}] {cat_title} "
            if idx == self.current_cat_idx:
                self.stdscr.attron(curses.color_pair(7) | curses.A_BOLD)
                self.safe_addstr(3, col, tag)
                self.stdscr.attroff(curses.color_pair(7) | curses.A_BOLD)
            else:
                self.stdscr.attron(curses.color_pair(1))
                self.safe_addstr(3, col, tag)
                self.stdscr.attroff(curses.color_pair(1))
            col += len(tag) + 1

        self.safe_addstr(3, w - 1, "│")

        # 3. Режим сортировки и поиск (строка 4)
        sort_name = SORT_MODES[self.current_sort_idx][1]
        sort_str = f"│ Сортировка: [S] {sort_name}"
        self.stdscr.attron(curses.A_BOLD)
        self.safe_addstr(4, 0, sort_str)
        self.stdscr.attroff(curses.A_BOLD)
        if self.search_query:
            s_tag = f"  Поиск: [{self.search_query}]"
            self.stdscr.attron(curses.color_pair(3))
            self.safe_addstr(4, len(sort_str), s_tag)
            self.stdscr.attroff(curses.color_pair(3))
        self.safe_addstr(4, w - 1, "│")

        # Разделитель
        self.safe_addstr(5, 0, "├" + "─" * (w - 2) + "┤")

        # 4. Основной сплит (Левая колонка = список, Правая = детали)
        left_w = min(42, max(30, w // 2 - 2))
        right_w = max(10, w - left_w - 3)

        # Заголовки колонок
        self.stdscr.attron(curses.A_BOLD)
        header_left = f"│ {'Название':<15} {'Кат':<5} {'RAM':<7} {'Диск':<6} "
        self.safe_addstr(6, 0, header_left)
        self.safe_addstr(6, left_w + 1, "│ Подробные характеристики сервиса")
        self.safe_addstr(6, w - 1, "│")
        self.stdscr.attroff(curses.A_BOLD)

        # Список сервисов
        filtered = self.get_filtered_sorted_hosts()
        list_max_h = max(1, h - 10)

        # Корректировка скролла
        if self.selected_idx >= len(filtered):
            self.selected_idx = max(0, len(filtered) - 1)

        scroll_offset = 0
        if self.selected_idx >= list_max_h:
            scroll_offset = self.selected_idx - list_max_h + 1

        for i in range(list_max_h):
            row_y = 7 + i
            item_idx = scroll_offset + i

            self.safe_addstr(row_y, 0, "│")
            if item_idx < len(filtered):
                item = filtered[item_idx]
                is_sel = (item_idx == self.selected_idx)

                name_s = item["name"][:14]
                cat_s = item.get("category", "")[:4]
                ram_mb = item.get("ram_mb", 0)
                ram_s = f"{ram_mb//1024}GB" if ram_mb >= 1024 else (f"{ram_mb}MB" if ram_mb > 0 else "-")
                disk_mb = item.get("disk_mb", 0)
                disk_s = f"{disk_mb//1024}GB" if disk_mb >= 1024 else (f"{disk_mb}MB" if disk_mb > 0 else "-")

                row_str = f" {'>' if is_sel else ' '} {name_s:<14} {cat_s:<5} {ram_s:<7} {disk_s:<6}"

                if is_sel:
                    self.stdscr.attron(curses.color_pair(4) | curses.A_BOLD)
                    self.safe_addstr(row_y, 1, row_str.ljust(left_w))
                    self.stdscr.attroff(curses.color_pair(4) | curses.A_BOLD)
                else:
                    self.safe_addstr(row_y, 1, row_str.ljust(left_w))
            else:
                self.safe_addstr(row_y, 1, " " * left_w)

            # Вертикальный разделитель
            self.safe_addstr(row_y, left_w + 1, "│")

            # Очистка правой колонки
            self.safe_addstr(row_y, left_w + 2, " " * right_w)
            self.safe_addstr(row_y, w - 1, "│")

        # 5. Отрисовка деталей выбранного сервиса в правой колонке
        if filtered and 0 <= self.selected_idx < len(filtered):
            sel_item = filtered[self.selected_idx]
            self.draw_details(sel_item, left_w + 3, 7, right_w - 2, list_max_h)

        # 6. Статусная строка и подсказки снизу
        bottom_y = h - 3
        self.safe_addstr(bottom_y, 0, "├" + "─" * (w - 2) + "┤")

        # Статус
        self.stdscr.attron(curses.color_pair(self.status_color))
        status_line = f"│ ℹ️ {self.status_msg}"
        self.safe_addstr(bottom_y + 1, 0, status_line[:w - 1].ljust(w - 1) + "│")
        self.stdscr.attroff(curses.color_pair(self.status_color))

        # Горячие клавиши
        keys_hint = "│ [↑/↓] Навигация  [1-5/Tab] Категория  [S] Сорт  [/] Поиск  [O/Enter] Открыть  [C] URL  [U] Обновить  [Q] Выход"
        self.stdscr.attron(curses.A_DIM)
        self.safe_addstr(bottom_y + 2, 0, keys_hint[:w - 1].ljust(w - 1) + "│")
        self.stdscr.attroff(curses.A_DIM)

        self.safe_addstr(h - 1, 0, "└" + "─" * (w - 2) + "┘")

    def draw_details(self, item, x, y, max_w, max_h):
        line = 0

        def put_line(text, color=0, bold=False):
            nonlocal line
            if line >= max_h:
                return
            attrs = curses.color_pair(color)
            if bold:
                attrs |= curses.A_BOLD
            self.stdscr.attron(attrs)
            self.safe_addstr(y + line, x, text[:max_w])
            self.stdscr.attroff(attrs)
            line += 1

        # Заголовок сервиса
        cat_badge = f"[{item.get('category', 'OTHER')}]"
        put_line(f"📌 {item['name']}  {cat_badge} — {item.get('type_desc', '')}", color=1, bold=True)
        put_line("─" * max_w)

        # Спецификации
        ram = item.get("ram_mb", 0)
        ram_txt = f"{ram/1024:.1f} GB" if ram >= 1024 else f"{ram} MB"
        disk = item.get("disk_mb", 0)
        disk_txt = f"{disk/1024:.1f} GB" if disk >= 1024 else f"{disk} MB"
        ssh_txt = "✅ ЕСТЬ (Full Shell)" if item.get("has_ssh") else "❌ Нет (только HTTP/Web)"
        docker_txt = "✅ ЕСТЬ" if item.get("has_containers") else "❌ Нет"

        put_line(f"• ОЗУ (RAM):        {ram_txt}", color=2, bold=True)
        put_line(f"• Диск (Storage):   {disk_txt}", color=2, bold=True)
        put_line(f"• SSH Доступ:       {ssh_txt}")
        put_line(f"• Docker Контейнеры:{docker_txt}")
        put_line(f"• Режим сна:        {item.get('sleep_policy', 'Всегда активен')}")
        put_line(f"• Верификация:      {item.get('verification', 'Без карт')}", color=3, bold=True)
        put_line("─" * max_w)

        # Плюсы и особенности
        put_line("💡 Плюсы и фичи:", color=3, bold=True)
        for pro in item.get("pros", []):
            put_line(f"  + {pro}")

        # Минусы
        if item.get("cons"):
            put_line("⚠️ Нюансы и ограничения:", color=3)
            for con in item.get("cons", []):
                put_line(f"  - {con}")

        line += 1
        put_line(f"🔗 Ссылка: {item.get('url', '')}", color=1, bold=True)
        put_line("   (Нажми [O] чтобы открыть в браузере, [C] чтобы скопировать)", color=0)

def print_table_cli(category="ALL", sort_mode="gem"):
    tui = FreeHostsTUI(None)
    for idx, (cat_code, _) in enumerate(CATEGORIES):
        if cat_code.lower() == category.lower():
            tui.current_cat_idx = idx
            break
    for idx, (sm, _) in enumerate(SORT_MODES):
        if sm.lower() == sort_mode.lower():
            tui.current_sort_idx = idx
            break

    hosts = tui.get_filtered_sorted_hosts()
    sort_name = SORT_MODES[tui.current_sort_idx][1]
    cat_name = CATEGORIES[tui.current_cat_idx][1]

    print(f"\n🚀 FREE-HOSTS // Бесплатные VDS, Базы Данных и S3 (100% БЕЗ КАРТ)")
    print(f"Категория: {cat_name} | Сортировка: {sort_name}")
    print("=" * 105)
    print(f"{'#':<3} {'Название':<25} {'Категория':<12} {'RAM':<8} {'Диск':<9} {'SSH':<6} {'Ссылка'}")
    print("-" * 105)

    for i, h in enumerate(hosts):
        ram = h.get('ram_mb', 0)
        ram_s = f"{ram//1024}GB" if ram >= 1024 else (f"{ram}MB" if ram > 0 else "-")
        disk = h.get('disk_mb', 0)
        disk_s = f"{disk//1024}GB" if disk >= 1024 else (f"{disk}MB" if disk > 0 else "-")
        ssh_s = "Да" if h.get('has_ssh') else "-"
        print(f"{i+1:<3} {h['name']:<25} {h.get('category',''):<12} {ram_s:<8} {disk_s:<9} {ssh_s:<6} {h.get('url','')}")

    print("=" * 105)
    print("💡 Совет: запусти просто 'free-hosts' в терминале для запуска интерактивного TUI!\n")

def main():
    args = [a.lower() for a in sys.argv[1:]]
    cli_flags = {"--table", "--list", "-l", "-t", "--help", "-h", "vps", "db", "s3", "paas", "gem", "power", "popular", "underground"}
    if any(a in cli_flags for a in args) or not sys.stdin.isatty():
        if "--help" in args or "-h" in args:
            print("Использование: free-hosts [категория: vps|db|s3|paas] [сортировка: gem|power|popular|underground] [--table]")
            print("Без аргументов запускается интерактивный TUI-навигатор.")
            return
        cat = "ALL"
        sort_m = "gem"
        for a in args:
            if a in ("vps", "db", "s3", "paas"):
                cat = a.upper()
            elif a in ("gem", "power", "popular", "underground"):
                sort_m = a
        print_table_cli(cat, sort_m)
        return

    try:
        curses.wrapper(lambda stdscr: FreeHostsTUI(stdscr).run())
    except KeyboardInterrupt:
        pass
    except Exception as e:
        # Fallback при невозможности открыть curses (например редкие терминалы)
        print(f"[!] TUI не смог открыться: {e}")
        print("[*] Переключаюсь в текстовый режим вывода таблицы:")
        print_table_cli()
        return

    print("\n[Free-Hosts] Завершено. Удачи в разработке инфраструктуры на Cez! 🚀\n")

if __name__ == "__main__":
    main()
