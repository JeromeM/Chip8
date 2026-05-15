# Référence des instructions CHIP-8

Doc de référence des 35 instructions du CHIP-8 original.

## Notation

Chaque opcode fait **16 bits = 4 nibbles** (1 nibble = 4 bits = 1 chiffre hexa).

| Symbole | Signification | Extraction depuis l'opcode |
|---|---|---|
| `X` | Index d'un registre Vx (0..F) | 2e nibble — `(opcode & 0x0F00) >> 8` |
| `Y` | Index d'un registre Vy (0..F) | 3e nibble — `(opcode & 0x00F0) >> 4` |
| `N` | Valeur 4 bits (0..F) | 4e nibble — `opcode & 0x000F` |
| `NN` | Valeur immédiate 8 bits | 8 bits bas — `(opcode & 0x00FF) as u8` |
| `NNN` | Adresse 12 bits | 12 bits bas — `opcode & 0x0FFF` |

## Légende du statut

- ✅ Implémenté
- 🚧 En cours / prochain
- ⏳ Pas encore

## Système

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `0NNN` | `SYS NNN` | Appelle une routine machine native (ignoré par les interpréteurs modernes — ne fais rien) | ⏳ |
| `00E0` | `CLS` | Efface l'écran (toute la display à `false`) | ✅ |
| `00EE` | `RET` | Retour de sous-routine : `pc = stack[sp]`, `sp -= 1` | ✅ |

## Saut et appel

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `1NNN` | `JP NNN` | Saute à l'adresse `NNN` (`pc = NNN`) | ✅ |
| `2NNN` | `CALL NNN` | Appelle la sous-routine `NNN` : `sp += 1`, `stack[sp] = pc`, `pc = NNN` | ✅ |
| `BNNN` | `JP V0, NNN` | Saute à `NNN + V0` | ✅ |

## Branchements conditionnels (skip 1 instruction si vrai)

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `3XNN` | `SE Vx, NN` | Skip si `Vx == NN` | ✅ |
| `4XNN` | `SNE Vx, NN` | Skip si `Vx != NN` | ✅ |
| `5XY0` | `SE Vx, Vy` | Skip si `Vx == Vy` | ✅ |
| `9XY0` | `SNE Vx, Vy` | Skip si `Vx != Vy` | ✅ |

`Skip` = `pc += 2` (saute la prochaine instruction).

## Chargement de valeurs immédiates

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `6XNN` | `LD Vx, NN` | `Vx = NN` | ✅ |
| `7XNN` | `ADD Vx, NN` | `Vx = Vx + NN` (wrap u8, **ne touche pas à VF**) | ✅ |
| `ANNN` | `LD I, NNN` | `I = NNN` | ✅ |
| `CXNN` | `RND Vx, NN` | `Vx = random_u8() & NN` | ✅ |

## Opérations sur les registres (`8XY*`)

Toutes opèrent entre `Vx` et `Vy`. Plusieurs modifient `VF` (flag).

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `8XY0` | `LD Vx, Vy` | `Vx = Vy` | ✅ |
| `8XY1` | `OR Vx, Vy` | `Vx = Vx \| Vy` | ✅ |
| `8XY2` | `AND Vx, Vy` | `Vx = Vx & Vy` | ✅ |
| `8XY3` | `XOR Vx, Vy` | `Vx = Vx ^ Vy` | ✅ |
| `8XY4` | `ADD Vx, Vy` | `Vx = Vx + Vy`, `VF = 1 si carry sinon 0` | ✅ |
| `8XY5` | `SUB Vx, Vy` | `Vx = Vx - Vy`, `VF = 1 si pas de borrow sinon 0` | ✅ |
| `8XY6` | `SHR Vx` | `VF = Vx & 1`, puis `Vx >>= 1` | ✅ |
| `8XY7` | `SUBN Vx, Vy` | `Vx = Vy - Vx`, `VF = 1 si pas de borrow sinon 0` | ✅ |
| `8XYE` | `SHL Vx` | `VF = (Vx >> 7) & 1`, puis `Vx <<= 1` | ✅ |

> ⚠️ `SHR` et `SHL` ont deux variantes historiques (originale COSMAC vs SUPER-CHIP). Ça vaut le coup de se documenter quand on y arrive.

## Affichage

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `DXYN` | `DRW Vx, Vy, N` | Dessine un sprite de `N` octets de haut, à la position `(Vx, Vy)`, depuis l'adresse `I` en mémoire. **XOR** sur l'écran. `VF = 1` si collision (pixel allumé éteint), sinon `0`. | ✅ |

C'est **l'opcode le plus complexe** de tout CHIP-8 — il va valoir le coup de prendre son temps dessus.

## Entrée clavier

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `EX9E` | `SKP Vx` | Skip si la touche dont le code est `Vx` est enfoncée | ✅ |
| `EXA1` | `SKNP Vx` | Skip si la touche dont le code est `Vx` n'est PAS enfoncée | ✅ |
| `FX0A` | `LD Vx, K` | **Bloque** jusqu'à ce qu'une touche soit pressée, stocke son code dans `Vx` | ✅ |

## Timers

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `FX07` | `LD Vx, DT` | `Vx = delay_timer` | ✅ |
| `FX15` | `LD DT, Vx` | `delay_timer = Vx` | ✅ |
| `FX18` | `LD ST, Vx` | `sound_timer = Vx` | ✅ |

Les deux timers se décrémentent à **60 Hz** tant qu'ils sont > 0. `sound_timer > 0` doit produire un bip.

## Manipulation de `I` et de la mémoire

| Opcode | Mnémonique | Description | Statut |
|---|---|---|---|
| `FX1E` | `ADD I, Vx` | `I = I + Vx` | ✅ |
| `FX29` | `LD F, Vx` | `I = adresse du sprite du caractère hexa Vx` (table de fonts intégrée) | ✅ |
| `FX33` | `LD B, Vx` | Stocke la représentation BCD de `Vx` dans `memory[I]`, `memory[I+1]`, `memory[I+2]` (centaines, dizaines, unités) | ✅ |
| `FX55` | `LD [I], Vx` | Sauvegarde `V0..=Vx` dans `memory[I..]` | ✅ |
| `FX65` | `LD Vx, [I]` | Charge `V0..=Vx` depuis `memory[I..]` | ✅ |

## Algo général de l'exécution

```text
loop {
    opcode = fetch()           // 2 octets à memory[pc], big endian
    pc += 2
    nibbles = decode(opcode)
    execute(nibbles)
}
```

Cadence cible : **~700 instructions par seconde**, indépendamment du rafraîchissement de l'écran (60 Hz pour les timers).

## Sources canoniques

- [Cowgod's Chip-8 Technical Reference v1.0](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM) — la référence historique, la plus citée.
- [Tobias Langhoff — Building a CHIP-8 emulator](https://tobiasvl.github.io/blog/write-a-chip-8-emulator/) — tutoriel pédagogique très clair.
- [Wikipedia — CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) — vue d'ensemble et histoire.

## ROMs de test recommandées (dans l'ordre)

1. **IBM Logo** — n'utilise que `00E0`, `1NNN`, `6XNN`, `7XNN`, `ANNN`, `DXYN`. Le tout premier test.
2. **BC_test** — vérifie le bon comportement de presque toutes les instructions arithmétiques.
3. **test_opcode.ch8** (Skosulor / corax) — la suite de tests la plus utilisée.
4. **Quirks Test** — pour valider les variantes d'opcodes (COSMAC vs SUPER-CHIP).
