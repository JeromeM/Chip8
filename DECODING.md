# Anatomie d'un opcode CHIP-8 — cheat sheet visuelle

> À lire avec [INSTRUCTIONS.md](INSTRUCTIONS.md) ouvert à côté.

## 1. Un opcode = 16 bits = 4 nibbles = 4 chiffres hexa

```
                  16 bits = 2 octets = 4 nibbles
            ┌─────────────────────────────────────┐
opcode  =   │ nibble 1 │ nibble 2 │ nibble 3 │ nibble 4 │
            │   4 bits │  4 bits  │  4 bits  │  4 bits  │
            └─────────────────────────────────────┘

Exemple : 0xA22A
            ┌──────┬──────┬──────┬──────┐
            │  A   │  2   │  2   │  A   │   <- 4 chiffres hexa
            │ 1010 │ 0010 │ 0010 │ 1010 │   <- bits
            └──────┴──────┴──────┴──────┘
              ↑      ↑      ↑      ↑
            pos.    pos.   pos.   pos.
            12-15   8-11   4-7    0-3
```

**Règle d'or :** 1 chiffre hexa = 1 nibble = 4 bits.

## 2. Le rôle de chaque nibble

```
        +─────────+─────────+─────────+─────────+
opcode  │ nibble1 │ nibble2 │ nibble3 │ nibble4 │
        +─────────+─────────+─────────+─────────+
            ↑         ↑         ↑         ↑
         "famille"    X         Y       sous-type
       (type d'op)            ou ce qui reste
                                   (selon famille)
```

- **nibble 1** : la **famille** de l'opcode. C'est lui qui te dit "ah, c'est un saut" ou "c'est de l'arithmétique". → `match` dessus.
- **nibble 2** : presque toujours **`X`**, l'index d'un registre `Vx` (0..F).
- **nibble 3** : souvent **`Y`**, l'index d'un autre registre `Vy`. Parfois fait partie de `NN`.
- **nibble 4** : souvent **`N`** (une petite valeur 0..15), ou un sous-type d'opcode pour la famille `8XYn` et `FX**`.

## 3. Les variables qu'on extrait

C'est ce que ton code fait dans la phase **DECODE** :

```
opcode = ABCD (en hexa)

  ┌────────┬────────┬────────┬────────┐
  │   A    │   B    │   C    │   D    │
  └────────┴────────┴────────┴────────┘

  nibbles.0 = A          → famille (rarement utilisée)
  nibbles.1 = B  → x     → "index registre Vx"
  nibbles.2 = C  → y     → "index registre Vy"
  nibbles.3 = D  → n     → "valeur 4 bits" ou "sous-type"

  nn  = CD               → octet immédiat (8 bits)
  nnn = BCD              → adresse 12 bits
```

### Comment chaque variable est extraite (en code)

```rust
// opcode = 0xABCD
let nibbles = (
    (opcode & 0xF000) >> 12,  // A
    (opcode & 0x0F00) >> 8,   // B
    (opcode & 0x00F0) >> 4,   // C
    (opcode & 0x000F),        // D
);
let nnn = opcode & 0x0FFF;          // BCD (les 12 bits du bas)
let nn  = (opcode & 0x00FF) as u8;  // CD  (les 8 bits du bas)
let x   = nibbles.1 as usize;       // B
let y   = nibbles.2 as usize;       // C
let n   = nibbles.3 as usize;       // D
```

### Astuce mnémotechnique

```
ABCD
│└┴┴── nnn = adresse 12 bits (BCD)
│ └┴── nn  = octet 8 bits   (CD)
│  └── n   = 1 nibble       (D)
│
└──── famille (A)
```

## 4. Exemples concrets — opcodes décomposés

### `0xA22A` → ANNN (I = NNN)

```
A     2     2     A
↑     └─────┴─────┴── nnn = 0x22A
└─────────────────── famille A : "I = NNN"

Action exécutée :  self.i = 0x22A
```

### `0x6A07` → 6XNN (Vx = NN)

```
6     A     0     7
↑     ↑     └──┴── nn = 0x07
│     └──────── x = 0xA       (registre VA)
└────────── famille 6 : "Vx = NN"

Action :  self.v[0xA] = 0x07
                (Vx)     (NN)
```

### `0xD01F` → DXYN (dessine sprite)

```
D     0     1     F
↑     ↑     ↑     ↑
│     │     │     └── n = 0xF (hauteur du sprite : 15 lignes)
│     │     └── y = 0 (registre V1)
│     └── x = 0 (registre V0)
└── famille D : "draw"

Action :  dessine 15 lignes à (V[0], V[1]), depuis I
```

### `0x83A4` → 8XY4 (Vx += Vy)

```
8     3     A     4
↑     ↑     ↑     ↑
│     │     │     └── sous-type 4 : "ADD"
│     │     └── y = 0xA (registre VA)
│     └── x = 3 (registre V3)
└── famille 8 : "arithmétique sur registres"

Action :  V[3] += V[A], et VF = carry
```

### `0xFB33` → FX33 (BCD)

```
F     B     3     3
↑     ↑     └─┴── sous-type 33 : "BCD"
│     └── x = 0xB (registre VB)
└── famille F : "système / I / timers / mémoire"

Action :  memory[I..I+3] = chiffres décimaux de V[B]
```

## 5. La carte des familles

```
opcode = 1er-nibble + reste
         ─────────────────
nibble1 = 0   → spécial selon le reste :
                  0x00E0 → CLS
                  0x00EE → RET
                  0x0NNN → SYS (ignoré)

nibble1 = 1   → 1NNN  JP   pc = nnn
nibble1 = 2   → 2NNN  CALL push pc, pc = nnn
nibble1 = 3   → 3XNN  SE   skip si V[x] == nn
nibble1 = 4   → 4XNN  SNE  skip si V[x] != nn
nibble1 = 5   → 5XY0  SE   skip si V[x] == V[y]
nibble1 = 6   → 6XNN  LD   V[x] = nn
nibble1 = 7   → 7XNN  ADD  V[x] += nn   (sans carry)
nibble1 = 8   → 8XY*  arithmétique entre V[x] et V[y]
                           sous-type = dernier nibble
nibble1 = 9   → 9XY0  SNE  skip si V[x] != V[y]
nibble1 = A   → ANNN  LD   I = nnn
nibble1 = B   → BNNN  JP   pc = nnn + V[0]
nibble1 = C   → CXNN  RND  V[x] = random & nn
nibble1 = D   → DXYN  DRW  dessine sprite N lignes
nibble1 = E   → EX*   clavier (skip si key Vx pressée/non)
nibble1 = F   → FX*   système, timers, mémoire, I
```

## 6. Comment lire un opcode dans la tête en 3 secondes

1. **Regarde le 1er chiffre hexa** → trouve la famille dans la liste ci-dessus.
2. **Si la famille a "X"** → le 2e chiffre = registre Vx.
3. **Si la famille a "Y"** → le 3e chiffre = registre Vy.
4. **Le reste** est soit `N`, `NN`, ou `NNN` selon ce que la spec dit.

Exemple à blanc : `0x7A12`

> "Famille 7. 7XNN = Vx += NN. X=A, NN=12. → V[A] += 0x12."

## 7. Pourquoi tant de quirks dans le decode ?

Parce que CHIP-8 est **minuscule** (~35 instructions) et que ses concepteurs ont voulu tout faire tenir dans 16 bits. Ils ont donc surchargé chaque nibble selon l'opcode. C'est pour ça qu'il faut **matcher des patterns** (parfois sur tous les nibbles, parfois juste sur le premier).

Les familles "complexes" sont :
- **`0`** : 2 opcodes différents qui partagent le même nibble 1.
- **`5` et `9`** : exigent que le 4e nibble soit `0`.
- **`8`** : 9 opcodes différents qui partagent le nibble 1 mais ont des nibbles 4 différents (0..7, E).
- **`E` et `F`** : plusieurs opcodes différents, distingués par les **deux derniers nibbles** combinés (`9E`, `A1`, `07`, `15`, `1E`, `29`, `33`, `55`, `65`, etc.).

C'est pour ça que dans le `match` Rust on voit parfois `(0x8, _, _, 0x4)` (matche un nibble précis tout au bout) ou `(0xF, _, 0x3, 0x3)` (matche les deux derniers).
