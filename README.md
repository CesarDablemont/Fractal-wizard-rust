# FractalWizard

Un éditeur d'images fractales interactif, rewrite en Rust du projet original [FractalWizard](https://github.com/ClementBRISSON/FractalWizard) en C++.


## About

**Figure** — éditeur de formes géométriques (polygones, formes libres, triangles équilatéraux) avec gizmo de transformation, grille et snap.

**Pattern** — définissez les transformations IFS (translation, rotation, échelle) qui génèrent la fractale. Calcul de dimension estimée.

**Initiale** — positionnez les figures de départ avant itération.

**Fractale** — génération IFS, 3 modes de champ de densité (Displacement, Contraction, Iteration), calcul de dimension par box-counting (spectre D_q), marches aléatoires avec statistiques (nombre de Polya), heatmaps globales/individuelles, animateur et export CSV des points.

**Général** :
- Canvas interactif (pan, zoom, grille adaptative, snap)
- Gizmo de transformation 3 axes
- exemples inclus (Koch, Sierpinski, tapis, etc.)
- Formats : `.firfw` (figure), `.ptnfw` (pattern), `.filfw` (initiale), `.ftlfw` (projet complet)

## Installation

### Binaire pré-compilé (recommandé)

Télécharge la dernière version depuis [GitHub Releases](https://github.com/CesarDablemont/Fractal-wizard-rust/releases/latest) :

| Plateforme | Fichier |
|---|---|
| Linux | `fractal-wizard-x86_64-linux.tar.gz` |
| Windows | `fractal-wizard-x86_64-windows.zip` |

**Linux :**
```sh
tar xzf fractal-wizard-x86_64-linux.tar.gz
./fractal-wizard
```

**Windows :**
Extrais le `.zip` et lance `fractal-wizard.exe`.

### Depuis les sources

**Rust toolchain requis** → [rustup.rs](https://rustup.rs)

```sh
git clone https://github.com/CesarDablemont/Fractal-wizard-rust.git
cd Fractal-wizard-rust
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
