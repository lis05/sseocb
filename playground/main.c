extern int printf(const char *fmt, ...);

static int math_mul(int a, int b) {
    int res = 0;
    int neg = 0;
    if (a < 0) { a = -a; neg = !neg; }
    if (b < 0) { b = -b; neg = !neg; }
    while (b > 0) {
        if (b & 1) res += a;
        a <<= 1;
        b >>= 1;
    }
    return neg ? -res : res;
}

static int math_div(int a, int b) {
    if (b == 0) return 0;
    int neg = 0;
    if (a < 0) { a = -a; neg = !neg; }
    if (b < 0) { b = -b; neg = !neg; }
    int q = 0;
    for (int i = 30; i >= 0; i--) {
        if ((a >> i) >= b) {
            a -= (b << i);
            q |= (1 << i);
        }
    }
    return neg ? -q : q;
}

static int math_mod(int a, int b) {
    if (b == 0) return 0;
    return a - math_mul(math_div(a, b), b);
}

static int math_abs(int x) {
    return x < 0 ? -x : x;
}

static int math_max(int a, int b) {
    return a > b ? a : b;
}

static int math_min(int a, int b) {
    return a < b ? a : b;
}

static int math_clamp(int val, int low, int high) {
    if (val < low) return low;
    if (val > high) return high;
    return val;
}

static unsigned int s_rng_seed = 0x98765432;

static unsigned int rng_step(void) {
    unsigned int x = s_rng_seed;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    s_rng_seed = x;
    return x;
}

static int rng_in_range(int low, int high) {
    if (low >= high) return low;
    int range = high - low + 1;
    int val = rng_step() & 0x7FFFFFFF;
    return low + math_mod(val, range);
}

typedef struct {
    int x;
    int y;
} Point;

typedef struct {
    int x;
    int y;
    int w;
    int h;
} Rect;

typedef struct {
    int id;
    int kind;
    int value;
    int bonus_atk;
    int bonus_def;
    int bonus_hp;
    int equipped;
} Item;

typedef struct {
    int x;
    int y;
    int hp;
    int max_hp;
    int mana;
    int max_mana;
    int atk;
    int def;
    int gold;
    int exp;
    int level;
    int kills;
    int turns;
    int floor;
    int score;
    int alive;
    Item inv[16];
    int inv_count;
} Hero;

typedef struct Monster Monster;
typedef void (*monster_ai_fn)(Monster *self, Hero *hero);
typedef int (*monster_hit_fn)(Monster *self, int raw_damage);
typedef void (*monster_drop_fn)(Monster *self, Hero *hero);

struct Monster {
    int x;
    int y;
    int hp;
    int max_hp;
    int atk;
    int def;
    int kind;
    int state;
    int alert;
    int alive;
    monster_ai_fn ai;
    monster_hit_fn on_hit;
    monster_drop_fn on_death;
};

typedef void (*spell_fn)(Hero *hero, int target_x, int target_y);
typedef void (*tile_fn)(Hero *hero, int tx, int ty);

static char s_grid[24][24];
static char s_fov[24][24];
static Hero s_hero;
static Monster s_monsters[12];
static Item s_ground_items[8];
static int s_ground_count = 0;
static Rect s_rooms[8];
static int s_room_count = 0;

void bresenham_los(int x0, int y0, int x1, int y1, int max_dist) {
    int dx = math_abs(x1 - x0);
    int dy = math_abs(y1 - y0);
    int sx = (x0 < x1) ? 1 : -1;
    int sy = (y0 < y1) ? 1 : -1;
    int err = dx - dy;
    int cx = x0;
    int cy = y0;
    int steps = 0;

    while (steps <= max_dist) {
        if (cx >= 0 && cx < 24 && cy >= 0 && cy < 24) {
            s_fov[cy][cx] = 1;
            if (s_grid[cy][cx] == '#') {
                break;
            }
        }
        if (cx == x1 && cy == y1) break;
        int e2 = err << 1;
        if (e2 > -dy) {
            err -= dy;
            cx += sx;
        }
        if (e2 < dx) {
            err += dx;
            cy += sy;
        }
        steps++;
    }
}

void compute_fov(int hx, int hy, int radius) {
    for (int y = 0; y < 24; y++) {
        for (int x = 0; x < 24; x++) {
            s_fov[y][x] = 0;
        }
    }
    for (int x = 0; x < 24; x++) {
        bresenham_los(hx, hy, x, 0, radius);
        bresenham_los(hx, hy, x, 23, radius);
    }
    for (int y = 0; y < 24; y++) {
        bresenham_los(hx, hy, 0, y, radius);
        bresenham_los(hx, hy, 23, y, radius);
    }
}

int bfs_next_step(int sx, int sy, int gx, int gy, int *out_x, int *out_y) {
    if (sx == gx && sy == gy) {
        *out_x = sx;
        *out_y = sy;
        return 1;
    }
    int parent_x[24][24];
    int parent_y[24][24];
    char visited[24][24];
    for (int y = 0; y < 24; y++) {
        for (int x = 0; x < 24; x++) {
            visited[y][x] = 0;
            parent_x[y][x] = -1;
            parent_y[y][x] = -1;
        }
    }

    Point queue[256];
    int head = 0;
    int tail = 0;

    queue[tail].x = sx;
    queue[tail].y = sy;
    tail++;
    visited[sy][sx] = 1;

    int found = 0;
    int dirs[4][2] = {{0, -1}, {0, 1}, {-1, 0}, {1, 0}};

    while (head < tail && head < 250) {
        Point curr = queue[head++];
        if (curr.x == gx && curr.y == gy) {
            found = 1;
            break;
        }
        for (int i = 0; i < 4; i++) {
            int nx = curr.x + dirs[i][0];
            int ny = curr.y + dirs[i][1];
            if (nx >= 0 && nx < 24 && ny >= 0 && ny < 24) {
                if (!visited[ny][nx] && (s_grid[ny][nx] != '#' || (nx == gx && ny == gy))) {
                    visited[ny][nx] = 1;
                    parent_x[ny][nx] = curr.x;
                    parent_y[ny][nx] = curr.y;
                    queue[tail].x = nx;
                    queue[tail].y = ny;
                    tail++;
                }
            }
        }
    }

    if (!found) {
        *out_x = sx;
        *out_y = sy;
        return 0;
    }

    int cx = gx;
    int cy = gy;
    while (parent_x[cy][cx] != sx || parent_y[cy][cx] != sy) {
        int px = parent_x[cy][cx];
        int py = parent_y[cy][cx];
        if (px == -1 || py == -1) break;
        cx = px;
        cy = py;
    }
    *out_x = cx;
    *out_y = cy;
    return 1;
}

void ai_patrol_slime(Monster *self, Hero *hero) {
    if (!self->alive) return;
    int d = math_abs(self->x - hero->x) + math_abs(self->y - hero->y);
    if (d <= 1) {
        int dmg = math_max(1, self->atk - hero->def);
        hero->hp = math_max(0, hero->hp - dmg);
        return;
    }
    int nx, ny;
    if (bfs_next_step(self->x, self->y, hero->x, hero->y, &nx, &ny)) {
        self->x = nx;
        self->y = ny;
    }
}

void ai_archer_goblin(Monster *self, Hero *hero) {
    if (!self->alive) return;
    int d = math_abs(self->x - hero->x) + math_abs(self->y - hero->y);
    if (d >= 2 && d <= 5) {
        int dmg = math_max(1, self->atk - hero->def);
        hero->hp = math_max(0, hero->hp - dmg);
        self->alert = 1;
    } else if (d < 2) {
        int away_x = self->x + (self->x - hero->x);
        int away_y = self->y + (self->y - hero->y);
        away_x = math_clamp(away_x, 1, 22);
        away_y = math_clamp(away_y, 1, 22);
        if (s_grid[away_y][away_x] != '#') {
            self->x = away_x;
            self->y = away_y;
        }
    } else {
        int nx, ny;
        if (bfs_next_step(self->x, self->y, hero->x, hero->y, &nx, &ny)) {
            self->x = nx;
            self->y = ny;
        }
    }
}

void ai_shadow_assassin(Monster *self, Hero *hero) {
    if (!self->alive) return;
    int d = math_abs(self->x - hero->x) + math_abs(self->y - hero->y);
    if (d > 6) {
        self->state = 0;
        return;
    }
    self->state = 1;
    if (d == 1) {
        int crit = (self->state == 1) ? self->atk << 1 : self->atk;
        int dmg = math_max(2, crit - hero->def);
        hero->hp = math_max(0, hero->hp - dmg);
    } else {
        int nx, ny;
        if (bfs_next_step(self->x, self->y, hero->x, hero->y, &nx, &ny)) {
            self->x = nx;
            self->y = ny;
        }
    }
}

void ai_undead_necromancer(Monster *self, Hero *hero) {
    if (!self->alive) return;
    int d = math_abs(self->x - hero->x) + math_abs(self->y - hero->y);
    if (d <= 4 && self->alert < 3) {
        self->alert++;
        for (int i = 0; i < 12; i++) {
            if (!s_monsters[i].alive && s_monsters[i].kind == 0) {
                s_monsters[i].alive = 1;
                s_monsters[i].hp = s_monsters[i].max_hp >> 1;
                s_monsters[i].x = math_clamp(self->x + rng_in_range(-1, 1), 1, 22);
                s_monsters[i].y = math_clamp(self->y + rng_in_range(-1, 1), 1, 22);
                break;
            }
        }
    }
    int nx, ny;
    if (bfs_next_step(self->x, self->y, hero->x, hero->y, &nx, &ny)) {
        self->x = nx;
        self->y = ny;
    }
}

void ai_dungeon_dragon(Monster *self, Hero *hero) {
    if (!self->alive) return;
    int d = math_abs(self->x - hero->x) + math_abs(self->y - hero->y);
    if (d == 1) {
        int dmg = math_max(3, math_mul(self->atk, 2) - hero->def);
        hero->hp = math_max(0, hero->hp - dmg);
    } else if (d <= 6) {
        int breath = self->atk;
        hero->hp = math_max(0, hero->hp - breath);
        if (self->hp < (self->max_hp >> 1)) {
            self->atk += 2;
        }
    } else {
        int nx, ny;
        if (bfs_next_step(self->x, self->y, hero->x, hero->y, &nx, &ny)) {
            self->x = nx;
            self->y = ny;
        }
    }
}

int hit_normal(Monster *self, int raw_damage) {
    int effective = math_max(1, raw_damage - self->def);
    self->hp -= effective;
    if (self->hp <= 0) {
        self->hp = 0;
        self->alive = 0;
        return 1;
    }
    return 0;
}

int hit_shielded(Monster *self, int raw_damage) {
    int blocked = raw_damage - (self->def << 1);
    int effective = math_max(1, blocked);
    self->hp -= effective;
    if (self->hp <= 0) {
        self->hp = 0;
        self->alive = 0;
        return 1;
    }
    return 0;
}

int hit_dodging(Monster *self, int raw_damage) {
    if ((rng_step() & 3) == 0) {
        return 0;
    }
    int effective = math_max(1, raw_damage - self->def);
    self->hp -= effective;
    if (self->hp <= 0) {
        self->hp = 0;
        self->alive = 0;
        return 1;
    }
    return 0;
}

void drop_common(Monster *self, Hero *hero) {
    hero->gold += rng_in_range(5, 20);
    hero->exp += 25;
    hero->kills++;
}

void drop_rare(Monster *self, Hero *hero) {
    hero->gold += rng_in_range(20, 60);
    hero->exp += 75;
    hero->kills++;
    if (hero->inv_count < 16) {
        Item item;
        item.id = hero->kills;
        item.kind = 1;
        item.value = 50;
        item.bonus_atk = 3;
        item.bonus_def = 1;
        item.bonus_hp = 10;
        item.equipped = 0;
        hero->inv[hero->inv_count++] = item;
    }
}

void drop_boss(Monster *self, Hero *hero) {
    hero->gold += 500;
    hero->exp += 500;
    hero->kills++;
    hero->score += 1000;
    if (hero->inv_count < 16) {
        Item weapon;
        weapon.id = 999;
        weapon.kind = 2;
        weapon.value = 500;
        weapon.bonus_atk = 15;
        weapon.bonus_def = 5;
        weapon.bonus_hp = 50;
        weapon.equipped = 1;
        hero->inv[hero->inv_count++] = weapon;
        hero->atk += weapon.bonus_atk;
        hero->def += weapon.bonus_def;
    }
}

void cast_fireball(Hero *hero, int tx, int ty) {
    if (hero->mana < 10) return;
    hero->mana -= 10;
    for (int i = 0; i < 12; i++) {
        if (s_monsters[i].alive) {
            int d = math_abs(s_monsters[i].x - tx) + math_abs(s_monsters[i].y - ty);
            if (d <= 2) {
                int dmg = math_max(5, 20 - math_mul(d, 5));
                if (s_monsters[i].on_hit(&s_monsters[i], dmg)) {
                    s_monsters[i].on_death(&s_monsters[i], hero);
                }
            }
        }
    }
}

void cast_healing(Hero *hero, int tx, int ty) {
    if (hero->mana < 8) return;
    hero->mana -= 8;
    hero->hp = math_min(hero->max_hp, hero->hp + 30);
}

void cast_blink(Hero *hero, int tx, int ty) {
    if (hero->mana < 12) return;
    tx = math_clamp(tx, 1, 22);
    ty = math_clamp(ty, 1, 22);
    if (s_grid[ty][tx] != '#') {
        hero->mana -= 12;
        hero->x = tx;
        hero->y = ty;
    }
}

void tile_heal_fountain(Hero *hero, int tx, int ty) {
    hero->hp = hero->max_hp;
    hero->mana = hero->max_mana;
}

void tile_spike_trap(Hero *hero, int tx, int ty) {
    hero->hp = math_max(0, hero->hp - 15);
}

void tile_gold_stash(Hero *hero, int tx, int ty) {
    hero->gold += 75;
    hero->score += 150;
    s_grid[ty][tx] = '.';
}

void tile_portal_exit(Hero *hero, int tx, int ty) {
    hero->floor++;
    hero->score += 500;
}

monster_ai_fn g_ai_table[5] = {
    ai_patrol_slime,
    ai_archer_goblin,
    ai_shadow_assassin,
    ai_undead_necromancer,
    ai_dungeon_dragon
};

monster_hit_fn g_hit_table[3] = {
    hit_normal,
    hit_shielded,
    hit_dodging
};

monster_drop_fn g_drop_table[3] = {
    drop_common,
    drop_rare,
    drop_boss
};

spell_fn g_spells[3] = {
    cast_fireball,
    cast_healing,
    cast_blink
};

tile_fn g_tile_handlers[4] = {
    tile_heal_fountain,
    tile_spike_trap,
    tile_gold_stash,
    tile_portal_exit
};

void generate_room(int idx, int x, int y, int w, int h) {
    s_rooms[idx].x = x;
    s_rooms[idx].y = y;
    s_rooms[idx].w = w;
    s_rooms[idx].h = h;
    for (int ry = y; ry < y + h; ry++) {
        for (int rx = x; rx < x + w; rx++) {
            if (rx > 0 && rx < 23 && ry > 0 && ry < 23) {
                s_grid[ry][rx] = '.';
            }
        }
    }
}

void connect_rooms(int x1, int y1, int x2, int y2) {
    int cx = x1;
    int cy = y1;
    while (cx != x2) {
        if (cx > 0 && cx < 23 && cy > 0 && cy < 23) {
            s_grid[cy][cx] = '.';
        }
        cx += (x2 > cx) ? 1 : -1;
    }
    while (cy != y2) {
        if (cx > 0 && cx < 23 && cy > 0 && cy < 23) {
            s_grid[cy][cx] = '.';
        }
        cy += (y2 > cy) ? 1 : -1;
    }
}

void build_dungeon(void) {
    for (int y = 0; y < 24; y++) {
        for (int x = 0; x < 24; x++) {
            s_grid[y][x] = '#';
        }
    }

    generate_room(0, 2, 2, 6, 5);
    generate_room(1, 14, 2, 7, 6);
    generate_room(2, 2, 14, 8, 6);
    generate_room(3, 13, 14, 8, 7);
    s_room_count = 4;

    connect_rooms(5, 4, 17, 5);
    connect_rooms(5, 4, 5, 17);
    connect_rooms(17, 5, 17, 17);
    connect_rooms(5, 17, 17, 17);

    s_grid[3][3] = 'F';
    s_grid[15][15] = 'T';
    s_grid[4][18] = '$';
    s_grid[18][18] = '>';
}

void spawn_monsters(void) {
    for (int i = 0; i < 12; i++) {
        int kind = math_mod(i, 5);
        s_monsters[i].kind = kind;
        s_monsters[i].alive = 1;
        s_monsters[i].alert = 0;
        s_monsters[i].state = 0;
        s_monsters[i].ai = g_ai_table[kind];

        if (kind == 4) {
            s_monsters[i].hp = 120;
            s_monsters[i].max_hp = 120;
            s_monsters[i].atk = 18;
            s_monsters[i].def = 6;
            s_monsters[i].x = 18;
            s_monsters[i].y = 17;
            s_monsters[i].on_hit = g_hit_table[1];
            s_monsters[i].on_death = g_drop_table[2];
        } else if (kind == 2) {
            s_monsters[i].hp = 30;
            s_monsters[i].max_hp = 30;
            s_monsters[i].atk = 12;
            s_monsters[i].def = 2;
            s_monsters[i].x = rng_in_range(14, 20);
            s_monsters[i].y = rng_in_range(3, 7);
            s_monsters[i].on_hit = g_hit_table[2];
            s_monsters[i].on_death = g_drop_table[1];
        } else {
            s_monsters[i].hp = 20 + math_mul(kind, 5);
            s_monsters[i].max_hp = s_monsters[i].hp;
            s_monsters[i].atk = 6 + math_mul(kind, 2);
            s_monsters[i].def = 1 + kind;
            s_monsters[i].x = rng_in_range(3, 10);
            s_monsters[i].y = rng_in_range(14, 19);
            s_monsters[i].on_hit = g_hit_table[0];
            s_monsters[i].on_death = g_drop_table[0];
        }
    }
}

void hero_init(void) {
    s_hero.x = 4;
    s_hero.y = 4;
    s_hero.hp = 80;
    s_hero.max_hp = 80;
    s_hero.mana = 40;
    s_hero.max_mana = 40;
    s_hero.atk = 10;
    s_hero.def = 3;
    s_hero.gold = 50;
    s_hero.exp = 0;
    s_hero.level = 1;
    s_hero.kills = 0;
    s_hero.turns = 0;
    s_hero.floor = 1;
    s_hero.alive = 1;
    s_hero.inv_count = 0;
}

void hero_gain_exp(int amount) {
    s_hero.exp += amount;
    while (s_hero.exp >= math_mul(s_hero.level, 100)) {
        s_hero.exp -= math_mul(s_hero.level, 100);
        s_hero.level++;
        s_hero.max_hp += 15;
        s_hero.hp = s_hero.max_hp;
        s_hero.max_mana += 10;
        s_hero.mana = s_hero.max_mana;
        s_hero.atk += 3;
        s_hero.def += 1;
    }
}

void hero_try_interact_tile(int tx, int ty) {
    char tile = s_grid[ty][tx];
    if (tile == 'F') {
        g_tile_handlers[0](&s_hero, tx, ty);
    } else if (tile == 'T') {
        g_tile_handlers[1](&s_hero, tx, ty);
    } else if (tile == '$') {
        g_tile_handlers[2](&s_hero, tx, ty);
    } else if (tile == '>') {
        g_tile_handlers[3](&s_hero, tx, ty);
    }
}

int hero_move_or_attack(int dx, int dy) {
    int nx = s_hero.x + dx;
    int ny = s_hero.y + dy;
    if (nx < 0 || nx >= 24 || ny < 0 || ny >= 24) return 0;
    if (s_grid[ny][nx] == '#') return 0;

    for (int i = 0; i < 12; i++) {
        if (s_monsters[i].alive && s_monsters[i].x == nx && s_monsters[i].y == ny) {
            int killed = s_monsters[i].on_hit(&s_monsters[i], s_hero.atk);
            if (killed) {
                s_monsters[i].on_death(&s_monsters[i], &s_hero);
                hero_gain_exp(30);
            }
            return 1;
        }
    }

    s_hero.x = nx;
    s_hero.y = ny;
    hero_try_interact_tile(nx, ny);
    return 1;
}

void execute_game_turn(int action_code, int param_x, int param_y) {
    if (!s_hero.alive) return;
    s_hero.turns++;

    if (action_code >= 0 && action_code <= 3) {
        int dirs[4][2] = {{0, -1}, {0, 1}, {-1, 0}, {1, 0}};
        hero_move_or_attack(dirs[action_code][0], dirs[action_code][1]);
    } else if (action_code == 4) {
        g_spells[0](&s_hero, param_x, param_y);
    } else if (action_code == 5) {
        g_spells[1](&s_hero, param_x, param_y);
    } else if (action_code == 6) {
        g_spells[2](&s_hero, param_x, param_y);
    }

    for (int i = 0; i < 12; i++) {
        if (s_monsters[i].alive && s_monsters[i].ai) {
            s_monsters[i].ai(&s_monsters[i], &s_hero);
        }
    }

    if (s_hero.hp <= 0) {
        s_hero.alive = 0;
    }
}

void display_hud(void) {
    printf("[Floor: %d | Turn: %d | Level: %d | HP: %d/%d | MP: %d/%d | ATK: %d | DEF: %d | Gold: %d]\n",
           s_hero.floor, s_hero.turns, s_hero.level,
           s_hero.hp, s_hero.max_hp, s_hero.mana, s_hero.max_mana,
           s_hero.atk, s_hero.def, s_hero.gold);
}

void display_map(void) {
    compute_fov(s_hero.x, s_hero.y, 8);
    for (int y = 0; y < 24; y++) {
        char row[25];
        for (int x = 0; x < 24; x++) {
            if (!s_fov[y][x]) {
                row[x] = ' ';
                continue;
            }
            if (s_hero.x == x && s_hero.y == y) {
                row[x] = '@';
                continue;
            }
            int mon_drawn = 0;
            for (int i = 0; i < 12; i++) {
                if (s_monsters[i].alive && s_monsters[i].x == x && s_monsters[i].y == y) {
                    if (s_monsters[i].kind == 4) row[x] = 'D';
                    else if (s_monsters[i].kind == 3) row[x] = 'N';
                    else if (s_monsters[i].kind == 2) row[x] = 'A';
                    else if (s_monsters[i].kind == 1) row[x] = 'G';
                    else row[x] = 'S';
                    mon_drawn = 1;
                    break;
                }
            }
            if (!mon_drawn) {
                row[x] = s_grid[y][x];
            }
        }
        row[24] = '\0';
        printf("%s\n", row);
    }
}

void hero_equip_item(Hero *h, int slot) {
    if (slot < 0 || slot >= h->inv_count) return;
    Item *it = &h->inv[slot];
    if (it->equipped) {
        it->equipped = 0;
        h->atk -= it->bonus_atk;
        h->def -= it->bonus_def;
        h->max_hp -= it->bonus_hp;
        if (h->hp > h->max_hp) h->hp = h->max_hp;
        printf("Unequipped item %d\n", it->id);
    } else {
        it->equipped = 1;
        h->atk += it->bonus_atk;
        h->def += it->bonus_def;
        h->max_hp += it->bonus_hp;
        h->hp += it->bonus_hp;
        printf("Equipped item %d (+%d atk, +%d def)\n", it->id, it->bonus_atk, it->bonus_def);
    }
}

int hero_craft_item(Hero *h, int recipe) {
    if (h->inv_count >= 16) return 0;
    if (recipe == 1 && h->gold >= 50) {
        h->gold -= 50;
        Item it;
        it.id = 101;
        it.kind = 1;
        it.value = 60;
        it.bonus_atk = 8;
        it.bonus_def = 2;
        it.bonus_hp = 10;
        it.equipped = 0;
        h->inv[h->inv_count++] = it;
        printf("Crafted Iron Sword\n");
        return 1;
    } else if (recipe == 2 && h->gold >= 80) {
        h->gold -= 80;
        Item it;
        it.id = 102;
        it.kind = 2;
        it.value = 100;
        it.bonus_atk = 1;
        it.bonus_def = 12;
        it.bonus_hp = 30;
        it.equipped = 0;
        h->inv[h->inv_count++] = it;
        printf("Crafted Steel Plate\n");
        return 1;
    } else if (recipe == 3 && h->gold >= 120) {
        h->gold -= 120;
        Item it;
        it.id = 103;
        it.kind = 3;
        it.value = 150;
        it.bonus_atk = 15;
        it.bonus_def = 8;
        it.bonus_hp = 20;
        it.equipped = 0;
        h->inv[h->inv_count++] = it;
        printf("Crafted Runed Blade\n");
        return 1;
    }
    return 0;
}

void shopkeeper_interact(Hero *h, int action, int item_idx) {
    if (action == 1) {
        if (h->gold >= 30 && h->inv_count < 16) {
            h->gold -= 30;
            Item pot;
            pot.id = 201;
            pot.kind = 0;
            pot.value = 15;
            pot.bonus_atk = 0;
            pot.bonus_def = 0;
            pot.bonus_hp = 0;
            pot.equipped = 0;
            h->inv[h->inv_count++] = pot;
            printf("Bought Health Potion\n");
        }
    } else if (action == 2) {
        if (item_idx >= 0 && item_idx < h->inv_count) {
            Item *it = &h->inv[item_idx];
            if (it->equipped) hero_equip_item(h, item_idx);
            h->gold += it->value;
            printf("Sold item %d for %d gold\n", it->id, it->value);
            for (int k = item_idx; k < h->inv_count - 1; k++) {
                h->inv[k] = h->inv[k + 1];
            }
            h->inv_count--;
        }
    }
}

int evaluate_achievements(Hero *h) {
    int points = 0;
    if (h->kills >= 1) points += 10;
    if (h->kills >= 5) points += 50;
    if (h->kills >= 10) points += 200;
    if (h->level >= 2) points += 20;
    if (h->level >= 5) points += 100;
    if (h->gold >= 100) points += 50;
    if (h->gold >= 500) points += 250;
    if (h->score >= 1000) points += 500;
    h->score += points;
    return points;
}

void _start(void) {
    build_dungeon();
    spawn_monsters();
    hero_init();

    hero_craft_item(&s_hero, 1);
    hero_equip_item(&s_hero, s_hero.inv_count - 1);
    shopkeeper_interact(&s_hero, 1, 0);

    display_hud();
    display_map();

    int script_actions[24] = {
        1, 1, 3, 3, 3, 1, 3, 3,
        4, 1, 1, 4, 5, 2, 2, 0,
        6, 1, 1, 3, 3, 1, 1, 3
    };

    for (int i = 0; i < 24; i++) {
        int act = script_actions[i];
        execute_game_turn(act, s_hero.x + 2, s_hero.y);
        display_hud();
        display_map();
        if (!s_hero.alive) {
            printf("Hero perished at turn %d!\n", s_hero.turns);
            break;
        }
    }

    int alive_count = 0;
    for (int i = 0; i < 12; i++) {
        if (s_monsters[i].alive) alive_count++;
    }
    int bonus = evaluate_achievements(&s_hero);
    printf("Dungeon Run Finished. Alive monsters: %d, Hero Level: %d, Kills: %d, Score: %d (+%d)\n",
           alive_count, s_hero.level, s_hero.kills, s_hero.score, bonus);

    while (1) {
    }
}
