use num_complex::Complex64 as C;

use super::{finite, NetworkError};

pub(super) fn solve(mut a: Vec<Vec<C>>, mut b: Vec<C>) -> Result<(Vec<C>, f64, f64), NetworkError> {
    let original_a = a.clone();
    let original_b = b.clone();
    let n = b.len();
    // Equilibrate the rows and columns after dimensional normalization. Keep
    // the original system for the residual, so scaling cannot hide bad KCL.
    for (row, rhs) in a.iter_mut().zip(&mut b) {
        let max = row.iter().map(|v| v.norm()).fold(0.0, f64::max);
        if max == 0.0 {
            return Err(NetworkError::Singular);
        }
        if !max.is_finite() || !finite(*rhs) {
            return Err(NetworkError::NonFinite);
        }
        row.iter_mut().for_each(|v| *v /= max);
        *rhs /= max;
    }
    let mut scale = vec![0.0_f64; n];
    for row in &a {
        for (s, v) in scale.iter_mut().zip(row) {
            *s = s.max(v.norm());
        }
    }
    if scale.contains(&0.0) {
        return Err(NetworkError::Singular);
    }
    for row in &mut a {
        row.iter_mut().zip(&scale).for_each(|(v, s)| *v /= *s);
    }
    let (mut largest, mut smallest) = (0.0_f64, f64::INFINITY);
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&i, &j| a[i][col].norm().total_cmp(&a[j][col].norm()))
            .unwrap();
        a.swap(pivot, col);
        b.swap(pivot, col);
        let diagonal = a[col][col];
        let norm = diagonal.norm();
        if !norm.is_finite() {
            return Err(NetworkError::NonFinite);
        }
        if norm == 0.0 {
            return Err(NetworkError::Singular);
        }
        largest = largest.max(norm);
        smallest = smallest.min(norm);
        let (top, bottom) = a.split_at_mut(col + 1);
        let pivot_row = &top[col];
        let rhs = b[col];
        for (row, target_rhs) in bottom.iter_mut().zip(&mut b[col + 1..]) {
            let factor = row[col] / diagonal;
            for (target, &v) in row[col + 1..].iter_mut().zip(&pivot_row[col + 1..]) {
                *target -= factor * v;
            }
            *target_rhs -= factor * rhs;
        }
    }
    let ratio = smallest / largest;
    if ratio < 1e-12 {
        return Err(NetworkError::IllConditioned(ratio));
    }
    let mut x = vec![C::default(); n];
    for i in (0..n).rev() {
        let dot: C = a[i][i + 1..]
            .iter()
            .zip(&x[i + 1..])
            .map(|(&a, &b)| a * b)
            .sum();
        x[i] = (b[i] - dot) / a[i][i];
    }
    x.iter_mut().zip(scale).for_each(|(v, s)| *v /= s);
    if x.iter().any(|&v| !finite(v)) {
        return Err(NetworkError::NonFinite);
    }
    let x_norm = x.iter().map(|v| v.norm()).fold(0.0, f64::max);
    let mut residual = 0.0_f64;
    for (row, rhs) in original_a.iter().zip(original_b) {
        let mut row_norm = 0.0;
        let mut sum = C::default();
        for (&a, &x) in row.iter().zip(&x) {
            let v = a * x;
            row_norm += a.norm();
            sum += v;
        }
        let denominator = row_norm * x_norm + rhs.norm();
        if !denominator.is_finite() || !finite(sum) {
            return Err(NetworkError::NonFinite);
        }
        residual = residual.max((sum - rhs).norm() / denominator.max(f64::MIN_POSITIVE));
    }
    if residual > 1e-8 {
        return Err(NetworkError::Residual(residual));
    }
    Ok((x, residual, ratio))
}
