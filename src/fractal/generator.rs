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
            find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, p, s.scale, *regroup);
        }
        for seg in &sl {
            let a_idx = find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, seg[0], s.scale, *regroup);
            let b_idx = find_or_add_point(&mut final_points, &mut final_point_scale, &mut point_map, seg[1], s.scale, *regroup);
            final_lines.push([a_idx, b_idx]);
        }
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
    regroup: bool,
) -> usize {
    let key = point_key(p);
    if let Some(&idx) = map.get(&key) {
        scales[idx] = scales[idx].max(scale);
        return idx;
    }
    if regroup {
        let tolerance = 0.001;
        let tolerance_sq = tolerance * tolerance;
        for (i, &pt) in points.iter().enumerate() {
            let dx = pt.x - p.x;
            let dy = pt.y - p.y;
            if dx * dx + dy * dy < tolerance_sq {
                scales[i] = scales[i].max(scale);
                map.insert(key, i);
                return i;
            }
        }
    }
    let idx = points.len();
    points.push(p);
    scales.push(scale);
    map.insert(key, idx);
    idx
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
