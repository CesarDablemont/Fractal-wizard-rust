use std::collections::HashMap;
use eframe::egui::{pos2, Pos2, Vec2};
use crate::shapes::shape::apply_transform;
use crate::types::{DensityMode, DensitySource, Line, ShapePatternData};
use super::dimension;

pub struct FractalResult {
    pub points: Vec<Pos2>,
    pub lines: Vec<Line>,
    pub dimension: f32,
    pub point_scale: Vec<f32>,
    pub box_counting: Option<dimension::BoxCountingResult>,
}

pub struct FractalConfig<'a> {
    pub get_points: &'a dyn Fn(Pos2, f32, f32) -> Vec<Pos2>,
    pub get_lines: &'a dyn Fn(Pos2, f32, f32) -> Vec<[Pos2; 2]>,
    pub pattern: &'a [ShapePatternData],
    pub initial: &'a [ShapePatternData],
    pub iterations: usize,
    pub regroup: bool,
    pub display_parent: bool,
    pub delta_radius: f32,
    pub density_sources: &'a [DensitySource],
}

/// Forme avec sa profondeur actuelle et sa profondeur maximale autorisée.
struct ShapeWithDepth {
    data: ShapePatternData,
    depth: usize,
    max_allowed: usize,
}

pub fn generate_fractal(config: &FractalConfig<'_>) -> FractalResult {
    let FractalConfig {
        get_points, get_lines, pattern, initial,
        iterations, regroup, display_parent, delta_radius, density_sources,
    } = config;

    // Séparer les sources selon leur mode
    let contraction_sources: Vec<&DensitySource> = density_sources
        .iter()
        .filter(|s| s.mode == DensityMode::Contraction)
        .collect();
    let iteration_sources: Vec<&DensitySource> = density_sources
        .iter()
        .filter(|s| s.mode == DensityMode::Iteration)
        .collect();
    let displacement_sources: Vec<&DensitySource> = density_sources
        .iter()
        .filter(|s| s.mode == DensityMode::Displacement)
        .collect();

    let has_contraction = !contraction_sources.is_empty();
    let has_iteration = !iteration_sources.is_empty();

    // Safety cap: limite le nombre d'itérations par branche pour éviter l'explosion exponentielle.
    let max_depth = iterations + 3;

    let mut all_shapes: Vec<ShapePatternData> = Vec::new();

    if has_iteration {
        // Mode Itération : chaque branche porte sa profondeur et son max autorisé.
        // On utilise une pile explicite (DFS) au lieu d'une boucle plate.
        let mut stack: Vec<ShapeWithDepth> = initial
            .iter()
            .map(|s| ShapeWithDepth {
                data: s.clone(),
                depth: 0,
                max_allowed: *iterations,
            })
            .collect();

        while let Some(item) = stack.pop() {
            if item.depth >= item.max_allowed {
                all_shapes.push(item.data);
                continue;
            }

            if *display_parent {
                all_shapes.push(item.data.clone());
            }

            for pat in *pattern {
                let base_scale = 1.0 / pat.scale;
                let transformed = apply_transform(
                    pat.translate,
                    Pos2::ZERO,
                    item.data.rotate,
                    Vec2::new(item.data.scale, item.data.scale),
                );
                let child_pos = pos2(
                    item.data.translate.x + transformed.x,
                    item.data.translate.y + transformed.y,
                );

                // Modulation de contraction (si sources de contraction présentes)
                let scale_modulation = if has_contraction {
                    compute_contraction_factor(child_pos, &contraction_sources)
                } else {
                    1.0
                };

                let child_scale = item.data.scale * base_scale * scale_modulation;
                let child_rotate = item.data.rotate + pat.rotate;

                // Recalculer le transform avec le scale modulé
                let transformed = apply_transform(
                    pat.translate,
                    Pos2::ZERO,
                    item.data.rotate,
                    Vec2::new(
                        item.data.scale * scale_modulation,
                        item.data.scale * scale_modulation,
                    ),
                );
                let child_translate = pos2(
                    item.data.translate.x + transformed.x,
                    item.data.translate.y + transformed.y,
                );

                // Bonus d'itérations selon la position
                // Le max autorisé augmente, mais la profondeur augmente toujours de 1.
                // Donc la branche converge toujours (depth augmente strictement).
                let iter_bonus = compute_iteration_bonus(child_pos, &iteration_sources);
                let child_max_allowed = (item.max_allowed + iter_bonus).min(max_depth);

                stack.push(ShapeWithDepth {
                    data: ShapePatternData {
                        translate: child_translate,
                        rotate: child_rotate,
                        scale: child_scale,
                    },
                    depth: item.depth + 1,
                    max_allowed: child_max_allowed,
                });
            }
        }
    } else {
        // Mode standard / contraction : boucle plate (comportement existant)
        let mut current = initial.to_vec();

        for _ in 0..*iterations {
            let mut children = Vec::new();
            for parent in &current {
                for pat in *pattern {
                    let base_scale = 1.0 / pat.scale;

                    let transformed = if has_contraction {
                        apply_transform(
                            pat.translate,
                            Pos2::ZERO,
                            parent.rotate,
                            Vec2::new(parent.scale, parent.scale),
                        )
                    } else {
                        Pos2::ZERO
                    };
                    let child_pos = if has_contraction {
                        pos2(
                            parent.translate.x + transformed.x,
                            parent.translate.y + transformed.y,
                        )
                    } else {
                        Pos2::ZERO
                    };

                    let scale_modulation = if has_contraction {
                        compute_contraction_factor(child_pos, &contraction_sources)
                    } else {
                        1.0
                    };

                    let child_scale = parent.scale * base_scale * scale_modulation;
                    let child_rotate = parent.rotate + pat.rotate;

                    let transformed = apply_transform(
                        pat.translate,
                        Pos2::ZERO,
                        parent.rotate,
                        Vec2::new(
                            parent.scale * scale_modulation,
                            parent.scale * scale_modulation,
                        ),
                    );
                    let child_translate = pos2(
                        parent.translate.x + transformed.x,
                        parent.translate.y + transformed.y,
                    );

                    children.push(ShapePatternData {
                        translate: child_translate,
                        rotate: child_rotate,
                        scale: child_scale,
                    });
                }
            }
            if *display_parent {
                all_shapes.extend(current);
            }
            current = children;
        }
        all_shapes.extend(current);
    }

    let mut final_points: Vec<Pos2> = Vec::new();
    let mut final_lines: Vec<Line> = Vec::new();
    let mut final_point_scale: Vec<f32> = Vec::new();
    let mut point_map: HashMap<u64, usize> = HashMap::new();

    for s in &all_shapes {
        let sp = get_points(s.translate, s.rotate, s.scale);
        let sl = get_lines(s.translate, s.rotate, s.scale);

        for &p in &sp {
            find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, p, s.scale);
        }
        for seg in &sl {
            let a_idx = find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, seg[0], s.scale);
            let b_idx = find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, seg[1], s.scale);
            final_lines.push([a_idx, b_idx]);
        }
    }

    if *regroup {
        let (canonical, merged_lines) = merge_vertices(&final_points, &final_lines);
        let mut remap: Vec<usize> = vec![usize::MAX; final_points.len()];
        let mut compacted_points: Vec<Pos2> = Vec::new();
        let mut compacted_scales: Vec<f32> = Vec::new();
        for i in 0..final_points.len() {
            let rep = canonical[i];
            if remap[rep] == usize::MAX {
                remap[rep] = compacted_points.len();
                compacted_points.push(final_points[rep]);
                compacted_scales.push(final_point_scale[rep]);
            }
            if final_point_scale[i] > compacted_scales[remap[rep]] {
                compacted_scales[remap[rep]] = final_point_scale[i];
            }
        }
        final_points = compacted_points;
        final_point_scale = compacted_scales;
        final_lines = merged_lines.iter().map(|l| [remap[l[0]], remap[l[1]]]).collect();
    }

    let dimension = if !pattern.is_empty() && pattern[0].scale > 1.0 {
        let n = pattern.len() as f32;
        let s = pattern[0].scale;
        n.log10() / s.log10()
    } else {
        0.0
    };

    if *delta_radius > 0.0 {
        use rand::Rng;
        let mut rng = rand::rng();
        for p in &mut final_points {
            let angle = rng.random::<f32>() * std::f32::consts::TAU;
            let r = *delta_radius * rng.random::<f32>().sqrt();
            p.x += r * angle.cos();
            p.y += r * angle.sin();
        }
    }

    if !displacement_sources.is_empty() {
        let disp_refs: Vec<DensitySource> = displacement_sources.into_iter().cloned().collect();
        apply_density_field(&mut final_points, &disp_refs);
    }

    // Box-counting sur les centres des copies (masse uniforme par copie)
    // plutôt que sur les points dédupliqués (masse non-uniforme à cause des sommets partagés).
    let shape_centers: Vec<Pos2> = all_shapes.iter().map(|s| s.translate).collect();
    let box_counting = dimension::box_counting(&shape_centers, *iterations);

    FractalResult {
        points: final_points,
        lines: final_lines,
        point_scale: final_point_scale,
        dimension,
        box_counting,
    }
}

/// Calcule le facteur de contraction modulé par les sources de densité.
fn compute_contraction_factor(pos: Pos2, sources: &[&DensitySource]) -> f32 {
    let mut modulation = 1.0;
    for source in sources {
        let dx = source.position.x - pos.x;
        let dy = source.position.y - pos.y;
        let dist_sq = dx * dx + dy * dy;
        let r = source.radius;
        if dist_sq < r * r && dist_sq > 0.0 {
            let dist = dist_sq.sqrt();
            let t = 1.0 - dist / r;
            let influence = t.powf(source.exponent) * source.force;
            modulation *= 1.0 + influence / 100.0;
        }
    }
    modulation.clamp(0.01, 100.0)
}

/// Calcule le bonus d'itérations pour une position donnée.
///
/// `force > 0` : itérations supplémentaires (plus de détail près de la source)
/// `force < 0` : itérations en moins (moins de détail près de la source)
///
/// Le bonus est arrondi à un entier. force=25 → 1 itération supplémentaire max,
/// force=50 → 2 itérations supplémentaires max.
fn compute_iteration_bonus(pos: Pos2, sources: &[&DensitySource]) -> usize {
    let mut total_bonus: f32 = 0.0;
    for source in sources {
        let dx = source.position.x - pos.x;
        let dy = source.position.y - pos.y;
        let dist_sq = dx * dx + dy * dy;
        let r = source.radius;
        if dist_sq < r * r && dist_sq > 0.0 {
            let dist = dist_sq.sqrt();
            let t = 1.0 - dist / r;
            let influence = t.powf(source.exponent) * source.force;
            // force=25 → 1 iter, force=50 → 2 iters
            total_bonus += influence / 25.0;
        }
    }
    if total_bonus > 0.0 {
        total_bonus.round().max(0.0) as usize
    } else {
        0
    }
}

fn point_key(p: Pos2) -> u64 {
    u64::from(p.x.to_bits()) ^ (u64::from(p.y.to_bits()) << 32)
}

fn find_or_add_point(
    points: &mut Vec<Pos2>,
    scales: &mut Vec<f32>,
    map: &mut HashMap<u64, usize>,
    p: Pos2,
    scale: f32,
) -> usize {
    let key = point_key(p);
    if let Some(&idx) = map.get(&key) {
        scales[idx] = scales[idx].max(scale);
        return idx;
    }
    let idx = points.len();
    points.push(p);
    scales.push(scale);
    map.insert(key, idx);
    idx
}

pub(crate) fn merge_vertices(points: &[Pos2], lines: &[Line]) -> (Vec<usize>, Vec<Line>) {
    let mut max_coord = 0.0f32;
    for p in points {
        max_coord = max_coord.max(p.x.abs()).max(p.y.abs());
    }
    let tolerance = (max_coord * 1e-5).max(1e-6);
    let tolerance_sq = tolerance * tolerance;

    let mut canonical: Vec<usize> = (0..points.len()).collect();
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for i in 0..points.len() {
        let cell = (
            (points[i].x / tolerance).floor() as i32,
            (points[i].y / tolerance).floor() as i32,
        );
        let mut merged = false;
        'cells: for cx in (cell.0 - 1)..=(cell.0 + 1) {
            for cy in (cell.1 - 1)..=(cell.1 + 1) {
                if let Some(entries) = grid.get(&(cx, cy)) {
                    for &j in entries {
                        let dx = points[i].x - points[j].x;
                        let dy = points[i].y - points[j].y;
                        if dx * dx + dy * dy < tolerance_sq {
                            canonical[i] = canonical[j];
                            merged = true;
                            break 'cells;
                        }
                    }
                }
            }
        }
        if !merged {
            grid.entry(cell).or_default().push(i);
        }
    }

    let lines: Vec<Line> = lines
        .iter()
        .map(|l| [canonical[l[0]], canonical[l[1]]])
        .filter(|l| l[0] != l[1])
        .collect();
    (canonical, lines)
}

fn apply_density_field(points: &mut [Pos2], sources: &[DensitySource]) {
    for p in points.iter_mut() {
        for source in sources {
            let dx = source.position.x - p.x;
            let dy = source.position.y - p.y;
            let dist_sq = dx * dx + dy * dy;
            let r = source.radius;
            if dist_sq < r * r && dist_sq > 0.0 {
                let dist = dist_sq.sqrt();
                let t = dist / r;
                let influence = t.powf(source.exponent) * source.force;
                let nx = dx / dist;
                let ny = dy / dist;
                p.x += nx * influence;
                p.y += ny * influence;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    #[test]
    fn sierpinski_dimension() {
        let pattern = vec![
            ShapePatternData { translate: pos2(0.0, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(1.0, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(0.5, 0.866), rotate: 0.0, scale: 2.0 },
        ];
        let initial = vec![ShapePatternData::default()];
        let config = FractalConfig {
            get_points: &|_, _, _| vec![pos2(0.0, 0.0)],
            get_lines: &|_, _, _| vec![],
            pattern: &pattern,
            initial: &initial,
            iterations: 1,
            regroup: false,
            display_parent: false,
            delta_radius: 0.0,
            density_sources: &[],
        };
        let result = generate_fractal(&config);
        let expected = (3.0f32).log10() / (2.0f32).log10();
        assert!((result.dimension - expected).abs() < 0.001);
    }

    #[test]
    fn points_generated_count() {
        // pattern with non-zero translate so each child produces a distinct point
        let pattern = vec![ShapePatternData { translate: pos2(10.0, 0.0), rotate: 0.0, scale: 2.0 }];
        let initial = vec![ShapePatternData::default()];
        let config = FractalConfig {
            get_points: &|translate, _rotate, _scale| vec![translate],
            get_lines: &|_, _, _| vec![],
            pattern: &pattern,
            initial: &initial,
            iterations: 2,
            regroup: false,
            display_parent: true,
            delta_radius: 0.0,
            density_sources: &[],
        };
        let result = generate_fractal(&config);
        // display_parent=true: initial + 2 iterations = 3 shapes, each with distinct translate
        assert_eq!(result.points.len(), 3);
    }

    #[test]
    fn lines_are_connected() {
        let pattern = vec![ShapePatternData { translate: pos2(0.0, 0.0), rotate: 0.0, scale: 2.0 }];
        let initial = vec![ShapePatternData::default()];
        let config = FractalConfig {
            get_points: &|_, _, _| vec![pos2(0.0, 0.0), pos2(1.0, 0.0)],
            get_lines: &|_, _, _| vec![[pos2(0.0, 0.0), pos2(1.0, 0.0)]],
            pattern: &pattern,
            initial: &initial,
            iterations: 2,
            regroup: false,
            display_parent: false,
            delta_radius: 0.0,
            density_sources: &[],
        };
        let result = generate_fractal(&config);
        for line in &result.lines {
            assert!(line[0] < result.points.len());
            assert!(line[1] < result.points.len());
        }
    }

    #[test]
    fn regroup_merges_shared_vertices() {
        let base = vec![pos2(-5.0, 0.0), pos2(5.0, 0.0), pos2(0.0, 8.660254)];
        let pattern = vec![
            ShapePatternData { translate: pos2(-2.5, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(2.5, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(-6.7055225e-8, 4.330127), rotate: 0.0, scale: 2.0 },
        ];
        let initial = vec![ShapePatternData::default()];
        let base_ref = &base;
        let get_points = |t: Pos2, r: f32, s: f32| -> Vec<Pos2> {
            base_ref.iter().map(|&p| apply_transform(p, t, r, vec2(s, s))).collect()
        };
        let get_lines = |t: Pos2, r: f32, s: f32| -> Vec<[Pos2; 2]> {
            let pts: Vec<Pos2> = base_ref.iter().map(|&p| apply_transform(p, t, r, vec2(s, s))).collect();
            vec![[pts[0], pts[1]], [pts[1], pts[2]], [pts[2], pts[0]]]
        };
        let unmerged = generate_fractal(&FractalConfig {
            get_points: &get_points,
            get_lines: &get_lines,
            pattern: &pattern,
            initial: &initial,
            iterations: 5,
            regroup: false,
            display_parent: false,
            delta_radius: 0.0,
            density_sources: &[],
        });
        assert_eq!(unmerged.points.len(), 410);

        let merged = generate_fractal(&FractalConfig {
            get_points: &get_points,
            get_lines: &get_lines,
            pattern: &pattern,
            initial: &initial,
            iterations: 5,
            regroup: true,
            display_parent: false,
            delta_radius: 0.0,
            density_sources: &[],
        });
        assert_eq!(merged.points.len(), 366);

        let n = merged.points.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], i: usize) -> usize {
            let mut r = i;
            while p[r] != r {
                r = p[r];
            }
            r
        }
        for l in &merged.lines {
            let ra = find(&mut parent, l[0]);
            let rb = find(&mut parent, l[1]);
            if ra != rb {
                parent[ra] = rb;
            }
        }
        for i in 1..n {
            assert_eq!(find(&mut parent, i), find(&mut parent, 0), "graphe déconnecté");
        }
    }

    #[test]
    fn regroup_max_depth_count() {
        let base = vec![pos2(-5.0, 0.0), pos2(5.0, 0.0), pos2(0.0, 8.660254)];
        let pattern = vec![
            ShapePatternData { translate: pos2(-2.5, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(2.5, 0.0), rotate: 0.0, scale: 2.0 },
            ShapePatternData { translate: pos2(-6.7055225e-8, 4.330127), rotate: 0.0, scale: 2.0 },
        ];
        let initial = vec![ShapePatternData::default()];
        let base_ref = &base;
        let get_points = |t: Pos2, r: f32, s: f32| -> Vec<Pos2> {
            base_ref.iter().map(|&p| apply_transform(p, t, r, vec2(s, s))).collect()
        };
        let get_lines = |t: Pos2, r: f32, s: f32| -> Vec<[Pos2; 2]> {
            let pts: Vec<Pos2> = base_ref.iter().map(|&p| apply_transform(p, t, r, vec2(s, s))).collect();
            vec![[pts[0], pts[1]], [pts[1], pts[2]], [pts[2], pts[0]]]
        };
        let result = generate_fractal(&FractalConfig {
            get_points: &get_points,
            get_lines: &get_lines,
            pattern: &pattern,
            initial: &initial,
            iterations: 10,
            regroup: true,
            display_parent: false,
            delta_radius: 0.0,
            density_sources: &[],
        });
        assert_eq!(result.points.len(), 88_575);
    }
}
