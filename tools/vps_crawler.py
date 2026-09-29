#!/usr/bin/env python3
"""
vps_crawler.py — Deep underground crawler for free VPS / SSH shells.
Sources: GitHub aggregators, UNIX Shells DB, Tildeverse, LowEndTalk, community databases.
HARD FILTER: Credit cards, KYC, billing info → immediate DROP.
"""

import sys, os, re, json, urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed

USER_AGENT = "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0"

# ─── Стоп-слова (карты, KYC, платные триалы) ─────────────────────────────────
CARD_KILL = [
    "credit card", "debit card", "billing info", "billing details",
    "card required", "card verification", "bank account", "passport",
    "id verification", "identity verification", "kyc", "payment method",
    "card on file", "$1 verif", "0.99 verif", "pro only", "paid tier",
]

# ─── VPS/Shell — настоящие маркеры (нужно хотя бы одно из них) ───────────────
VPS_MUST = [
    r"\bvps\b", r"\bssh\b", r"\bshell\b",
    r"compute\s+instance", r"cloud\s+server",
    r"linux\s+server", r"virtual\s+machine",
    r"full.?root", r"root\s+access",
]

# ─── Маркеры НЕ-VPS (исключаем SaaS, CRM, трекеры, мессенджеры и т.д.) ──────
SAAS_KILL = [
    r"\bcrm\b", r"\bticket(ing)?\b", r"\bspreed?sheet\b",
    r"\bhelpdesk\b", r"\baccounting\b", r"\bscheduling\b",
    r"\bcalendar\b", r"\bchat (app|platform|tool)\b",
    r"\bdesign tool\b", r"\banalytics (platform|tool)\b",
    r"\bseo tool\b", r"\bno.?code\b", r"\blow.?code\b",
    r"\bemail marketing\b", r"\bproject management\b",
    r"\bhuman resources\b", r"zoho\.(desk|crm|books|recruit|cliq)",
]

# ─── Рейтинг «Скрытый алмаз» (больше = лучше) ────────────────────────────────
def gem_score(h):
    ram   = h.get("ram_mb", 0)
    disk  = h.get("disk_mb", 0)
    ssh   = 45 if h.get("has_ssh")        else 0
    dock  = 30 if h.get("has_containers") else 0
    aon   = 15 if h.get("always_on")      else 0
    power = (ram / 64) + (disk / 256) + ssh + dock + aon
    pop   = max(1, h.get("popularity", 3))
    return power / (pop ** 0.55)


# ─── Базовая база проверенных андерграунд-шеллов ─────────────────────────────
KNOWN_HOSTS = [
    # ── UNIX Public Shells ────────────────────────────────────────────────────
    {
        "name": "SDF (Super Dimension Fortress)",
        "category": "VPS", "popularity": 2,
        "ram_mb": 1024, "disk_mb": 5120, "cpu_cores": 1.0, "bandwidth_gb": 500,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email (нет карты)",
        "url": "https://sdf.org/",
        "source": "UNIX Shells", "bad": False,
        "notes": "Некоммерческий кластер на NetBSD с 1987 г. SSH, gcc/g++/rust, web-hosting, 5GB диска."
    },
    {
        "name": "Blinkenshell",
        "category": "VPS", "popularity": 2,
        "ram_mb": 512, "disk_mb": 2048, "cpu_cores": 1.0, "bandwidth_gb": 100,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email / IRC",
        "url": "https://blinkenshell.org/",
        "source": "UNIX Shells", "bad": False,
        "notes": "Ubuntu шелл-аккаунт. SSH, Python, screen, tmux, IRC, 2GB диска."
    },
    {
        "name": "Grex Public Shell",
        "category": "VPS", "popularity": 1,
        "ram_mb": 512, "disk_mb": 1024, "cpu_cores": 1.0, "bandwidth_gb": 50,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email",
        "url": "https://grex.org/",
        "source": "UNIX Shells", "bad": False,
        "notes": "OpenBSD публичный шелл. Старейший No-CC вариант, компиляция, почта, web."
    },
    {
        "name": "Tildebox / Tildeverse",
        "category": "VPS", "popularity": 2,
        "ram_mb": 1024, "disk_mb": 3072, "cpu_cores": 1.0, "bandwidth_gb": 200,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Форм на сайте (нет карты)",
        "url": "https://tildeverse.org/",
        "source": "Tildeverse", "bad": False,
        "notes": "Сеть 30+ независимых Linux серверов (tilde.club, tilde.team, ttm.sh...). SSH, веб, IRC."
    },
    {
        "name": "Tilde.team",
        "category": "VPS", "popularity": 2,
        "ram_mb": 1024, "disk_mb": 4096, "cpu_cores": 1.0, "bandwidth_gb": 200,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email",
        "url": "https://tilde.team/",
        "source": "Tildeverse", "bad": False,
        "notes": "Крупный сервер Tildeverse. Полноценный bash, Python, Ruby, Node, 4GB диска."
    },
    {
        "name": "Cosmic.voyage",
        "category": "VPS", "popularity": 1,
        "ram_mb": 512, "disk_mb": 2048, "cpu_cores": 1.0, "bandwidth_gb": 100,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email",
        "url": "https://cosmic.voyage/",
        "source": "Tildeverse", "bad": False,
        "notes": "Collaborative fiction tilde server. Linux shell с ssh, гемини-протоколом и web."
    },
    {
        "name": "TTM.sh",
        "category": "VPS", "popularity": 1,
        "ram_mb": 512, "disk_mb": 2048, "cpu_cores": 1.0, "bandwidth_gb": 100,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "IRC (нет карты)",
        "url": "https://ttm.sh/",
        "source": "Tildeverse", "bad": False,
        "notes": "tilde-сервер с ssh, gemini, gopher. Малоизвестный, стабильный."
    },
    {
        "name": "Envs.net",
        "category": "VPS", "popularity": 1,
        "ram_mb": 1024, "disk_mb": 5120, "cpu_cores": 2.0, "bandwidth_gb": 500,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email / Форма",
        "url": "https://envs.net/",
        "source": "Tildeverse", "bad": False,
        "notes": "Немецкий публичный Linux шелл. SSH, 5GB диска, Gitea, Matrix, рилейн до Tor."
    },
    {
        "name": "Pebble.ink",
        "category": "VPS", "popularity": 1,
        "ram_mb": 512, "disk_mb": 1024, "cpu_cores": 1.0, "bandwidth_gb": 50,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email",
        "url": "https://pebble.ink/",
        "source": "Tildeverse", "bad": False,
        "notes": "Тихий tilde-шелл с ssh. Минималистский, почти неизвестный."
    },
    # ── Verified Providers (без карт) ─────────────────────────────────────────
    {
        "name": "Serv00",
        "category": "VPS", "popularity": 5,
        "ram_mb": 512, "disk_mb": 3072, "cpu_cores": 1.0, "bandwidth_gb": 1000,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email Only",
        "url": "https://www.serv00.com/",
        "source": "Direct Provider", "bad": False,
        "notes": "FreeBSD 14, открытые TCP/UDP порты, cron, tmux, screen, 10 лет бесплатно."
    },
    {
        "name": "CT8.pl",
        "category": "VPS", "popularity": 3,
        "ram_mb": 512, "disk_mb": 3072, "cpu_cores": 1.0, "bandwidth_gb": 1000,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email Only",
        "url": "https://www.ct8.pl/",
        "source": "Direct Provider", "bad": False,
        "notes": "Клон Serv00. Идентичное железо и условия, другой пул серверов."
    },
    {
        "name": "DomCloud",
        "category": "VPS", "popularity": 3,
        "ram_mb": 512, "disk_mb": 1024, "cpu_cores": 1.0, "bandwidth_gb": 10,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "GitHub OAuth",
        "url": "https://domcloud.co/",
        "source": "Direct Provider", "bad": False,
        "notes": "Linux хостинг с SSH, MariaDB, Node.js, Python, PHP."
    },
    {
        "name": "AlwaysData",
        "category": "VPS", "popularity": 3,
        "ram_mb": 512, "disk_mb": 1024, "cpu_cores": 1.0, "bandwidth_gb": 50,
        "has_ssh": True, "has_containers": False, "always_on": True,
        "verification": "Email (нет карты)",
        "url": "https://www.alwaysdata.com/en/",
        "source": "Direct Provider", "bad": False,
        "notes": "Французский хостинг. SSH, SFTP, Python, Node, PHP, Go, Rust, MySQL, PostgreSQL."
    },
    {
        "name": "GitHub Codespaces",
        "category": "VPS", "popularity": 8,
        "ram_mb": 8192, "disk_mb": 32768, "cpu_cores": 2.0, "bandwidth_gb": 100,
        "has_ssh": True, "has_containers": True, "always_on": False,
        "verification": "GitHub Account (no card, 120 core-h/mo)",
        "url": "https://github.com/features/codespaces",
        "source": "GitHub Native", "bad": False,
        "notes": "8GB RAM, 2 vCPU, sudo/root, Docker. 60ч/мес работы без карт."
    },
    {
        "name": "Koyeb",
        "category": "VPS", "popularity": 5,
        "ram_mb": 512, "disk_mb": 2048, "cpu_cores": 0.5, "bandwidth_gb": 100,
        "has_ssh": False, "has_containers": True, "always_on": False,
        "verification": "GitHub OAuth (нет карты)",
        "url": "https://www.koyeb.com/",
        "source": "Direct Provider", "bad": False,
        "notes": "Serverless Docker. Бесплатный тариф без привязки карт."
    },
    {
        "name": "Glitch",
        "category": "VPS", "popularity": 6,
        "ram_mb": 512, "disk_mb": 200, "cpu_cores": 1.0, "bandwidth_gb": 100,
        "has_ssh": False, "has_containers": True, "always_on": False,
        "verification": "Email / GitHub (нет карты)",
        "url": "https://glitch.com/",
        "source": "Direct Provider", "bad": False,
        "notes": "Linux контейнер с онлайн-редактором, bash-терминалом, Node/Python/C++."
    },
    # ── Малоизвестные PaaS / Compute (No-CC) ─────────────────────────────────
    {
        "name": "Fl0",
        "category": "VPS", "popularity": 2,
        "ram_mb": 512, "disk_mb": 1024, "cpu_cores": 1.0, "bandwidth_gb": 50,
        "has_ssh": False, "has_containers": True, "always_on": False,
        "verification": "GitHub OAuth (нет карты)",
        "url": "https://www.fl0.com/",
        "source": "Underground PaaS", "bad": False,
        "notes": "Бесплатный Docker PaaS от стартапа. 512MB RAM, авто-деплой из GitHub."
    },
    {
        "name": "Northflank (Free Tier)",
        "category": "VPS", "popularity": 2,
        "ram_mb": 256, "disk_mb": 500, "cpu_cores": 0.1, "bandwidth_gb": 10,
        "has_ssh": False, "has_containers": True, "always_on": False,
        "verification": "GitHub / Email (нет карты)",
        "url": "https://northflank.com/",
        "source": "Underground PaaS", "bad": False,
        "notes": "Kubernetes-based PaaS. Бесплатный тир с 256MB RAM и поддержкой Docker."
    },
    {
        "name": "Hetzner Cloud (школьная программа)",
        "category": "VPS", "popularity": 2,
        "ram_mb": 2048, "disk_mb": 20480, "cpu_cores": 1.0, "bandwidth_gb": 200,
        "has_ssh": True, "has_containers": True, "always_on": True,
        "verification": "Student Email (edu-verify, без карты)",
        "url": "https://www.hetzner.com/education/",
        "source": "Education Program", "bad": False,
        "notes": "Для студентов: Hetzner выдает кредиты через .edu адрес без карты."
    },
    {
        "name": "OVHcloud Student (Horizon)",
        "category": "VPS", "popularity": 2,
        "ram_mb": 2048, "disk_mb": 20480, "cpu_cores": 1.0, "bandwidth_gb": 200,
        "has_ssh": True, "has_containers": True, "always_on": True,
        "verification": "Student Email (edu, без карты)",
        "url": "https://www.ovhcloud.com/en-gb/startup/",
        "source": "Education Program", "bad": False,
        "notes": "OVH образовательная программа. Студентам — VPS с SSH без карт."
    },
    {
        "name": "InfinityFree",
        "category": "VPS", "popularity": 4,
        "ram_mb": 512, "disk_mb": 5120, "cpu_cores": 1.0, "bandwidth_gb": 300,
        "has_ssh": False, "has_containers": False, "always_on": True,
        "verification": "Email (нет карты)",
        "url": "https://www.infinityfree.com/",
        "source": "Web Hosting", "bad": False,
        "notes": "Безлимитный хостинг PHP/MySQL (5GB), без SSH, зато без ограничений по трафику."
    },
    {
        "name": "000webhost",
        "category": "VPS", "popularity": 6,
        "ram_mb": 512, "disk_mb": 1024, "cpu_cores": 1.0, "bandwidth_gb": 50,
        "has_ssh": False, "has_containers": False, "always_on": False,
        "verification": "Email (нет карты)",
        "url": "https://www.000webhost.com/",
        "source": "Web Hosting", "bad": False,
        "notes": "PHP+MySQL хостинг без карт. Засыпает ночью — для нагруженных не подходит."
    },
]


def fetch_url(url, timeout=10):
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.read().decode("utf-8", errors="replace")
    except Exception:
        return ""


def has_card_requirement(text):
    t = text.lower()
    return any(kw in t for kw in CARD_KILL)


def is_real_vps(name, desc):
    """Строгая проверка: это точно VPS/SSH/Container, а не SaaS?"""
    text = (name + " " + desc).lower()
    # Если попадает под SaaS-kill → дроп
    for pat in SAAS_KILL:
        if re.search(pat, text):
            return False
    # Если нет ни одного VPS-маркера → дроп
    for pat in VPS_MUST:
        if re.search(pat, text):
            return True
    return False


def parse_specs(text):
    ram_mb, disk_mb = 0, 0
    has_ssh, has_containers = False, False

    m = re.search(r"(\d+)\s*GB\s*(?:of\s+)?(?:RAM|memory)", text, re.I)
    if m:  ram_mb = int(m.group(1)) * 1024
    else:
        m2 = re.search(r"(\d+)\s*MB\s*(?:of\s+)?(?:RAM|memory)", text, re.I)
        if m2: ram_mb = int(m2.group(1))

    m = re.search(r"(\d+)\s*GB\s*(?:SSD|NVMe|HDD|disk|storage)", text, re.I)
    if m:  disk_mb = int(m.group(1)) * 1024
    else:
        m2 = re.search(r"(\d+)\s*MB\s*(?:SSD|HDD|disk|storage)", text, re.I)
        if m2: disk_mb = int(m2.group(1))

    if re.search(r"\bssh\b|shell\s+access|terminal", text, re.I):
        has_ssh = True
    if re.search(r"\bdocker\b|container|kubernetes|\bk8s\b", text, re.I):
        has_containers = True

    return ram_mb, disk_mb, has_ssh, has_containers


def crawl_github_free_for_dev():
    """Парсим free-for-dev только секцию Compute (не весь файл)"""
    url = "https://raw.githubusercontent.com/ripienaar/free-for-dev/master/README.md"
    content = fetch_url(url)
    if not content:
        return []

    # Обрезаем до нужных секций
    sections = ["## PaaS", "## IaaS", "## Cloud management solutions",
                "## Compute", "## Development"]
    results = []
    in_section = False

    lines = content.split("\n")
    for i, line in enumerate(lines):
        # Включаем/выключаем секцию
        if any(line.strip().startswith(s) for s in sections):
            in_section = True
            continue
        if in_section and line.startswith("## "):
            in_section = False
            continue
        if not in_section:
            continue

        # Ищем bullet-ссылки
        m = re.match(r"\s*[-*]\s+\[([^\]]+)\]\((https?://[^\)]+)\)\s*[-–—:]\s*(.+)", line)
        if not m:
            continue

        name, url_m, desc = m.group(1).strip(), m.group(2).strip(), m.group(3).strip()

        if has_card_requirement(name + desc):
            continue
        if not is_real_vps(name, desc):
            continue

        ram_mb, disk_mb, has_ssh, has_containers = parse_specs(desc)

        results.append({
            "name": name,
            "category": "VPS",
            "popularity": 5,
            "ram_mb": ram_mb or 512,
            "disk_mb": disk_mb or 1024,
            "cpu_cores": 1.0,
            "bandwidth_gb": 50,
            "has_ssh": has_ssh,
            "has_containers": has_containers,
            "always_on": False,
            "verification": "No Credit Card (from free-for-dev)",
            "url": url_m,
            "source": "free-for-dev",
            "notes": desc[:120]
        })

    return results


def crawl_lowendtalk():
    url = "https://lowendtalk.com/categories/free-giveaways/feed.rss"
    content = fetch_url(url, timeout=8)
    if not content:
        return []

    results = []
    for block in re.findall(r"<item>(.*?)</item>", content, re.DOTALL):
        tm = re.search(r"<title>(.*?)</title>", block)
        lm = re.search(r"<link>(.*?)</link>", block)
        if not tm or not lm:
            continue

        title = re.sub(r"<[^>]+>", "", tm.group(1)).strip()
        link  = lm.group(1).strip()

        if has_card_requirement(title):
            continue
        if not re.search(r"vps|shell|ssh|hosting|server|kvm|openvz|ipv6", title, re.I):
            continue

        ram_mb, disk_mb, has_ssh, _ = parse_specs(title)
        results.append({
            "name": f"LET: {title[:26]}",
            "category": "VPS", "popularity": 1,
            "ram_mb": ram_mb or 512,
            "disk_mb": disk_mb or 2048,
            "cpu_cores": 1.0, "bandwidth_gb": 100,
            "has_ssh": True, "has_containers": False, "always_on": True,
            "verification": "Forum giveaway (No card)",
            "url": link,
            "source": "LowEndTalk RSS",
            "notes": f"LowEndTalk Free Giveaway: {title}"
        })
    return results


def run_crawler(verbose=True):
    def log(msg):
        if verbose: print(msg)

    log("🔍 [1/4] Загрузка проверенных underground UNIX / Tildeverse шеллов...")
    all_hosts = list(KNOWN_HOSTS)
    known = {h["url"].lower().rstrip("/") for h in all_hosts}

    log("🌐 [2/4] Краулинг GitHub free-for-dev (Compute/PaaS/IaaS секции)...")
    with ThreadPoolExecutor(max_workers=4) as ex:
        futures = {
            ex.submit(crawl_github_free_for_dev): "free-for-dev",
        }
        for f in as_completed(futures):
            try:
                for item in f.result():
                    u = item["url"].lower().rstrip("/")
                    if u not in known:
                        all_hosts.append(item)
                        known.add(u)
            except Exception:
                pass

    log("📡 [3/4] Парсинг LowEndTalk Free Giveaways RSS...")
    for item in crawl_lowendtalk():
        u = item["url"].lower().rstrip("/")
        if u not in known:
            all_hosts.append(item)
            known.add(u)

    # Только VPS/Compute (без чистых DB/S3/PaaS-только)
    vps_hosts = [h for h in all_hosts
                 if h.get("has_ssh") or h.get("has_containers")
                 or h.get("category") == "VPS"]

    log(f"\n✅ [4/4] Готово! Найдено {len(vps_hosts)} VPS/Shell сервисов без карт")

    cache = os.path.expanduser("~/.cache/cez/vps_crawled.json")
    os.makedirs(os.path.dirname(cache), exist_ok=True)
    with open(cache, "w", encoding="utf-8") as f:
        json.dump(vps_hosts, f, indent=2, ensure_ascii=False)

    return vps_hosts


def print_table(hosts, limit=None, sort="gem"):
    SORTS = {
        "gem":     ("💎 Скрытый алмаз (Мощь + Underground)", gem_score),
        "power":   ("🚀 По мощности",   lambda h: h.get("ram_mb", 0) + h.get("disk_mb", 0) / 4),
        "popular": ("⭐ По популярности", lambda h: -h.get("popularity", 5)),
        "underground": ("🕳️  Underground", lambda h: h.get("popularity", 5)),
    }
    sort_label, key_fn = SORTS.get(sort, SORTS["gem"])
    sorted_h = sorted(hosts, key=key_fn, reverse=(sort != "underground"))
    if limit:
        sorted_h = sorted_h[:limit]

    W = 118
    print("\n" + "━" * W)
    print(f"  🚀 VPS / Shells без кредитных карт  |  {sort_label}  |  Всего: {len(hosts)}")
    print("━" * W)
    hdr = f"{'#':<3} {'Название':<27} {'RAM':<7} {'Диск':<7} {'SSH':^4} {'Docker':^7} {'24/7':^5} {'Источник':<17} Ссылка"
    print(hdr)
    print("─" * W)

    for i, h in enumerate(sorted_h):
        r  = h.get("ram_mb", 0)
        d  = h.get("disk_mb", 0)
        rs = f"{r//1024}GB" if r >= 1024 else (f"{r}MB" if r else "-")
        ds = f"{d//1024}GB" if d >= 1024 else (f"{d}MB" if d else "-")
        ssh   = "✓" if h.get("has_ssh")        else "-"
        dock  = "✓" if h.get("has_containers") else "-"
        aon   = "✓" if h.get("always_on")      else "-"
        src   = h.get("source", "")[:16]
        print(f"{i+1:<3} {h['name'][:26]:<27} {rs:<7} {ds:<7} {ssh:^4} {dock:^7} {aon:^5} {src:<17} {h['url']}")

    print("━" * W)


def main():
    args = sys.argv[1:]
    sort  = "gem"
    limit = None
    refresh = "--refresh" in args or "-r" in args

    for a in args:
        if a in ("power", "popular", "underground", "gem"): sort = a
        if a.isdigit(): limit = int(a)

    if refresh or not os.path.exists(os.path.expanduser("~/.cache/cez/vps_crawled.json")):
        hosts = run_crawler()
    else:
        cache = os.path.expanduser("~/.cache/cez/vps_crawled.json")
        with open(cache, encoding="utf-8") as f:
            hosts = json.load(f)
        print(f"[cache] {len(hosts)} сервисов загружено из кэша. Используй --refresh для обновления.")

    print_table(hosts, limit=limit, sort=sort)


if __name__ == "__main__":
    main()
