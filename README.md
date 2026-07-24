# FractalWizard

Un éditeur d'images fractales interactif, rewrite en Rust du projet original [FractalWizard](https://github.com/ClementBRISSON/FractalWizard) en C++.


## About

- 4 éditeurs : Figure, Pattern, Initiale, Fractale
- Formes polygonales et libres avec gizmo de transformation
- Génération fractale par IFS avec calcul de dimension (box counting)
- Simulations de marche aléatoire avec statistiques et heatmaps
- Animation avec 5 modes de lecture
- Import/export au format JSON (`.firfw`, `.ptnfw`, `.filfw`, `.ftlfw`)

## Installation

**Rust toolchain requis.** ([rustup.rs](https://rustup.rs))

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Usage

```sh
cargo run
```

En release :

```sh
cargo run --release
```

## Workflow

1. **Figure** : créez une forme (polygone ou libre)
2. **Pattern** : envoyez la figure, ajoutez des transformations IFS
3. **Initiale** : positionnez les figures de départ
4. **Fractale** : générez et explorez (simulations, heatmaps, animation)

Bouton **➡ Envoyer** dans chaque éditeur pour transférer vers le suivant.

## Original

Rewrite de [FractalWizard](https://github.com/ClementBRISSON/FractalWizard) (C++).
